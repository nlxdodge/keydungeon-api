use axum::Router;
use axum::middleware;

use crate::api::{
    auth_resource, event_resource, health_resource, password_resources, user_resource,
};
use crate::handlers::jwt::jwt_middleware;
use crate::models::app_state::AppState;

pub fn create_router(state: AppState) -> Router {
    let router = Router::new()
        .nest("/auth", auth_resource::routing())
        .nest("/health", health_resource::routing());

    let protected = Router::new()
        .nest("/users", user_resource::routing())
        .nest("/passwords", password_resources::routing())
        .nest("/events", event_resource::routing())
        .layer(middleware::from_fn(jwt_middleware));

    router.merge(protected).with_state(state)
}
