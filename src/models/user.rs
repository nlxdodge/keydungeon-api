use serde::Serialize;
use sqlx::{FromRow};
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct User {
    pub uuid: Uuid,
    pub username: String,
    pub password: String,
}
