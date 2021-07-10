use std::convert::TryFrom;

use serde_derive::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::models as db_models;

#[derive(Debug, Error)]
pub enum MetaError {
    #[error("error parsing date: {0}")]
    Chrono(#[from] chrono::ParseError),
    #[error("error parsing uuid: {0}")]
    Uuid(#[from] uuid::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Meta {
    pub uuid: uuid::Uuid,
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Meta {
    pub fn new() -> Self {
        let intf = pnet::datalink::interfaces()
            .iter()
            .find_map(|intf| if !intf.is_loopback() { intf.mac } else { None })
            .unwrap_or_else(pnet::datalink::MacAddr::default);

        Self {
            uuid: uuid::Uuid::new_v5(&uuid::Uuid::default(), format!("{}", intf).as_bytes()),
            created_at: chrono::Utc::now(),
        }
    }
}

impl TryFrom<db_models::DbMeta> for Meta {
    type Error = MetaError;

    fn try_from(db: db_models::DbMeta) -> Result<Self, Self::Error> {
        Ok(Self {
            uuid: uuid::Uuid::parse_str(&db.uuid)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&db.created_at)?
                .with_timezone(&chrono::Utc),
        })
    }
}
