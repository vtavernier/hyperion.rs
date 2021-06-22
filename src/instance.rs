use thiserror::Error;
use tokio::select;
use tokio::sync::broadcast;

use crate::{
    color::{ChannelAdjustments, ChannelAdjustmentsBuilder},
    global::{Global, Message, MuxedMessage, MuxedMessageData},
    image::RawImage,
    models::{self, DeviceConfig, InstanceConfig},
};

mod black_border_detector;
use black_border_detector::*;

mod device;
use device::*;

mod smoothing;
use smoothing::*;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("device error: {0}")]
    Device(#[from] DeviceError),
    #[error("recv error: {0}")]
    Recv(#[from] broadcast::error::RecvError),
    #[error("command not supported: {0:?}")]
    NotSupported(MuxedMessageData),
}

pub struct Instance {
    config: InstanceConfig,
    device: Device,
    receiver: broadcast::Receiver<MuxedMessage>,
    color_data: Vec<models::Color16>,
    black_border_detector: BlackBorderDetector,
    channel_adjustments: ChannelAdjustments,
    smoothing: Smoothing,
}

impl Instance {
    pub async fn new(global: Global, config: InstanceConfig) -> Result<Self, InstanceError> {
        let device = Device::new(&config.instance.friendly_name, config.device.clone()).await?;
        let led_count = config.device.hardware_led_count();
        let black_border_detector = BlackBorderDetector::new(config.black_border_detector.clone());
        let channel_adjustments = ChannelAdjustmentsBuilder::new()
            .adjustments(config.color.channel_adjustment.iter())
            .led_count(led_count as _)
            .build();
        let smoothing = Smoothing::new(config.smoothing.clone(), led_count);

        Ok(Self {
            config,
            device,
            receiver: global.subscribe_muxed().await,
            color_data: vec![models::Color16::default(); led_count],
            black_border_detector,
            channel_adjustments,
            smoothing,
        })
    }

    fn handle_color(&mut self, color: models::Color) {
        let color = crate::utils::color_to16(color);
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

            *value = models::Color16::from_components((
                (r_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (g_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (b_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
            ));
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
        }

        // In-place transform colors
        self.channel_adjustments.apply(&mut self.color_data);

        // Update the smoothing state with the new color data
        self.smoothing.set_target(&self.color_data);
    }

    pub async fn run(mut self) -> Result<(), InstanceError> {
        loop {
            select! {
                _ = self.device.update() => {
                    // Device update completed
                },
                led_data = self.smoothing.update() => {
                    // The smoothing state has updated
                    self.device.set_led_data(led_data).await?;
                },
                message = self.receiver.recv() => {
                    self.handle_message(message?);
                }
            }
        }
    }
}
