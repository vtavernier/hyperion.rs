use std::{
    collections::BTreeMap,
    convert::TryFrom,
    path::{Path, PathBuf},
};

use async_trait::async_trait;

use super::ConfigBackend;
use crate::models::*;

pub trait ConfigExt {
    fn to_string(&self) -> Result<String, toml::ser::Error>;
}

impl ConfigExt for Config {
    fn to_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(&SerializableConfig::from(self))
    }
}

pub struct FileBackend {
    path: PathBuf,
}

impl FileBackend {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
}

#[async_trait]
impl ConfigBackend for FileBackend {
    async fn load(&mut self) -> Result<Config, ConfigError> {
        use tokio::io::AsyncReadExt;

        let mut file = tokio::fs::File::open(&self.path).await?;
        let mut full = String::new();
        file.read_to_string(&mut full).await?;

        let config: DeserializableConfig = toml::from_str(&full)?;
        Ok(config.try_into()?)
    }
}

#[derive(Serialize)]
struct SerializableConfig<'c> {
    instances: BTreeMap<String, &'c InstanceConfig>,
    #[serde(flatten)]
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
    #[serde(default, flatten)]
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
