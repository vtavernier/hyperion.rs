use std::path::Path;

use sqlx::prelude::*;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqliteConnection;

pub mod models;

pub type DbError = sqlx::Error;

pub struct Db {
    connection: SqliteConnection,
}

impl Db {
    pub async fn open(path: &Path) -> Result<Self, DbError> {
        debug!(path = %path.display(), "loading database");

        Ok(Self {
            connection: SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(path))
                .await?,
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
