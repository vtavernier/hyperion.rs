use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct DbUser {
    pub user: String,
    pub password: Vec<u8>,
    pub token: Vec<u8>,
    pub salt: Vec<u8>,
    pub comment: Option<String>,
    pub id: Option<String>,
    pub created_at: String,
    pub last_use: String,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct DbInstance {
    pub instance: i32,
    pub friendly_name: String,
    pub enabled: i32,
    pub last_use: String,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct DbMeta {
    pub uuid: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct DbSetting {
    #[sqlx(rename = "type")]
    pub ty: String,
    pub config: String,
    pub hyperion_inst: Option<i32>,
    pub updated_at: String,
}
