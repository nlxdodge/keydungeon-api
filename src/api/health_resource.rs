use axum::{Json, Router, extract::State, routing::get};

use crate::models::{app_state::AppState, health::Health};

pub fn routing() -> Router<AppState> {
    Router::new().route("/", get(get_health))
}

async fn get_health(State(app_state): State<AppState>) -> Json<Health> {
    Json(Health {
        endpoints: true,
        database: db_accessable(&app_state).await,
    })
}

async fn db_accessable(app_state: &AppState) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&app_state.db)
        .await
        .is_ok()
}
