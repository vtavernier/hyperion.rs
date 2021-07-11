use std::{collections::BTreeMap, convert::TryFrom};

use async_trait::async_trait;

use super::ConfigBackend;
use crate::{
    db::{models as db_models, Db},
    models::*,
};

pub struct DbBackend {
    db: Db,
}

impl DbBackend {
    pub fn new(db: Db) -> Self {
        Self::from(db)
    }
}

impl From<Db> for DbBackend {
    fn from(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConfigBackend for DbBackend {
    async fn load(&mut self) -> Result<Config, ConfigError> {
        let mut instances = BTreeMap::new();
        let mut global = GlobalConfigCreator::default();

        for instance in sqlx::query_as::<_, db_models::DbInstance>("SELECT * FROM instances")
            .fetch_all(&mut *self.db)
            .await?
            .into_iter()
            .map(Instance::try_from)
        {
            let instance = instance?;
            instances.insert(instance.id, InstanceConfigCreator::new(instance));
        }

        for setting in sqlx::query_as::<_, db_models::DbSetting>("SELECT * FROM settings")
            .fetch_all(&mut *self.db)
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
            .fetch_all(&mut *self.db)
            .await?
            .into_iter()
            .map(Meta::try_from)
            .collect();
        let meta = meta?;

        let users: Result<Vec<_>, _> = sqlx::query_as::<_, db_models::DbUser>("SELECT * FROM auth")
            .fetch_all(&mut *self.db)
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

        Ok(Config {
            instances,
            global,
            meta,
            users,
        })
    }
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
