use serde::{Deserialize, Serialize};
use sqlx::Type;
use sqlx::{types::chrono::NaiveDateTime, FromRow};
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow)]
pub struct Event {
    uuid: Uuid,
    user_uuid: Uuid,
    event_type: EventType,
    metadata: String,
    timestamp: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Type)]
#[sqlx(type_name = "TEXT")]
pub enum EventType {
    SignIn,
    SignOut,
    CreatePassword,
    ShowPassword,
    EditPassword,
    DeletePassword,
    CreateUser,
    ShowUser,
    EditUser,
    DeleteUser,
}
