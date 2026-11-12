use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::event::Event;

#[derive(Clone)]
pub struct EventRepository {
    pub pool: SqlitePool,
}

impl EventRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_events(
        &self,
        user_uuid: Uuid,
        limit: u32,
        offset: i64,
    ) -> Result<Vec<Event>, sqlx::Error> {
        sqlx::query_as::<_, Event>(
            "SELECT uuid, user_uuid, event_type, metadata, timestamp FROM events WHERE user_uuid = $1 \
             ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(user_uuid)
        .bind(i64::from(limit))
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn count_events(&self, user_uuid: Uuid) -> Result<u64, sqlx::Error> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM events WHERE user_uuid = $1")
                .bind(user_uuid)
                .fetch_one(&self.pool)
                .await?;

        Ok(count as u64)
    }

    /// Deletes events older than `cutoff`, except deletion events.
    ///
    /// `DeleteUser` and `DeletePassword` (and any future `Delete*` type) are
    /// kept indefinitely as part of the audit trail.
    pub async fn delete_expired_events(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM events WHERE timestamp < $1 AND event_type NOT LIKE 'Delete%'",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
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
