use std::{convert::TryFrom, num::ParseIntError};

use slotmap::{DefaultKey, SlotMap};

use crate::models::{Color, Color16};

#[derive(Default, Debug, Clone, Copy)]
struct RgbChannelAdjustment {
    adjust: Color,
}

impl RgbChannelAdjustment {
    pub fn apply(&self, input: u8, brightness: u8) -> Color {
        Color::new(
            ((brightness as u32 * input as u32 * self.adjust.red as u32) / 65025) as _,
            ((brightness as u32 * input as u32 * self.adjust.green as u32) / 65025) as _,
            ((brightness as u32 * input as u32 * self.adjust.blue as u32) / 65025) as _,
        )
    }
}

impl From<Color> for RgbChannelAdjustment {
    fn from(color: Color) -> Self {
        Self { adjust: color }
    }
}

#[derive(Debug, Clone, Copy)]
struct RgbTransform {
    backlight_enabled: bool,
    backlight_colored: bool,
    sum_brightness_low: f32,
    gamma_r: f32,
    gamma_g: f32,
    gamma_b: f32,
    brightness: u8,
    brightness_compensation: u8,
}

impl From<&crate::models::ChannelAdjustment> for RgbTransform {
    fn from(settings: &crate::models::ChannelAdjustment) -> Self {
        Self {
            backlight_enabled: false,
            backlight_colored: settings.backlight_colored,
            sum_brightness_low: 765.0
                * (((2.0f32.powf(settings.backlight_threshold as f32 / 100.0) * 2.0) - 1.0) / 3.0),
            gamma_r: settings.gamma_red,
            gamma_g: settings.gamma_green,
            gamma_b: settings.gamma_blue,
            brightness: settings.brightness as _,
            brightness_compensation: settings.brightness_compensation as _,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct BrightnessComponents {
    pub rgb: u8,
    pub cmy: u8,
    pub w: u8,
}

impl RgbTransform {
    fn gamma(x: u8, gamma: f32) -> u8 {
        ((x as f32 / 255.0).powf(gamma) * 255.0) as u8
    }

    pub fn brightness_components(&self) -> BrightnessComponents {
        let fw = self.brightness_compensation as f32 + 2.0 / 100.0 + 1.0;
        let fcmy = self.brightness_compensation as f32 / 100.0 + 1.0;

        if self.brightness > 0 {
            let b_in = if self.brightness < 50 {
                -0.09 * self.brightness as f32 + 7.5
            } else {
                -0.04 * self.brightness as f32 + 5.0
            };

            BrightnessComponents {
                rgb: (255.0 / b_in).min(255.0) as u8,
                cmy: (255.0 / (b_in + fcmy)).min(255.0) as u8,
                w: (255.0 / (b_in + fw)).min(255.0) as u8,
            }
        } else {
            BrightnessComponents::default()
        }
    }

    pub fn apply(&self, input: Color) -> Color {
        let (r, g, b) = input.into_components();

        // Apply gamma
        let (r, g, b) = (
            Self::gamma(r, self.gamma_r),
            Self::gamma(g, self.gamma_g),
            Self::gamma(b, self.gamma_b),
        );

        // Apply brightness
        let mut rgb_sum = r as f32 + g as f32 + b as f32;

        if self.backlight_enabled
            && self.sum_brightness_low > 0.
            && rgb_sum < self.sum_brightness_low
        {
            if self.backlight_colored {
                let (mut r, mut g, mut b) = (r, g, b);

                if rgb_sum == 0. {
                    r = r.max(1);
                    g = g.max(1);
                    b = b.max(1);
                    rgb_sum = r as f32 + g as f32 + b as f32;
                }

                let cl = (self.sum_brightness_low / rgb_sum as f32).min(255.0);

                r = (r as f32 * cl) as u8;
                g = (g as f32 * cl) as u8;
                b = (b as f32 * cl) as u8;

                Color::new(r, g, b)
            } else {
                let x = (self.sum_brightness_low / 3.0).min(255.0) as u8;
                Color::new(x, x, x)
            }
        } else {
            Color::new(r, g, b)
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ColorAdjustmentData {
    black: RgbChannelAdjustment,
    white: RgbChannelAdjustment,
    red: RgbChannelAdjustment,
    green: RgbChannelAdjustment,
    blue: RgbChannelAdjustment,
    cyan: RgbChannelAdjustment,
    magenta: RgbChannelAdjustment,
    yellow: RgbChannelAdjustment,
    transform: RgbTransform,
}

impl ColorAdjustmentData {
    pub fn apply(&self, color: Color) -> Color {
        let (ored, ogreen, oblue) = self.transform.apply(color).into_components();
        let brightness_components = self.transform.brightness_components();

        // Upgrade to u32
        let (ored, ogreen, oblue) = (ored as u32, ogreen as u32, oblue as u32);

        let nrng = (255 - ored) * (255 - ogreen);
        let rng = ored * (255 - ogreen);
        let nrg = (255 - ored) * ogreen;
        let rg = ored * ogreen;

        let black = nrng * (255 - oblue) / 65025;
        let red = rng * (255 - oblue) / 65025;
        let green = nrg * (255 - oblue) / 65025;
        let blue = nrng * (oblue) / 65025;
        let cyan = nrg * (oblue) / 65025;
        let magenta = rng * (oblue) / 65025;
        let yellow = rg * (255 - oblue) / 65025;
        let white = rg * (oblue) / 65025;

        let o = self.black.apply(black as _, 255);
        let r = self.red.apply(red as _, brightness_components.rgb);
        let g = self.green.apply(green as _, brightness_components.rgb);
        let b = self.blue.apply(blue as _, brightness_components.rgb);
        let c = self.cyan.apply(cyan as _, brightness_components.cmy);
        let m = self.magenta.apply(magenta as _, brightness_components.cmy);
        let y = self.yellow.apply(yellow as _, brightness_components.cmy);
        let w = self.white.apply(white as _, brightness_components.w);

        Color::new(
            o.red + r.red + g.red + b.red + c.red + m.red + y.red + w.red,
            o.green + r.green + g.green + b.green + c.green + m.green + y.green + w.green,
            o.blue + r.blue + g.blue + b.blue + c.blue + m.blue + y.blue + w.blue,
        )
    }
}

impl From<&crate::models::ChannelAdjustment> for ColorAdjustmentData {
    fn from(settings: &crate::models::ChannelAdjustment) -> Self {
        Self {
            black: Default::default(),
            white: settings.white.into(),
            red: settings.red.into(),
            green: settings.green.into(),
            blue: settings.blue.into(),
            cyan: settings.cyan.into(),
            magenta: settings.magenta.into(),
            yellow: settings.yellow.into(),
            transform: settings.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LedMatch {
    /// *
    All,
    /// Range
    Ranges(LedRanges),
    /// Invalid filter
    None,
}

lazy_static::lazy_static! {
    static ref PATTERN_REGEX: regex::Regex = regex::Regex::new("([0-9]+(\\-[0-9]+)?)(,[ ]*([0-9]+(\\-[0-9]+)?))*").unwrap();
}

#[derive(Debug, Clone)]
pub struct LedRanges {
    ranges: Vec<std::ops::RangeInclusive<usize>>,
}

impl TryFrom<&str> for LedRanges {
    type Error = &'static str;

    fn try_from(pattern: &str) -> Result<Self, Self::Error> {
        Ok(Self {
            ranges: pattern
                .split(',')
                .map(|led_index_list| {
                    if led_index_list.contains('-') {
                        let split: Vec<_> = led_index_list.splitn(2, '-').collect();
                        let start = split[0].parse()?;
                        let end = split[1].parse()?;

                        Ok(start..=end)
                    } else {
                        let index = led_index_list.trim().parse()?;
                        Ok(index..=index)
                    }
                })
                .collect::<Result<Vec<_>, ParseIntError>>()
                .map_err(|_| "invalid index")?,
        })
    }
}

impl From<&str> for LedMatch {
    fn from(pattern: &str) -> Self {
        if pattern == "*" {
            return Self::All;
        }

        if PATTERN_REGEX.is_match(pattern) {
            if let Ok(ranges) = LedRanges::try_from(pattern) {
                return Self::Ranges(ranges);
            }
        }

        error!("invalid format for LED pattern `{}`, ignoring", pattern);
        Self::None
    }
}

#[derive(Debug, Clone)]
pub struct ColorAdjustment {
    leds: LedMatch,
    data: ColorAdjustmentData,
}

impl From<&crate::models::ChannelAdjustment> for ColorAdjustment {
    fn from(settings: &crate::models::ChannelAdjustment) -> Self {
        let data = settings.into();

        Self {
            leds: settings.leds.as_str().into(),
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelAdjustmentsBuilder {
    adjustments: Vec<ColorAdjustment>,
    led_count: u32,
}

impl ChannelAdjustmentsBuilder {
    pub fn new() -> Self {
        Self {
            adjustments: Vec::with_capacity(1),
            led_count: 0,
        }
    }

    pub fn adjustments<T: Into<ColorAdjustment>>(
        &mut self,
        adjustments: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.adjustments
            .extend(adjustments.into_iter().map(Into::into));
        self
    }

    pub fn led_count(&mut self, led_count: u32) -> &mut Self {
        self.led_count = led_count;
        self
    }

    pub fn build(&self) -> ChannelAdjustments {
        let mut adjustments = SlotMap::with_capacity(self.adjustments.len());
        let mut led_mappings = Vec::with_capacity(self.led_count as _);

        for adjustment in &self.adjustments {
            match &adjustment.leds {
                LedMatch::All => {
                    let key = adjustments.insert(adjustment.data);
                    led_mappings.fill(key);
                }
                LedMatch::Ranges(ranges) => {
                    let key = adjustments.insert(adjustment.data);
                    for range in &ranges.ranges {
                        if let Some(range) = led_mappings.get_mut(range.clone()) {
                            range.fill(key);
                        } else {
                            error!("invalid range {:?} for {} leds", range, self.led_count);
                        }
                    }
                }
                LedMatch::None => {}
            }
        }

        ChannelAdjustments {
            adjustments,
            led_mappings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelAdjustments {
    adjustments: SlotMap<DefaultKey, ColorAdjustmentData>,
    led_mappings: Vec<DefaultKey>,
}

impl ChannelAdjustments {
    pub fn apply(&self, led_data: &mut [Color16]) {
        for (i, led) in led_data.iter_mut().enumerate() {
            if let Some(adjustment) = self
                .led_mappings
                .get(i)
                .and_then(|key| self.adjustments.get(*key))
            {
                // TODO: Actual 16-bit color?
                *led = color_to16(adjustment.apply(color_to8(*led)));
            }
        }
    }
}

pub fn color_to8(color: Color16) -> Color {
    let (r, g, b) = color.into_components();
    Color::from_components(((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8))
}

pub fn color_to16(color: Color) -> Color16 {
    let (r, g, b) = color.into_components();
    Color16::from_components(((r as u16) << 8, (g as u16) << 8, (b as u16) << 8))
}
