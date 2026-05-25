use serde::{Deserialize, Serialize};
use sqlx::{types::chrono::NaiveDateTime, FromRow};
use uuid::Uuid;
use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Clone, FromRow)]
pub struct Password {
    pub uuid: Uuid,
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
