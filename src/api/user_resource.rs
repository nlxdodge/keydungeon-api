use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{ConnectInfo, Extension, Path, State},
    http::StatusCode,
    routing::{delete, get, patch},
};
use uuid::Uuid;

use crate::handlers::claims::Claims;
use crate::handlers::events::record_event;
use crate::models::app_state::AppState;
use crate::models::error_response::ErrorResponse;
use crate::models::event::EventType;
use crate::models::user::{UpdateUserRequest, User};
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

fn parse_uuid(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| api_error(StatusCode::BAD_REQUEST, "Invalid user UUID format"))
}

fn ensure_owner(claims: &Claims, uuid: Uuid) -> Result<(), ApiError> {
    if claims.sub != uuid {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Unauthorized: you can only access your own account",
        ));
    }

    Ok(())
}

pub fn routing() -> Router<AppState> {
    Router::new()
        .route("/{uuid}", get(get_user))
        .route("/{uuid}", patch(update_user))
        .route("/{uuid}", delete(remove_user))
}

async fn get_user(
    State(app_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(claims): Extension<Claims>,
    Path(uuid): Path<String>,
) -> Result<Json<User>, ApiError> {
    let uuid = parse_uuid(&uuid)?;
    ensure_owner(&claims, uuid)?;

    let user = app_state
        .users
        .find_by_uuid(uuid)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to fetch user");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch user")
        })?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "User not found"))?;

    record_event(
        &app_state,
        uuid,
        EventType::ShowUser,
        format!("IP address: {}", addr.ip()),
    )
    .await;

    Ok(Json(user))
}

async fn update_user(
    State(app_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(claims): Extension<Claims>,
    Path(uuid): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<User>, ApiError> {
    let uuid = parse_uuid(&uuid)?;
    ensure_owner(&claims, uuid)?;

    if payload.username.is_none() && payload.password.is_none() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Provide a username or password to update",
        ));
    }

    if let Some(username) = &payload.username
        && username.trim().is_empty()
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Username cannot be empty",
        ));
    }

    if let Some(password) = &payload.password
        && password.len() < 8
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters",
        ));
    }

    let username = payload.username.map(|username| username.trim().to_string());
    let user = match app_state
        .users
        .update(uuid, username, payload.password)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return Err(api_error(StatusCode::NOT_FOUND, "User not found")),
        Err(UserRepositoryError::UsernameTaken) => {
            return Err(api_error(StatusCode::CONFLICT, "Username already exists"));
        }
        Err(error) => {
            tracing::error!(%error, "failed to update user");
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update user",
            ));
        }
    };

    record_event(
        &app_state,
        uuid,
        EventType::EditUser,
        format!("IP address: {}", addr.ip()),
    )
    .await;

    Ok(Json(user))
}

async fn remove_user(
    State(app_state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(claims): Extension<Claims>,
    Path(uuid): Path<String>,
) -> Result<StatusCode, ApiError> {
    let uuid = parse_uuid(&uuid)?;
    ensure_owner(&claims, uuid)?;

    record_event(
        &app_state,
        uuid,
        EventType::DeleteUser,
        format!("IP address: {}", addr.ip()),
    )
    .await;

    let deleted = app_state.users.delete(uuid).await.map_err(|error| {
        tracing::error!(%error, "failed to delete user");
        api_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete user")
    })?;

    if !deleted {
        return Err(api_error(StatusCode::NOT_FOUND, "User not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
