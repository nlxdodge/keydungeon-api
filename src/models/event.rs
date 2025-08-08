use crate::models::event_type::EventType;

pub struct Event {
    uuid: String,
    event_type: EventType,
    metadata: String
}