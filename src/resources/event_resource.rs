use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use sqlx::{Pool, Postgres};

use crate::models::event::Event;

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/events/:user_uuid", get(get_events))
        .route("/events/", post(save_events))
}

async fn get_events(
    Path(user_uuid): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Json<Vec<Event>> {
    let events = sqlx::query_as::<_, Event>(
        "SELECT uuid, user_uuid, event_type as \"event_type: model::EventType\", metadata, timestamp FROM events WHERE user_uuid = ? limit 25",
    )
    .bind(user_uuid)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch events");
    Json(events)
}

async fn save_events() {
    // let events = sqlx::query_as::<_, Event>(
    //     "INSERT INTO events where uuid=?, user_uuid=?, event_type=?, metadata=?, timestamp=?",
    // )
    // .bind(user_uuid)
    // .fetch_all(&pool)
    // .await
    // .expect("Failed to fetch events");
    // Json(events)
}
