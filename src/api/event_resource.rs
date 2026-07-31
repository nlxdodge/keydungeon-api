use axum::{
    Json, Router,
    extract::{Extension, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{error_response::ErrorResponse, event::Event};
use crate::{handlers::claims::Claims, models::app_state::AppState};

pub fn routing() -> Router<AppState> {
    Router::new().route("/", get(get_events))
}

async fn get_events(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Event>>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;
    match app_state.events.get_events(user_uuid).await {
        Ok(events) => Ok(Json(events)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to fetch events".to_string(),
            }),
        )),
    }
}
