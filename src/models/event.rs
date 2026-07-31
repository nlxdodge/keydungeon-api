use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::Type;
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow)]
pub struct Event {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    pub event_type: EventType,
    pub metadata: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Type)]
#[sqlx(type_name = "TEXT")]
pub enum EventType {
    SignIn,
    SignOut,
    CreatePassword,
    ShowPasswords,
    RevealPassword,
    EditPassword,
    DeletePassword,
    CreateUser,
    ShowUser,
    EditUser,
    DeleteUser,
}
