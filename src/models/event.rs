use crate::models::event_type::EventType;
use serde::Serialize;

#[derive(Serialize)]
pub struct Event {
    uuid: String,
    event_type: EventType,
    metadata: String
}