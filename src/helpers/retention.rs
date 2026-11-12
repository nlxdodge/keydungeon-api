use std::env;

use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::repositories::event_repository::EventRepository;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_RETENTION_DAYS: i64 = 90;
const DEFAULT_CLEANUP_CRON: &str = "0 0 3 * * *";

#[derive(Clone, Debug)]
struct RetentionConfig {
    retention_days: i64,
    cleanup_cron: String,
}

impl RetentionConfig {
    fn from_env() -> Self {
        let retention_days = env::var("EVENT_RETENTION_DAYS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RETENTION_DAYS);

        let cleanup_cron = env::var("EVENT_CLEANUP_CRON")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_CLEANUP_CRON.to_string());

        Self {
            retention_days,
            cleanup_cron,
        }
    }
}

/// Starts the in-process audit event retention scheduler.
///
/// Expired, non-deletion events are removed once a day. Deletion events are
/// kept indefinitely as part of the audit trail.
pub async fn start(pool: SqlitePool) -> Result<JobScheduler, BoxError> {
    let config = RetentionConfig::from_env();
    let scheduler = JobScheduler::new().await?;
    let job_config = config.clone();
    let job = Job::new_async(config.cleanup_cron.as_str(), move |_job_id, _scheduler| {
        let pool = pool.clone();
        let config = job_config.clone();

        Box::pin(async move {
            match delete_expired(&pool, config.retention_days, Utc::now()).await {
                Ok(deleted) => {
                    tracing::info!(
                        deleted,
                        retention_days = config.retention_days,
                        "pruned expired audit events"
                    );
                }
                Err(error) => {
                    tracing::error!(%error, "failed to prune expired audit events");
                }
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    tracing::info!(
        retention_days = config.retention_days,
        cron = %config.cleanup_cron,
        "audit event retention scheduler started"
    );

    Ok(scheduler)
}

async fn delete_expired(
    pool: &SqlitePool,
    retention_days: i64,
    now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let cutoff = now - Duration::days(retention_days);
    EventRepository::new(pool.clone())
        .delete_expired_events(cutoff)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use uuid::Uuid;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();

        sqlx::raw_sql(include_str!("../../init.sql"))
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    async fn insert_event(pool: &SqlitePool, event_type: &str, timestamp: DateTime<Utc>) {
        sqlx::query(
            "INSERT INTO events (uuid, user_uuid, event_type, metadata, timestamp) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(event_type)
        .bind("{}")
        .bind(timestamp)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn event_types(pool: &SqlitePool) -> Vec<String> {
        sqlx::query_scalar("SELECT event_type FROM events ORDER BY event_type")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn cleanup_deletes_old_non_delete_events_and_keeps_delete_events() {
        let pool = test_pool().await;
        let now = Utc::now();
        let old = now - Duration::days(91);
        let recent = now - Duration::days(89);

        insert_event(&pool, "ShowPasswords", old).await;
        insert_event(&pool, "RevealPassword", old).await;
        insert_event(&pool, "SignIn", recent).await;
        insert_event(&pool, "DeleteUser", old).await;
        insert_event(&pool, "DeletePassword", old).await;
        insert_event(&pool, "DeleteVault", old).await;

        let deleted = delete_expired(&pool, 90, now).await.unwrap();

        assert_eq!(deleted, 2);
        assert_eq!(
            event_types(&pool).await,
            vec!["DeletePassword", "DeleteUser", "DeleteVault", "SignIn"]
        );
    }

    #[tokio::test]
    async fn cleanup_keeps_events_newer_than_cutoff() {
        let pool = test_pool().await;
        let now = Utc::now();

        insert_event(&pool, "ShowPasswords", now - Duration::days(89)).await;
        insert_event(&pool, "RevealPassword", now).await;

        let deleted = delete_expired(&pool, 90, now).await.unwrap();

        assert_eq!(deleted, 0);
        assert_eq!(
            event_types(&pool).await,
            vec!["RevealPassword", "ShowPasswords"]
        );
    }
}
