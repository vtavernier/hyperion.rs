use std::convert::TryFrom;

use serde_derive::{Deserialize, Serialize};
use thiserror::Error;
use validator::Validate;

use crate::db::models as db_models;

use super::{default_true, Color, Device, ServerConfig};

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("error parsing date: {0}")]
    Chrono(#[from] chrono::ParseError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Instance {
    #[serde(skip)]
    pub id: i32,
    #[serde(default = "String::new")]
    pub friendly_name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "chrono::Utc::now")]
    pub last_use: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<db_models::DbInstance> for Instance {
    type Error = InstanceError;

    fn try_from(db: db_models::DbInstance) -> Result<Self, Self::Error> {
        Ok(Self {
            id: db.instance,
            friendly_name: db.friendly_name,
            enabled: db.enabled != 0,
            last_use: chrono::DateTime::parse_from_rfc3339(&db.last_use)?
                .with_timezone(&chrono::Utc),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum EffectType {
    Color,
    Effect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct BackgroundEffect {
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub color: Color,
    pub effect: String,
    pub enable: bool,
    #[serde(rename = "type")]
    pub ty: EffectType,
}

impl Default for BackgroundEffect {
    fn default() -> Self {
        Self {
            enable: true,
            ty: EffectType::Effect,
            color: Color::from_components((255, 138, 0)),
            effect: "Warm mood blobs".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum BlackBorderDetectorMode {
    Default,
    Classic,
    Osd,
    Letterbox,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackBorderDetector {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[validate(range(min = 0, max = 100))]
    pub threshold: u32,
    pub unknown_frame_cnt: u32,
    pub border_frame_cnt: u32,
    pub max_inconsistent_cnt: u32,
    pub blur_remove_cnt: u16,
    pub mode: BlackBorderDetectorMode,
}

impl Default for BlackBorderDetector {
    fn default() -> Self {
        Self {
            enable: true,
            threshold: 5,
            unknown_frame_cnt: 600,
            border_frame_cnt: 50,
            max_inconsistent_cnt: 10,
            blur_remove_cnt: 1,
            mode: BlackBorderDetectorMode::Default,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BoblightServer {
    pub enable: bool,
    #[validate(range(min = 1024))]
    pub port: u16,
    #[validate(range(min = 100, max = 254))]
    pub priority: i32,
}

impl Default for BoblightServer {
    fn default() -> Self {
        Self {
            enable: false,
            port: 19333,
            priority: 128,
        }
    }
}

impl ServerConfig for BoblightServer {
    fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ImageToLedMappingType {
    MulticolorMean,
    UnicolorMean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ColorAdjustment {
    pub image_to_led_mapping_type: ImageToLedMappingType,
    #[validate]
    pub channel_adjustment: Vec<ChannelAdjustment>,
}

impl Default for ColorAdjustment {
    fn default() -> Self {
        Self {
            image_to_led_mapping_type: ImageToLedMappingType::MulticolorMean,
            channel_adjustment: vec![ChannelAdjustment::default()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelAdjustment {
    pub id: String,
    pub leds: String,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub white: Color,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub red: Color,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub green: Color,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub blue: Color,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub cyan: Color,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub magenta: Color,
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub yellow: Color,
    #[validate(range(min = 0, max = 100))]
    pub backlight_threshold: u32,
    pub backlight_colored: bool,
    #[validate(range(min = 0, max = 100))]
    pub brightness: u32,
    #[validate(range(min = 0, max = 100))]
    pub brightness_compensation: u32,
    #[validate(range(min = 0.1, max = 5.0))]
    pub gamma_red: f32,
    #[validate(range(min = 0.1, max = 5.0))]
    pub gamma_green: f32,
    #[validate(range(min = 0.1, max = 5.0))]
    pub gamma_blue: f32,
}

impl Default for ChannelAdjustment {
    fn default() -> Self {
        Self {
            id: "A userdefined name".to_owned(),
            leds: "*".to_owned(),
            white: Color::from_components((255, 255, 255)),
            red: Color::from_components((255, 0, 0)),
            green: Color::from_components((0, 255, 0)),
            blue: Color::from_components((0, 0, 255)),
            cyan: Color::from_components((0, 255, 255)),
            magenta: Color::from_components((255, 0, 255)),
            yellow: Color::from_components((255, 255, 0)),
            backlight_threshold: 0,
            backlight_colored: false,
            brightness: 100,
            brightness_compensation: 0,
            gamma_red: 1.5,
            gamma_green: 1.5,
            gamma_blue: 1.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum ColorOrder {
    Rgb,
    Bgr,
    Rbg,
    Brg,
    Gbr,
    Grb,
}

impl ColorOrder {
    pub fn reorder_from_rgb(&self, color: Color) -> Color {
        let (r, g, b) = color.into_components();

        Color::from_components(match self {
            ColorOrder::Rgb => (r, g, b),
            ColorOrder::Bgr => (b, g, r),
            ColorOrder::Rbg => (r, b, g),
            ColorOrder::Brg => (b, r, g),
            ColorOrder::Gbr => (g, b, r),
            ColorOrder::Grb => (g, r, b),
        })
    }
}

impl Default for ColorOrder {
    fn default() -> Self {
        Self::Rgb
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Effects {
    #[validate(length(min = 1))]
    pub paths: Vec<String>,
    pub disable: Vec<String>,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            paths: vec!["$ROOT/custom-effects".to_owned()],
            disable: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct ForegroundEffect {
    #[serde(serialize_with = "crate::serde::serialize_color_as_array")]
    pub color: Color,
    pub effect: String,
    pub enable: bool,
    #[serde(rename = "type")]
    pub ty: EffectType,
    #[validate(range(min = 100))]
    pub duration_ms: Option<i32>,
}

impl Default for ForegroundEffect {
    fn default() -> Self {
        Self {
            enable: true,
            ty: EffectType::Effect,
            color: Color::from_components((255, 0, 0)),
            effect: "Rainbow swirl fast".to_owned(),
            duration_ms: Some(3000),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct InstanceCapture {
    pub system_enable: bool,
    #[validate(range(min = 100, max = 253))]
    pub system_priority: i32,
    pub v4l_enable: bool,
    #[validate(range(min = 100, max = 253))]
    pub v4l_priority: i32,
}

impl Default for InstanceCapture {
    fn default() -> Self {
        Self {
            system_enable: true,
            system_priority: 250,
            v4l_enable: false,
            v4l_priority: 240,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct ClassicLedConfig {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
    pub glength: u32,
    pub gpos: u32,
    pub position: i32,
    pub reverse: bool,
    #[validate(range(min = 1, max = 100))]
    pub hdepth: u32,
    #[validate(range(min = 1, max = 100))]
    pub vdepth: u32,
    #[validate(range(min = 0, max = 100))]
    pub overlap: u32,
    #[validate(range(max = 50))]
    pub edgegap: u32,
    #[validate(range(max = 100))]
    pub ptlh: u32,
    #[validate(range(max = 100))]
    pub ptlv: u32,
    #[validate(range(max = 100))]
    pub ptrh: u32,
    #[validate(range(max = 100))]
    pub ptrv: u32,
    #[validate(range(max = 100))]
    pub pblh: u32,
    #[validate(range(max = 100))]
    pub pblv: u32,
    #[validate(range(max = 100))]
    pub pbrh: u32,
    #[validate(range(max = 100))]
    pub pbrv: u32,
}

impl Default for ClassicLedConfig {
    fn default() -> Self {
        Self {
            top: 1,
            bottom: 0,
            left: 0,
            right: 0,
            glength: 0,
            gpos: 0,
            position: 0,
            reverse: false,
            hdepth: 8,
            vdepth: 5,
            overlap: 0,
            edgegap: 0,
            ptlh: 0,
            ptlv: 0,
            ptrh: 0,
            ptrv: 0,
            pblh: 0,
            pblv: 0,
            pbrh: 0,
            pbrv: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum MatrixCabling {
    Snake,
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum MatrixStart {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct MatrixLedConfig {
    #[validate(range(max = 50))]
    pub ledshoriz: u32,
    #[validate(range(max = 50))]
    pub ledsvert: u32,
    pub cabling: MatrixCabling,
    pub start: MatrixStart,
}

impl Default for MatrixLedConfig {
    fn default() -> Self {
        Self {
            ledshoriz: 1,
            ledsvert: 1,
            cabling: MatrixCabling::Snake,
            start: MatrixStart::TopLeft,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct LedConfig {
    #[validate]
    pub classic: ClassicLedConfig,
    #[validate]
    pub matrix: MatrixLedConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_scan_range", message = "invalid range"))]
pub struct Led {
    #[validate(range(min = 0., max = 1.))]
    pub hmin: f32,
    #[validate(range(min = 0., max = 1.))]
    pub hmax: f32,
    #[validate(range(min = 0., max = 1.))]
    pub vmin: f32,
    #[validate(range(min = 0., max = 1.))]
    pub vmax: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_order: Option<ColorOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Validate the bounds of a scan range
fn validate_scan_range(led: &Led) -> Result<(), validator::ValidationError> {
    if led.hmin > led.hmax {
        return Err(validator::ValidationError::new("invalid_range"));
    }

    if led.vmin > led.vmax {
        return Err(validator::ValidationError::new("invalid_range"));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Validate)]
pub struct Leds {
    #[validate]
    pub leds: Vec<Led>,
}

impl Default for Leds {
    fn default() -> Self {
        Self {
            leds: vec![Led {
                hmin: 0.,
                hmax: 1.,
                vmin: 0.,
                vmax: 1.,
                color_order: None,
                name: None,
            }],
        }
    }
}

impl serde::Serialize for Leds {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.leds.len()))?;
        for led in &self.leds {
            seq.serialize_element(led)?;
        }
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for Leds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Leds {
            leds: <Vec<Led> as serde::Deserialize>::deserialize(deserializer)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum SmoothingType {
    Linear,
    Decay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Smoothing {
    pub enable: bool,
    #[serde(rename = "type")]
    pub ty: SmoothingType,
    #[serde(rename = "time_ms")]
    #[validate(range(min = 25, max = 5000))]
    pub time_ms: u32,
    #[validate(range(min = 1., max = 2000.))]
    pub update_frequency: f32,
    #[validate(range(min = 1., max = 1000.))]
    pub interpolation_rate: f32,
    #[validate(range(min = 1., max = 1000.))]
    pub output_rate: f32,
    #[validate(range(min = 1., max = 20.))]
    pub decay: f32,
    pub dithering: bool,
    #[validate(range(max = 2048))]
    pub update_delay: u32,
    pub continuous_output: bool,
}

impl Default for Smoothing {
    fn default() -> Self {
        Self {
            enable: true,
            ty: SmoothingType::Linear,
            time_ms: 200,
            update_frequency: 25.0,
            interpolation_rate: 1.0,
            output_rate: 1.0,
            decay: 1.0,
            dithering: true,
            update_delay: 0,
            continuous_output: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstanceConfig {
    #[validate]
    pub instance: Instance,
    #[validate]
    #[serde(default = "Default::default")]
    pub background_effect: BackgroundEffect,
    #[validate]
    #[serde(default = "Default::default")]
    pub black_border_detector: BlackBorderDetector,
    #[validate]
    #[serde(default = "Default::default")]
    pub boblight_server: BoblightServer,
    #[validate]
    #[serde(default = "Default::default")]
    pub color: ColorAdjustment,
    #[validate]
    #[serde(default = "Default::default")]
    pub device: Device,
    #[validate]
    #[serde(default = "Default::default")]
    pub effects: Effects,
    #[validate]
    #[serde(default = "Default::default")]
    pub foreground_effect: ForegroundEffect,
    #[validate]
    #[serde(default = "Default::default")]
    pub instance_capture: InstanceCapture,
    #[validate]
    #[serde(default = "Default::default")]
    pub led_config: LedConfig,
    #[validate]
    #[serde(default = "Default::default")]
    pub leds: Leds,
    #[validate]
    #[serde(default = "Default::default")]
    pub smoothing: Smoothing,
}

impl InstanceConfig {
    pub fn new_dummy(id: i32) -> Self {
        Self {
            instance: Instance {
                id,
                friendly_name: "Dummy device".to_owned(),
                enabled: true,
                last_use: chrono::Utc::now(),
            },
            background_effect: Default::default(),
            black_border_detector: Default::default(),
            boblight_server: Default::default(),
            color: Default::default(),
            device: Default::default(),
            effects: Default::default(),
            foreground_effect: Default::default(),
            instance_capture: Default::default(),
            led_config: Default::default(),
            leds: Default::default(),
            smoothing: Default::default(),
        }
    }
}
