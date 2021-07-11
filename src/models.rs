use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
};

use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, EnumString};
use thiserror::Error;
use validator::Validate;

use crate::db::models as db_models;

pub mod backend;

mod devices;
pub use devices::*;

mod global;
pub use global::*;

mod instance;
pub use instance::*;

mod layouts;
pub use layouts::*;

mod meta;
pub use meta::*;

mod users;
pub use users::*;

pub type Color = palette::rgb::LinSrgb<u8>;
pub type Color16 = palette::rgb::LinSrgb<u16>;

pub trait ServerConfig {
    fn port(&self) -> u16;
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Setting {
    pub hyperion_inst: Option<i32>,
    pub config: SettingData,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, EnumDiscriminants, Deserialize)]
#[strum_discriminants(name(SettingKind), derive(EnumString))]
pub enum SettingData {
    // hyperion.ng settings
    BackgroundEffect(BackgroundEffect),
    BlackBorderDetector(BlackBorderDetector),
    BoblightServer(BoblightServer),
    ColorAdjustment(ColorAdjustment),
    Device(Device),
    Effects(Effects),
    FlatbuffersServer(FlatbuffersServer),
    ForegroundEffect(ForegroundEffect),
    Forwarder(Forwarder),
    Framegrabber(Framegrabber),
    General(General),
    GrabberV4L2(GrabberV4L2),
    InstanceCapture(InstanceCapture),
    JsonServer(JsonServer),
    LedConfig(LedConfig),
    Leds(Leds),
    Logger(Logger),
    Network(Network),
    ProtoServer(ProtoServer),
    Smoothing(Smoothing),
    WebConfig(WebConfig),
    // hyperion.rs settings
    Hooks(Hooks),
}

impl Validate for SettingData {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            SettingData::BackgroundEffect(setting) => setting.validate(),
            SettingData::BlackBorderDetector(setting) => setting.validate(),
            SettingData::BoblightServer(setting) => setting.validate(),
            SettingData::ColorAdjustment(setting) => setting.validate(),
            SettingData::Device(setting) => setting.validate(),
            SettingData::Effects(setting) => setting.validate(),
            SettingData::FlatbuffersServer(setting) => setting.validate(),
            SettingData::ForegroundEffect(setting) => setting.validate(),
            SettingData::Forwarder(setting) => setting.validate(),
            SettingData::Framegrabber(setting) => setting.validate(),
            SettingData::General(setting) => setting.validate(),
            SettingData::GrabberV4L2(setting) => setting.validate(),
            SettingData::InstanceCapture(setting) => setting.validate(),
            SettingData::JsonServer(setting) => setting.validate(),
            SettingData::LedConfig(setting) => setting.validate(),
            SettingData::Leds(setting) => setting.validate(),
            SettingData::Logger(setting) => setting.validate(),
            SettingData::Network(setting) => setting.validate(),
            SettingData::ProtoServer(setting) => setting.validate(),
            SettingData::Smoothing(setting) => setting.validate(),
            SettingData::WebConfig(setting) => setting.validate(),
            SettingData::Hooks(setting) => setting.validate(),
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingErrorKind {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("error parsing date")]
    Chrono(#[from] chrono::ParseError),
    #[error(transparent)]
    Validation(#[from] validator::ValidationErrors),
    #[error("unknown setting type")]
    UnknownType,
}

#[derive(Debug)]
pub struct SettingError {
    pub kind: SettingErrorKind,
    pub setting: &'static str,
    unknown: Option<String>,
}

impl SettingError {
    pub fn new(kind: impl Into<SettingErrorKind>, setting: &'static str) -> Self {
        Self {
            kind: kind.into(),
            setting,
            unknown: None,
        }
    }

    pub fn unknown(name: &str) -> Self {
        Self {
            kind: SettingErrorKind::UnknownType,
            setting: "",
            unknown: Some(name.to_owned()),
        }
    }
}

impl std::error::Error for SettingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl std::fmt::Display for SettingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.unknown {
            Some(setting) => write!(f, "`{}`: {}", setting, self.kind),
            None => write!(f, "`{}`: {}", self.setting, self.kind),
        }
    }
}

macro_rules! convert_settings {
    ($db:ident, $($name:literal => $variant:ident),*) => {
        match $db.ty.as_str() {
            $($name => {
                let config = SettingData::$variant(serde_json::from_str(&$db.config).map_err(|err| {
                    SettingError::new(err, $name)
                })?);

                (
                    match config.validate() {
                        Ok(_) => Ok(config),
                        Err(err) => Err(SettingError::new(err, $name)),
                    }?,
                    chrono::DateTime::parse_from_rfc3339(&$db.updated_at).map_err(|err| {
                        SettingError::new(err, $name)
                    })?.with_timezone(&chrono::Utc)
                )
            },)*
            other => {
                return Err(SettingError::unknown(other));
            }
        }
    };
}

impl TryFrom<db_models::DbSetting> for Setting {
    type Error = SettingError;

    fn try_from(db: db_models::DbSetting) -> Result<Self, Self::Error> {
        let (config, updated_at) = convert_settings!(db,
            "backgroundEffect" => BackgroundEffect,
            "blackborderdetector" => BlackBorderDetector,
            "boblightServer" => BoblightServer,
            "color" => ColorAdjustment,
            "device" => Device,
            "effects" => Effects,
            "flatbufServer" => FlatbuffersServer,
            "foregroundEffect" => ForegroundEffect,
            "forwarder" => Forwarder,
            "framegrabber" => Framegrabber,
            "general" => General,
            "grabberV4L2" => GrabberV4L2,
            "instCapture" => InstanceCapture,
            "jsonServer" => JsonServer,
            "ledConfig" => LedConfig,
            "leds" => Leds,
            "logger" => Logger,
            "network" => Network,
            "protoServer" => ProtoServer,
            "smoothing" => Smoothing,
            "webConfig" => WebConfig,
            "hooks" => Hooks
        );

        Ok(Self {
            hyperion_inst: db.hyperion_inst,
            config,
            updated_at,
        })
    }
}

fn default_none<T>() -> Option<T> {
    None
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("i/o error")]
    Io(#[from] std::io::Error),
    #[error("error querying the database")]
    Sqlx(#[from] sqlx::Error),
    #[error("error loading instance")]
    Instance(#[from] InstanceError),
    #[error("error loading setting")]
    Setting(#[from] SettingError),
    #[error("error loading meta")]
    Meta(#[from] MetaError),
    #[error("error loading user")]
    User(#[from] UserError),
    #[error("missing hyperion_inst field on instance setting {0}")]
    MissingHyperionInst(&'static str),
    #[error("invalid TOML")]
    Toml(#[from] toml::de::Error),
    #[error("instance id must be an integer, got {0}")]
    InvalidId(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub instances: BTreeMap<i32, InstanceConfig>,
    pub global: GlobalConfig,
    meta: Vec<Meta>,
    users: Vec<User>,
}

impl Config {
    pub fn uuid(&self) -> uuid::Uuid {
        // There should always be a meta uuid
        self.meta.first().map(|meta| meta.uuid).unwrap_or_default()
    }
}
