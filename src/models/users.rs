use std::convert::TryFrom;

use serde_derive::{Deserialize, Serialize};
use sha2::Digest;
use thiserror::Error;

use super::default_none;
use crate::db::models as db_models;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("error parsing date: {0}")]
    Chrono(#[from] chrono::ParseError),
    #[error("error parsing uuid: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("error decoding hex data: {0}")]
    Hex(#[from] hex::FromHexError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct User {
    pub name: String,
    #[serde(
        serialize_with = "hex::serialize",
        deserialize_with = "hex::deserialize"
    )]
    pub password: Vec<u8>,
    #[serde(
        serialize_with = "hex::serialize",
        deserialize_with = "hex::deserialize"
    )]
    pub token: Vec<u8>,
    pub salt: Vec<u8>,
    #[serde(default = "default_none")]
    pub comment: Option<String>,
    #[serde(default = "default_none")]
    pub id: Option<String>,
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "chrono::Utc::now")]
    pub last_use: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub fn hyperion() -> Self {
        let name = "Hyperion".to_owned();
        let salt = Self::generate_salt();
        let token = Self::generate_token();
        let password = Self::hash_password("hyperion", &salt);
        let created_at = chrono::Utc::now();
        let last_use = created_at;

        Self {
            name,
            password,
            token,
            salt,
            comment: None,
            id: None,
            created_at,
            last_use,
        }
    }

    pub fn generate_token() -> Vec<u8> {
        let mut hasher = sha2::Sha512::default();
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        hasher.finalize().to_vec()
    }

    pub fn generate_salt() -> Vec<u8> {
        hex::encode(Self::generate_token()).into_bytes()
    }

    pub fn hash_password(password: &str, salt: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha512::default();
        hasher.update(password.as_bytes());
        hasher.update(salt);
        hasher.finalize().to_vec()
    }
}

impl TryFrom<db_models::DbUser> for User {
    type Error = UserError;

    fn try_from(db: db_models::DbUser) -> Result<Self, Self::Error> {
        Ok(Self {
            name: db.user,
            password: hex::decode(db.password)?,
            token: hex::decode(db.token)?,
            salt: hex::decode(db.salt)?,
            comment: db.comment,
            id: db.id,
            created_at: chrono::DateTime::parse_from_rfc3339(&db.created_at)?
                .with_timezone(&chrono::Utc),
            last_use: chrono::DateTime::parse_from_rfc3339(&db.last_use)?
                .with_timezone(&chrono::Utc),
        })
    }
}
