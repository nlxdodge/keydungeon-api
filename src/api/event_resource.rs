use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{
    error_response::ErrorResponse,
    event::Event,
    pagination::{Paginated, Pagination},
};
use crate::{handlers::claims::Claims, models::app_state::AppState};

pub fn routing() -> Router<AppState> {
    Router::new().route("/", get(get_events))
}

async fn get_events(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Paginated<Event>>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;
    let page = pagination.resolve().map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message.to_string(),
            }),
        )
    })?;

    let events = app_state
        .events
        .get_events(user_uuid, page.limit, page.offset())
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch events".to_string(),
                }),
            )
        })?;

    let total = app_state
        .events
        .count_events(user_uuid)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to count events".to_string(),
                }),
            )
        })?;

    Ok(Json(Paginated::new(events, page, total)))
}
