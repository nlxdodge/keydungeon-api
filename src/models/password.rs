use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::chrono::NaiveDateTime};
use uuid::Uuid;
use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Clone, FromRow)]
pub struct Password {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    pub icon: String,
    pub url: String,
    pub name: String,
    pub username: String,
    pub password: String,
    pub timestamp: NaiveDateTime,
}

impl Drop for Password {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}
