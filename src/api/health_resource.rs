use axum::{Router, extract::State, http::StatusCode, routing::get};

use crate::models::app_state::AppState;

pub fn routing() -> Router<AppState> {
    Router::new().route("/", get(get_health))
}

async fn get_health(State(app_state): State<AppState>) -> StatusCode {
    if db_accessible(&app_state).await {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn db_accessible(app_state: &AppState) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&app_state.db)
        .await
        .is_ok()
}
