use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::middleware;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::{
    auth_resource, event_resource, health_resource, password_resources, user_resource,
};
use crate::handlers::jwt::jwt_middleware;
use crate::models::app_state::AppState;

const MAX_BODY_BYTES: usize = 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn create_router(state: AppState) -> Router {
    // `/auth/verify` needs the JWT middleware while `/auth/login` and
    // `/auth/register` must stay public.
    let auth = auth_resource::routing().merge(auth_resource::protected_routing().layer(
        middleware::from_fn_with_state(state.clone(), jwt_middleware),
    ));

    let protected = Router::new()
        .nest("/users", user_resource::routing())
        .nest("/passwords", password_resources::routing())
        .nest("/events", event_resource::routing())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_middleware,
        ));

    Router::new()
        .nest("/auth", auth)
        .nest("/health", health_resource::routing())
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .with_state(state)
}
