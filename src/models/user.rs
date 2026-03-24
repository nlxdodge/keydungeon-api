use serde::{Deserialize, Serialize};
use sqlx::{FromRow};
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow)]
pub struct User {
    pub uuid: Uuid,
    pub username: String,
    pub password: String,
}
