use std::sync::Arc;

use thiserror::Error;
use tokio::select;
use tokio::sync::broadcast;

use crate::{
    api::types::PriorityInfo,
    color::{color_to16, ChannelAdjustments, ChannelAdjustmentsBuilder},
    global::Global,
    image::RawImage,
    models::{Color, Color16, InstanceConfig},
    servers::{self, ServerHandle},
};

mod black_border_detector;
use black_border_detector::*;

mod device;
use device::*;

mod muxer;
use muxer::*;

mod smoothing;
use smoothing::*;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("device error: {0}")]
    Device(#[from] DeviceError),
    #[error("recv error: {0}")]
    Recv(#[from] broadcast::error::RecvError),
}

pub struct Instance {
    config: Arc<InstanceConfig>,
    device: Device,
    muxer: PriorityMuxer,
    color_data: Vec<Color16>,
    black_border_detector: BlackBorderDetector,
    channel_adjustments: ChannelAdjustments,
    smoothing: Smoothing,
    _boblight_server: ServerHandle,
}

impl Instance {
    pub async fn new(global: Global, config: InstanceConfig) -> Result<Self, InstanceError> {
        let device = Device::new(&config.instance.friendly_name, config.device.clone()).await?;
        let led_count = config.leds.leds.len();
        let black_border_detector = BlackBorderDetector::new(config.black_border_detector.clone());
        let channel_adjustments = ChannelAdjustmentsBuilder::new()
            .adjustments(config.color.channel_adjustment.iter())
            .led_count(led_count as _)
            .build();
        let smoothing = Smoothing::new(config.smoothing.clone(), led_count);

        let (muxer, tx) = PriorityMuxer::new(global.clone()).await;

        let config = Arc::new(config);

        let _boblight_server = servers::bind(
            "Boblight",
            config.boblight_server.clone(),
            global.clone(),
            {
                let instance = config.clone();
                move |tcp, global| {
                    servers::boblight::handle_client(tcp, tx.clone(), instance.clone(), global)
                }
            },
        )
        .await?;

        Ok(Self {
            config,
            device,
            muxer,
            color_data: vec![Color16::default(); led_count],
            black_border_detector,
            channel_adjustments,
            smoothing,
            _boblight_server,
        })
    }

    fn handle_color(&mut self, color: Color) {
        let color = crate::color::color_to16(color);
        self.color_data.iter_mut().map(|x| *x = color).count();
    }

    fn handle_image(&mut self, image: &RawImage) {
        // Update the black border
        self.black_border_detector.process(image);
        let black_border = self.black_border_detector.current_border();

        // Update the 16-bit color data from the LED ranges and the image
        let ((xmin, xmax), (ymin, ymax)) = black_border.get_ranges(image.width(), image.height());
        let width = (xmax - xmin) as f32;
        let height = (ymax - ymin) as f32;
        for (spec, value) in self.config.leds.leds.iter().zip(self.color_data.iter_mut()) {
            let mut r_acc = 0u64;
            let mut g_acc = 0u64;
            let mut b_acc = 0u64;
            let mut cnt = 0u64;

            // TODO: Fixed point arithmetic
            let lxmin = spec.hmin * width + xmin as f32;
            let lxmax = spec.hmax * width + xmin as f32;
            let lymin = spec.vmin * height + ymin as f32;
            let lymax = spec.vmax * height + ymin as f32;

            for y in lymin.floor() as u32..=(lymax.ceil() as u32).min(image.height() - 1) {
                let y_area = if (y as f32) < lymin {
                    (255. * (1. - lymin.fract())) as u64
                } else if (y + 1) as f32 > lymax {
                    (255. * lymax.fract()) as u64
                } else {
                    255
                };

                for x in lxmin.floor() as u32..=(lxmax.ceil() as u32).min(image.width() - 1) {
                    if let Some(rgb) = image.color_at(x, y) {
                        let x_area = if (x as f32) < lxmin {
                            (255. * (1. - lxmin.fract())) as u64
                        } else if (x + 1) as f32 > lxmax {
                            (255. * lxmax.fract()) as u64
                        } else {
                            255
                        };

                        let area = x_area * y_area / 255;

                        let (r, g, b) = rgb.into_components();
                        r_acc += (r as u64 * 255) * area;
                        g_acc += (g as u64 * 255) * area;
                        b_acc += (b as u64 * 255) * area;
                        cnt += area;
                    }
                }
            }

            *value = Color16::from_components((
                (r_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (g_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (b_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
            ));
        }
    }

    fn handle_led_colors(&mut self, led_colors: &[Color]) {
        if led_colors.len() != self.color_data.len() {
            error!(
                "invalid led color data, expected {} leds, got {}",
                self.color_data.len(),
                led_colors.len()
            );
        } else {
            for (led_mut, color) in self.color_data.iter_mut().zip(led_colors.iter()) {
                *led_mut = color_to16(*color);
            }
        }
    }

    fn handle_message(&mut self, message: MuxedMessage) {
        // Update color data
        match message.data() {
            MuxedMessageData::SolidColor { color, .. } => {
                // TODO: Replace with fill once it's stabilized
                self.handle_color(*color);
            }
            MuxedMessageData::Image { image, .. } => {
                self.handle_image(&*image);
            }
            MuxedMessageData::LedColors { led_colors, .. } => {
                self.handle_led_colors(&*led_colors);
            }
        }

        // In-place transform colors
        self.channel_adjustments.apply(&mut self.color_data);

        // Update the smoothing state with the new color data
        self.smoothing.set_target(&self.color_data);
    }

    pub async fn current_priorities(&self) -> Vec<PriorityInfo> {
        self.muxer.current_priorities().await
    }

    pub async fn run(mut self) -> Result<(), InstanceError> {
        loop {
            select! {
                _ = self.device.update() => {
                    // Device update completed
                },
                message = self.muxer.run() => {
                    // Muxer update completed
                    match message {
                        Ok(Some(message)) => {
                            self.handle_message(message);
                        },
                        Ok(None) => {
                            // Nothing to do
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            // No more input messages
                            return Ok(());
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("skipped {} input messages", skipped);
                        },
                    }
                },
                led_data = self.smoothing.update() => {
                    // The smoothing state has updated
                    self.device.set_led_data(led_data).await?;
                },
            }
        }
    }
}
