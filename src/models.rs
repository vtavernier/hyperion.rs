use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
};

use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, EnumString};
use thiserror::Error;
use validator::Validate;

use crate::db::models as db_models;

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

impl From<GlobalConfigCreator> for GlobalConfig {
    fn from(creator: GlobalConfigCreator) -> Self {
        Self {
            flatbuffers_server: creator.flatbuffers_server.unwrap_or_default(),
            forwarder: creator.forwarder.unwrap_or_default(),
            framegrabber: creator.framegrabber.unwrap_or_default(),
            general: creator.general.unwrap_or_default(),
            grabber_v4l2: creator.grabber_v4l2.unwrap_or_default(),
            json_server: creator.json_server.unwrap_or_default(),
            logger: creator.logger.unwrap_or_default(),
            network: creator.network.unwrap_or_default(),
            proto_server: creator.proto_server.unwrap_or_default(),
            web_config: creator.web_config.unwrap_or_default(),
            hooks: creator.hooks.unwrap_or_default(),
        }
    }
}

#[derive(Default)]
struct GlobalConfigCreator {
    flatbuffers_server: Option<FlatbuffersServer>,
    forwarder: Option<Forwarder>,
    framegrabber: Option<Framegrabber>,
    general: Option<General>,
    grabber_v4l2: Option<GrabberV4L2>,
    json_server: Option<JsonServer>,
    logger: Option<Logger>,
    network: Option<Network>,
    proto_server: Option<ProtoServer>,
    web_config: Option<WebConfig>,
    hooks: Option<Hooks>,
}

struct InstanceConfigCreator {
    instance: Instance,
    background_effect: Option<BackgroundEffect>,
    black_border_detector: Option<BlackBorderDetector>,
    boblight_server: Option<BoblightServer>,
    color: Option<ColorAdjustment>,
    device: Option<Device>,
    effects: Option<Effects>,
    foreground_effect: Option<ForegroundEffect>,
    instance_capture: Option<InstanceCapture>,
    led_config: Option<LedConfig>,
    leds: Option<Leds>,
    smoothing: Option<Smoothing>,
}

impl From<InstanceConfigCreator> for InstanceConfig {
    fn from(creator: InstanceConfigCreator) -> Self {
        Self {
            instance: creator.instance,
            background_effect: creator.background_effect.unwrap_or_default(),
            black_border_detector: creator.black_border_detector.unwrap_or_default(),
            boblight_server: creator.boblight_server.unwrap_or_default(),
            color: creator.color.unwrap_or_default(),
            device: creator.device.unwrap_or_default(),
            effects: creator.effects.unwrap_or_default(),
            foreground_effect: creator.foreground_effect.unwrap_or_default(),
            instance_capture: creator.instance_capture.unwrap_or_default(),
            led_config: creator.led_config.unwrap_or_default(),
            leds: creator.leds.unwrap_or_default(),
            smoothing: creator.smoothing.unwrap_or_default(),
        }
    }
}

impl InstanceConfigCreator {
    fn new(instance: Instance) -> Self {
        Self {
            instance,
            background_effect: None,
            black_border_detector: None,
            boblight_server: None,
            color: None,
            device: None,
            effects: None,
            foreground_effect: None,
            instance_capture: None,
            led_config: None,
            leds: None,
            smoothing: None,
        }
    }
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
    pub async fn load(db: &mut crate::db::Db) -> Result<Self, ConfigError> {
        let mut instances = BTreeMap::new();
        let mut global = GlobalConfigCreator::default();

        for instance in sqlx::query_as::<_, db_models::DbInstance>("SELECT * FROM instances")
            .fetch_all(&mut **db)
            .await?
            .into_iter()
            .map(Instance::try_from)
        {
            let instance = instance?;
            instances.insert(instance.id, InstanceConfigCreator::new(instance));
        }

        for setting in sqlx::query_as::<_, db_models::DbSetting>("SELECT * FROM settings")
            .fetch_all(&mut **db)
            .await?
            .into_iter()
            .map(Setting::try_from)
        {
            let setting = setting?;
            match setting.config {
                SettingData::BackgroundEffect(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("backgroundEffect"))?,
                        )
                        .unwrap()
                        .background_effect = Some(config)
                }
                SettingData::BlackBorderDetector(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("blackborderdetector"))?,
                        )
                        .unwrap()
                        .black_border_detector = Some(config)
                }
                SettingData::BoblightServer(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("boblightServer"))?,
                        )
                        .unwrap()
                        .boblight_server = Some(config)
                }
                SettingData::ColorAdjustment(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("color"))?,
                        )
                        .unwrap()
                        .color = Some(config)
                }
                SettingData::Device(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("device"))?,
                        )
                        .unwrap()
                        .device = Some(config)
                }
                SettingData::Effects(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("effects"))?,
                        )
                        .unwrap()
                        .effects = Some(config)
                }
                SettingData::ForegroundEffect(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("foregroundEffect"))?,
                        )
                        .unwrap()
                        .foreground_effect = Some(config)
                }
                SettingData::InstanceCapture(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("instCapture"))?,
                        )
                        .unwrap()
                        .instance_capture = Some(config)
                }
                SettingData::LedConfig(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("ledConfig"))?,
                        )
                        .unwrap()
                        .led_config = Some(config)
                }
                SettingData::Leds(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("leds"))?,
                        )
                        .unwrap()
                        .leds = Some(config)
                }
                SettingData::Smoothing(config) => {
                    instances
                        .get_mut(
                            &setting
                                .hyperion_inst
                                .ok_or(ConfigError::MissingHyperionInst("smoothing"))?,
                        )
                        .unwrap()
                        .smoothing = Some(config)
                }

                SettingData::FlatbuffersServer(config) => {
                    global.flatbuffers_server = Some(config);
                }
                SettingData::Forwarder(config) => {
                    global.forwarder = Some(config);
                }
                SettingData::Framegrabber(config) => {
                    global.framegrabber = Some(config);
                }
                SettingData::General(config) => {
                    global.general = Some(config);
                }
                SettingData::GrabberV4L2(config) => {
                    global.grabber_v4l2 = Some(config);
                }
                SettingData::JsonServer(config) => {
                    global.json_server = Some(config);
                }
                SettingData::Logger(config) => {
                    global.logger = Some(config);
                }
                SettingData::Network(config) => {
                    global.network = Some(config);
                }
                SettingData::ProtoServer(config) => {
                    global.proto_server = Some(config);
                }
                SettingData::WebConfig(config) => {
                    global.web_config = Some(config);
                }
                SettingData::Hooks(config) => {
                    global.hooks = Some(config);
                }
            }
        }

        let meta: Result<Vec<_>, _> = sqlx::query_as::<_, db_models::DbMeta>("SELECT * FROM meta")
            .fetch_all(&mut **db)
            .await?
            .into_iter()
            .map(Meta::try_from)
            .collect();
        let meta = meta?;

        let users: Result<Vec<_>, _> = sqlx::query_as::<_, db_models::DbUser>("SELECT * FROM auth")
            .fetch_all(&mut **db)
            .await?
            .into_iter()
            .map(User::try_from)
            .collect();
        let users = users?;

        let instances: BTreeMap<i32, InstanceConfig> =
            instances.into_iter().map(|(k, v)| (k, v.into())).collect();

        let global: GlobalConfig = global.into();

        debug!(
            name = %global.general.name,
            instances = %instances.len(),
            meta = %meta.len(),
            users = %users.len(),
            "loaded",
        );

        Ok(Self {
            instances,
            global,
            meta,
            users,
        })
    }

    pub async fn load_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        use tokio::io::AsyncReadExt;

        let mut file = tokio::fs::File::open(path).await?;
        let mut full = String::new();
        file.read_to_string(&mut full).await?;

        let config: DeserializableConfig = toml::from_str(&full)?;
        Ok(config.try_into()?)
    }

    pub fn to_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(&SerializableConfig::from(self))
    }

    pub fn uuid(&self) -> uuid::Uuid {
        // There should always be a meta uuid
        self.meta.first().map(|meta| meta.uuid).unwrap_or_default()
    }
}

#[derive(Serialize)]
struct SerializableConfig<'c> {
    instances: BTreeMap<String, &'c InstanceConfig>,
    global: &'c GlobalConfig,
    meta: &'c Vec<Meta>,
    users: &'c Vec<User>,
}

impl<'c> From<&'c Config> for SerializableConfig<'c> {
    fn from(config: &'c Config) -> Self {
        Self {
            instances: config
                .instances
                .iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            global: &config.global,
            meta: &config.meta,
            users: &config.users,
        }
    }
}

fn default_meta() -> Vec<Meta> {
    vec![Meta::new()]
}

fn default_users() -> Vec<User> {
    vec![User::hyperion()]
}

#[derive(Deserialize)]
struct DeserializableConfig {
    instances: BTreeMap<String, InstanceConfig>,
    #[serde(default)]
    global: GlobalConfig,
    #[serde(default = "default_meta")]
    meta: Vec<Meta>,
    #[serde(default = "default_users")]
    users: Vec<User>,
}

impl TryFrom<DeserializableConfig> for Config {
    type Error = ConfigError;

    fn try_from(value: DeserializableConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            instances: value
                .instances
                .into_iter()
                .map(|(k, v)| {
                    k.parse()
                        .map_err(|_| ConfigError::InvalidId(k.clone()))
                        .map(|k| (k, v))
                })
                .collect::<Result<_, _>>()?,
            global: value.global,
            meta: value.meta,
            users: value.users,
        })
    }
}
