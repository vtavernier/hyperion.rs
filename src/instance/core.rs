use crate::{
    color::{color_to16, ChannelAdjustments, ChannelAdjustmentsBuilder},
    image::prelude::*,
    models::{Color, Color16, InstanceConfig, Leds},
};

use super::{BlackBorderDetector, MuxedMessage, MuxedMessageData, Smoothing};

/// Core part of an instance
///
/// This handles incoming message and computes LED colors.
pub struct Core {
    leds: Leds,
    color_data: Vec<Color16>,
    black_border_detector: BlackBorderDetector,
    channel_adjustments: ChannelAdjustments,
    smoothing: Smoothing,
    notified_inconsistent_led_data: bool,
}

impl Core {
    pub async fn new(config: &InstanceConfig) -> Self {
        let led_count = config.leds.leds.len();
        let black_border_detector = BlackBorderDetector::new(config.black_border_detector.clone());
        let channel_adjustments = ChannelAdjustmentsBuilder::new()
            .adjustments(config.color.channel_adjustment.iter())
            .led_count(led_count as _)
            .build();
        let smoothing = Smoothing::new(config.smoothing.clone(), led_count);

        Self {
            leds: config.leds.clone(),
            color_data: vec![Color16::default(); led_count],
            black_border_detector,
            channel_adjustments,
            smoothing,
            notified_inconsistent_led_data: false,
        }
    }

    fn handle_color(&mut self, color: Color) {
        self.color_data.fill(color_to16(color));
    }

    fn handle_image(&mut self, image: &impl Image) {
        // Update the black border
        self.black_border_detector.process(image);
        let black_border = self.black_border_detector.current_border();

        // Update the 16-bit color data from the LED ranges and the image
        let image = {
            let (x, y) = black_border.get_ranges(image.width(), image.height());
            image.wrap(x, y)
        };

        let width = image.width() as f32;
        let height = image.height() as f32;
        for (spec, value) in self.leds.leds.iter().zip(self.color_data.iter_mut()) {
            let mut r_acc = 0u64;
            let mut g_acc = 0u64;
            let mut b_acc = 0u64;
            let mut cnt = 0u64;

            // TODO: Fixed point arithmetic
            let lxmin = spec.hmin * width;
            let lxmax = spec.hmax * width;
            let lymin = spec.vmin * height;
            let lymax = spec.vmax * height;

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

            *value = Color16::new(
                (r_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (g_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (b_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
            );
        }
    }

    fn handle_led_colors(&mut self, led_colors: &[Color]) {
        let led_count = self.color_data.len();
        let data_count = led_colors.len();

        let (src, dst, fill) = if led_count == data_count {
            self.notified_inconsistent_led_data = false;

            let (dst, fill) = self.color_data.split_at_mut(led_count);
            (led_colors, dst, fill)
        } else if led_count < data_count {
            if !self.notified_inconsistent_led_data {
                self.notified_inconsistent_led_data = true;
                warn!("too much LED data: {} extra", data_count - led_count);
            }

            let (dst, fill) = self.color_data.split_at_mut(led_count);
            (&led_colors[..led_count], dst, fill)
        } else {
            if !self.notified_inconsistent_led_data {
                self.notified_inconsistent_led_data = true;
                warn!("not enough LED data: {} missing", led_count - data_count);
            }

            let (dst, fill) = self.color_data.split_at_mut(data_count);
            (led_colors, dst, fill)
        };

        for (led_mut, color) in dst.iter_mut().zip(src.iter()) {
            *led_mut = color_to16(*color);
        }

        fill.fill(Color16::default());
    }

    pub fn handle_message(&mut self, message: MuxedMessage) {
        // Update color data
        match message.data() {
            MuxedMessageData::SolidColor { color, .. } => {
                // TODO: Replace with fill once it's stabilized
                self.handle_color(*color);
            }
            MuxedMessageData::Image { image, .. } => {
                self.handle_image(image.as_ref());
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

    pub async fn update(&mut self) -> &[Color] {
        self.smoothing.update().await
    }
}
