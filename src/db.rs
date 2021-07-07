use sqlx::prelude::*;
use sqlx::SqliteConnection;
use thiserror::Error;

pub mod models;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("error connecting to the settings database: {0}")]
    Connection(#[from] sqlx::Error),
    #[error("failed to find default path")]
    InvalidDefaultPath,
}

pub struct Db {
    connection: SqliteConnection,
}

impl Db {
    pub async fn try_default(path: Option<&str>) -> Result<Self, DbError> {
        let default_path;
        let path = if let Some(path) = path {
            path
        } else {
            default_path = std::env::var("DATABASE_URL")
                .map(|v| v.to_string())
                .or_else(|_| {
                    dirs::home_dir()
                        .and_then(|path| {
                            path.join(".config/hyperion.rs/hyperion.db")
                                .to_str()
                                .map(str::to_owned)
                        })
                        .ok_or_else(|| DbError::InvalidDefaultPath)
                })?;

            &default_path
        };

        Ok(Self::connect(path).await?)
    }

    pub async fn connect(path: &str) -> Result<Self, DbError> {
        debug!(path = %path, "loading database");

        Ok(Self {
            connection: SqliteConnection::connect(path).await?,
        })
    }
}

impl std::ops::Deref for Db {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl std::ops::DerefMut for Db {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}
