use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow)]
pub struct User {
    pub uuid: Uuid,
    pub username: String,
    /// Argon2 PHC hash. Never serialized to API clients.
    #[serde(skip_serializing)]
    pub password: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub password: Option<String>,
}
