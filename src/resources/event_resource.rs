use axum::{
    extract::{Extension, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{auth::claims::Claims, models::event::EventType};
use crate::models::event::Event;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct CreateEventRequest {
    pub uuid: Uuid,
    pub event_type: String,
    pub metadata: Option<String>,
}

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/", get(get_events))
        .route("/", post(save_events))
}

async fn get_events(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Event>>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;

    let events = sqlx::query_as::<_, Event>(
        "SELECT uuid, user_uuid, event_type, metadata, timestamp FROM events WHERE user_uuid = $1 LIMIT 50",
    )
    .bind(user_uuid)
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to fetch events".to_string(),
            }),
        )
    })?;

    Ok(Json(events))
}

async fn save_events(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<Event>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;

    let event = sqlx::query_as::<_, Event>(
        "INSERT INTO events (uuid, user_uuid, event_type, metadata, timestamp) VALUES ($1, $2, $3, $4, $5) RETURNING uuid, user_uuid, event_type, metadata, timestamp",
    )
    .bind(payload.uuid)
    .bind(user_uuid)
    .bind(&payload.event_type)
    .bind(&payload.metadata)
    .bind(Local::now())
    .fetch_one(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to save event".to_string(),
            }),
        )
    })?;

    Ok(Json(event))
}

pub async fn log_event(
    pool: &Pool<Postgres>,
    user_uuid: Uuid,
    event_type: EventType,
    metadata: Option<&str>,
) -> Result<Event, sqlx::Error> {
    let event_uuid = Uuid::new_v4();

    sqlx::query_as::<_, Event>(
        "INSERT INTO events (uuid, user_uuid, event_type, metadata, timestamp) VALUES ($1, $2, $3, $4, $5) RETURNING uuid, user_uuid, event_type, metadata, timestamp",
    )
    .bind(event_uuid)
    .bind(user_uuid)
    .bind(event_type)
    .bind(metadata)
    .bind(Local::now())
    .fetch_one(pool)
    .await
}
