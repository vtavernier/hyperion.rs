use crate::{
    color::{color_to16, ChannelAdjustments, ChannelAdjustmentsBuilder},
    image::{prelude::*, Reducer},
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
    reducer: Reducer,
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
            reducer: Default::default(),
        }
    }

    fn handle_color(&mut self, color: Color) {
        self.color_data.fill(color_to16(color));
    }

    fn handle_image(&mut self, image: &impl Image) {
        // Update the black border
        self.black_border_detector.process(image);
        let black_border = self.black_border_detector.current_border();

        // Crop the image using a view
        let image = {
            let (x, y) = black_border.get_ranges(image.width(), image.height());
            image.wrap(x, y)
        };

        // Update the 16-bit color data from the LED ranges and the image
        self.reducer
            .reduce(&image, &self.leds.leds[..], &mut self.color_data);
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
                warn!(extra = %(data_count - led_count), "too much LED data");
            }

            let (dst, fill) = self.color_data.split_at_mut(led_count);
            (&led_colors[..led_count], dst, fill)
        } else {
            if !self.notified_inconsistent_led_data {
                self.notified_inconsistent_led_data = true;
                warn!(missing = %(led_count - data_count), "not enough LED data");
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
