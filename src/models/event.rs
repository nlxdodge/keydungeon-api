use crate::models::event_type::EventType;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::chrono::NaiveDateTime};
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow)]
pub struct Event {
    uuid: Uuid,
    user_uuid: Uuid,
    event_type: EventType,
    metadata: String,
    timestamp: NaiveDateTime,
}
