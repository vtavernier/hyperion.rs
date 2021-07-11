use async_trait::async_trait;

use super::{Config, ConfigError};

mod db;
mod file;

#[async_trait]
pub trait ConfigBackend {
    async fn load(&mut self) -> Result<Config, ConfigError>;
}

pub use db::DbBackend;
pub use file::{ConfigExt, FileBackend};
