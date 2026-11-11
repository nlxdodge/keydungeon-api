use chrono::Utc;
use uuid::Uuid;

use crate::models::app_state::AppState;
use crate::models::event::{Event, EventType};

/// Persist an audit event without failing the request when the audit write
/// itself fails.
pub async fn record_event(
    app_state: &AppState,
    user_uuid: Uuid,
    event_type: EventType,
    metadata: String,
) {
    let event = Event {
        uuid: Uuid::new_v4(),
        user_uuid,
        event_type,
        metadata,
        timestamp: Utc::now(),
    };

    if let Err(error) = app_state.events.save_event(event).await {
        tracing::warn!(%error, "failed to record audit event");
    }
}
