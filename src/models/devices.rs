use ambassador::{delegatable_trait, Delegate};
use derive_more::From;
use serde_derive::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use validator::Validate;

use super::{default_false, ColorOrder};

#[delegatable_trait]
pub trait DeviceConfig: Sync + Send {
    fn hardware_led_count(&self) -> usize;

    fn rewrite_time(&self) -> Option<std::time::Duration> {
        None
    }

    fn latch_time(&self) -> std::time::Duration {
        Default::default()
    }
}

macro_rules! impl_device_config {
    ($t:ty) => {
        impl DeviceConfig for $t {
            fn hardware_led_count(&self) -> usize {
                self.hardware_led_count as _
            }

            fn rewrite_time(&self) -> Option<std::time::Duration> {
                if self.rewrite_time == 0 {
                    None
                } else {
                    Some(std::time::Duration::from_millis(self.rewrite_time as _))
                }
            }

            fn latch_time(&self) -> std::time::Duration {
                std::time::Duration::from_millis(self.latch_time as _)
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Dummy {
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    pub rewrite_time: u32,
    pub latch_time: u32,
}

impl_device_config!(Dummy);

impl Default for Dummy {
    fn default() -> Self {
        Self {
            hardware_led_count: 1,
            rewrite_time: 0,
            latch_time: 0,
        }
    }
}

fn default_ws_spi_rate() -> i32 {
    3000000
}

fn default_ws_spi_rewrite_time() -> u32 {
    1000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ws2812Spi {
    #[serde(default = "Default::default")]
    pub color_order: ColorOrder,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    #[serde(default = "default_false")]
    pub invert: bool,
    #[serde(default = "Default::default")]
    pub latch_time: u32,
    pub output: String,
    #[serde(default = "default_ws_spi_rate")]
    pub rate: i32,
    #[serde(default = "default_ws_spi_rewrite_time")]
    pub rewrite_time: u32,
}

impl_device_config!(Ws2812Spi);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhilipsHue {
    pub black_lights_timeout: i32,
    pub brightness_factor: f32,
    pub brightness_max: f32,
    pub brightness_min: f32,
    pub brightness_threshold: f32,
    #[serde(rename = "clientkey")]
    pub client_key: String,
    pub color_order: ColorOrder,
    pub debug_level: String,
    pub debug_streamer: bool,
    pub group_id: i32,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    pub light_ids: Vec<String>,
    pub output: String,
    pub restore_original_state: bool,
    #[serde(rename = "sslHSTimeoutMax")]
    pub ssl_hs_timeout_max: i32,
    #[serde(rename = "sslHSTimeoutMin")]
    pub ssl_hs_timeout_min: i32,
    pub ssl_read_timeout: i32,
    pub switch_off_on_black: bool,
    #[serde(rename = "transitiontime")]
    pub transition_time: f32,
    #[serde(rename = "useEntertainmentAPI")]
    pub use_entertainment_api: bool,
    pub username: String,
    pub verbose: bool,
}

impl DeviceConfig for PhilipsHue {
    fn hardware_led_count(&self) -> usize {
        self.hardware_led_count as _
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr, Delegate, From)]
#[serde(rename_all = "lowercase", tag = "type", deny_unknown_fields)]
#[delegate(DeviceConfig)]
pub enum Device {
    Dummy(Dummy),
    Ws2812Spi(Ws2812Spi),
    PhilipsHue(PhilipsHue),
}

impl Default for Device {
    fn default() -> Self {
        Self::Dummy(Dummy::default())
    }
}

impl Validate for Device {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            Device::Dummy(device) => device.validate(),
            Device::Ws2812Spi(device) => device.validate(),
            Device::PhilipsHue(device) => device.validate(),
        }
    }
}
