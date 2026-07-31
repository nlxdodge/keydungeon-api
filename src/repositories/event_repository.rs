use sqlx::PgPool;
use uuid::Uuid;

use crate::models::event::Event;

#[derive(Clone)]
pub struct EventRepository {
    pub pool: PgPool,
}

impl EventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_events(&self, user_uuid: Uuid) -> Result<Vec<Event>, sqlx::Error> {
        sqlx::query_as::<_, Event>(
            "SELECT uuid, user_uuid, event_type, metadata, timestamp FROM events WHERE user_uuid = $1 LIMIT 50",
        )
        .bind(user_uuid)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn save_event(&self, event: Event) -> Result<Event, sqlx::Error> {
        sqlx::query_as::<_, Event>(
        "INSERT INTO events (uuid, user_uuid, event_type, metadata, timestamp) VALUES ($1, $2, $3, $4, $5) RETURNING uuid, user_uuid, event_type, metadata, timestamp",
        )
        .bind(event.uuid)
        .bind(event.user_uuid)
        .bind(event.event_type)
        .bind(event.metadata)
        .bind(event.timestamp)
        .fetch_one(&self.pool)
        .await
    }
}
