use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::chrono::NaiveDateTime};
use uuid::Uuid;
use zeroize::{Zeroize};

#[derive(Serialize, Deserialize, FromRow, Zeroize)]
pub struct Password {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    pub icon: String,
    pub url: String,
    pub name: String,
    pub username: String,
    #[zeroize(drop)]
    pub password: String,
    pub timestamp: NaiveDateTime,
}
