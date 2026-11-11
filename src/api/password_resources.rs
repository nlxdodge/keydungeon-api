use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{ConnectInfo, Extension, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::handlers::claims::Claims;
use crate::handlers::events::record_event;
use crate::models::app_state::AppState;
use crate::models::error_response::ErrorResponse;
use crate::models::event::EventType;
use crate::models::pagination::{Paginated, Pagination};
use crate::models::password::Password;

#[derive(Deserialize)]
pub struct PasswordRequest {
    pub uuid: Uuid,
    pub icon: String,
    pub url: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

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
    Uuid::parse_str(raw)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Invalid password UUID format"))
}

fn map_password(user_uuid: Uuid, uuid: Uuid, request: PasswordRequest) -> Password {
    Password {
        uuid,
        user_uuid,
        icon: request.icon,
        url: request.url,
        name: request.name,
        username: request.username,
        password: request.password,
        timestamp: Utc::now(),
    }
}

pub fn routing() -> Router<AppState> {
    Router::new()
        .route("/", get(get_passwords))
        .route("/", post(save_password))
        .route("/{uuid}", get(get_password))
        .route("/{uuid}", patch(edit_password))
        .route("/{uuid}", delete(remove_password))
}

async fn get_passwords(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Paginated<Password>>, ApiError> {
    let user_uuid = claims.sub;
    let page = pagination
        .resolve()
        .map_err(|message| api_error(StatusCode::BAD_REQUEST, message))?;

    let passwords = app_state
        .passwords
        .get_passwords(user_uuid, page.limit, page.offset())
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to fetch passwords");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch passwords",
            )
        })?;

    let total = app_state
        .passwords
        .count_passwords(user_uuid)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to count passwords");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to count passwords",
            )
        })?;

    record_event(
        &app_state,
        user_uuid,
        EventType::ShowPasswords,
        format!("IP address: {}", addr.ip()),
    )
    .await;

    Ok(Json(Paginated::new(passwords, page, total)))
}

async fn get_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(uuid): Path<String>,
) -> Result<Json<String>, ApiError> {
    let user_uuid = claims.sub;
    let password_uuid = parse_uuid(&uuid)?;

    let password = app_state
        .passwords
        .get_password(user_uuid, password_uuid)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to reveal password");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch password",
            )
        })?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Password not found"))?;

    record_event(
        &app_state,
        user_uuid,
        EventType::RevealPassword,
        format!("Password UUID: {password_uuid}, IP address: {}", addr.ip()),
    )
    .await;

    Ok(Json(password))
}

async fn save_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<PasswordRequest>,
) -> Result<(StatusCode, Json<Password>), ApiError> {
    let user_uuid = claims.sub;
    let password = map_password(user_uuid, request.uuid, request);

    let saved = app_state
        .passwords
        .save_password(password)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to save password");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to save password")
        })?;

    record_event(
        &app_state,
        user_uuid,
        EventType::CreatePassword,
        format!("Password name: {}, IP address: {}", saved.name, addr.ip()),
    )
    .await;

    Ok((StatusCode::CREATED, Json(saved)))
}

async fn edit_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(uuid): Path<String>,
    Json(request): Json<PasswordRequest>,
) -> Result<Json<Password>, ApiError> {
    let user_uuid = claims.sub;
    let password_uuid = parse_uuid(&uuid)?;
    let password = map_password(user_uuid, password_uuid, request);

    let updated = app_state
        .passwords
        .update_password(user_uuid, password)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to update password");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update password",
            )
        })?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Password not found"))?;

    record_event(
        &app_state,
        user_uuid,
        EventType::EditPassword,
        format!("Password name: {}, IP address: {}", updated.name, addr.ip()),
    )
    .await;

    Ok(Json(updated))
}

async fn remove_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(uuid): Path<String>,
) -> Result<StatusCode, ApiError> {
    let user_uuid = claims.sub;
    let password_uuid = parse_uuid(&uuid)?;

    let deleted = app_state
        .passwords
        .delete_password(user_uuid, password_uuid)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to delete password");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete password",
            )
        })?;

    if !deleted {
        return Err(api_error(StatusCode::NOT_FOUND, "Password not found"));
    }

    record_event(
        &app_state,
        user_uuid,
        EventType::DeletePassword,
        format!("Password UUID: {password_uuid}, IP address: {}", addr.ip()),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
