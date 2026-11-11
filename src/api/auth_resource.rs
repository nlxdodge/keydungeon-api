use std::env;
use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{ConnectInfo, Extension, State},
    http::StatusCode,
    routing::post,
};
use uuid::Uuid;

use crate::handlers::claims::Claims;
use crate::handlers::events::record_event;
use crate::handlers::jwt::TokenManager;
use crate::helpers::password;
use crate::models::app_state::AppState;
use crate::models::auth::{AuthRequest, AuthResponse};
use crate::models::error_response::ErrorResponse;
use crate::models::event::EventType;
use crate::repositories::user_repository::UserRepositoryError;

type ApiError = (StatusCode, Json<ErrorResponse>);

fn api_error(status: StatusCode, message: impl Into<String>) -> ApiError {
    (
        status,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

fn get_jwt_valid_time() -> i64 {
    env::var("JWT_VALID_TIME")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24)
}

/// Public authentication routes.
pub fn routing() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
}

/// Authentication routes that require a valid JWT.
pub fn protected_routing() -> Router<AppState> {
    Router::new().route("/verify", post(verify))
}

async fn login(
    State(app_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let username = payload.username.trim();

    if username.is_empty() || payload.password.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Username and password are required",
        ));
    }

    let user = app_state
        .users
        .find_by_username(username)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to look up user");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Invalid username or password"))?;

    let valid = password::verify_async(payload.password, user.password.clone())
        .await
        .map_err(|error| {
            tracing::error!(%error, "password verification failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password verification failed",
            )
        })?;

    if !valid {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Invalid username or password",
        ));
    }

    record_event(
        &app_state,
        user.uuid,
        EventType::SignIn,
        format!("IP address: {}", addr.ip()),
    )
    .await;

    Ok(Json(auth_response(&app_state.jwt, user.uuid)?))
}

/// Register endpoint - creates a new user and returns a JWT token.
async fn register(
    State(app_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    let username = payload.username.trim();

    if username.is_empty() || payload.password.len() < 8 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Username required, password must be at least 8 characters",
        ));
    }

    if app_state
        .users
        .username_exists(username)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to check username");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?
    {
        return Err(api_error(StatusCode::CONFLICT, "Username already exists"));
    }

    let user = match app_state.users.create(username, payload.password).await {
        Ok(user) => user,
        Err(UserRepositoryError::UsernameTaken) => {
            return Err(api_error(StatusCode::CONFLICT, "Username already exists"));
        }
        Err(error) => {
            tracing::error!(%error, "failed to create user");
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create user",
            ));
        }
    };

    record_event(
        &app_state,
        user.uuid,
        EventType::CreateUser,
        format!("Username: {}, IP address: {}", user.username, addr.ip()),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(auth_response(&app_state.jwt, user.uuid)?),
    ))
}

async fn verify(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AuthResponse>, ApiError> {
    if claims.is_expired() {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Token has expired, login again!",
        ));
    }

    let user = app_state
        .users
        .find_by_uuid(claims.sub)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to look up user");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;

    if user.is_none() {
        return Err(api_error(StatusCode::UNAUTHORIZED, "User no longer exists"));
    }

    Ok(Json(auth_response(&app_state.jwt, claims.sub)?))
}

fn auth_response(jwt: &TokenManager, user_uuid: Uuid) -> Result<AuthResponse, ApiError> {
    let valid_time = get_jwt_valid_time();
    let token = jwt
        .generate_token_with_duration(user_uuid, valid_time)
        .map_err(|error| {
            tracing::error!(?error, "failed to generate token");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate token",
            )
        })?;
    let claims = Claims::with_duration(user_uuid, valid_time);

    Ok(AuthResponse { token, claims })
}
