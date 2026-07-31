use std::net::SocketAddr;

use crate::models::app_state::AppState;
use crate::models::error_response::ErrorResponse;
use crate::models::event::Event;
use crate::{handlers::claims::Claims, models::event::EventType};

use crate::models::password::Password;
use axum::extract::ConnectInfo;
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PasswordRequest {
    pub uuid: Uuid,
    pub icon: String,
    pub url: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

fn map_password(user_uuid: Uuid, request: PasswordRequest) -> Password {
    Password {
        uuid: request.uuid,
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
        .route("/{uuid}", get(get_password))
        .route("/", post(save_password))
        .route("/{uuid}", patch(edit_password))
        .route("/{uuid}", delete(remove_password))
}

async fn get_passwords(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<Password>>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;

    let _ = app_state.events.save_event(Event {
        uuid: Uuid::new_v4(),
        user_uuid,
        event_type: EventType::ShowPasswords,
        metadata: format!("IP address: {}", addr.ip()),
        timestamp: Utc::now(),
    }).await ;

    match app_state.passwords.get_passwords(user_uuid).await {
        Ok(passwords) => Ok(Json(passwords)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to fetch passwords".to_string(),
            }),
        )),
    }
}

async fn get_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(uuid): Path<String>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;
    let password_uuid = Uuid::parse_str(&uuid).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid password UUID format".to_string(),
            }),
        )
    })?;

    let owned = app_state
        .passwords
        .owns_password(password_uuid, user_uuid)
        .await;
    if !owned {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Password not found or unauthorized".to_string(),
            }),
        ));
    }

    let _ = app_state.events.save_event(Event {
        uuid: Uuid::new_v4(),
        user_uuid,
        event_type: EventType::RevealPassword,
        metadata: format!("IP addres: {}", addr),
        timestamp: Utc::now(),
    }).await;

    match app_state
        .passwords
        .get_password(user_uuid, password_uuid)
        .await
    {
        Ok(password) => Ok(password),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to delete password".to_string(),
            }),
        )),
    }
}

async fn save_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<PasswordRequest>,
) -> Result<Json<Password>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;

    let _ = app_state
        .events
        .save_event(Event {
            uuid: Uuid::new_v4(),
            user_uuid,
            event_type: EventType::CreatePassword,
            metadata: format!("Password name: {}, IP address: {}", request.name, addr),
            timestamp: Utc::now(),
        })
        .await;

    match app_state
        .passwords
        .save_password(map_password(user_uuid, request))
        .await
    {
        Ok(password) => Ok(Json(password)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to save password".to_string(),
            }),
        )),
    }
}

async fn edit_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(uuid): Path<String>,
    Json(request): Json<PasswordRequest>,
) -> Result<Json<Password>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;
    let password_uuid = Uuid::parse_str(&uuid).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid password UUID".to_string(),
            }),
        )
    })?;

    let owned = app_state
        .passwords
        .owns_password(password_uuid, user_uuid)
        .await;
    if !owned {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Password not found or unauthorized".to_string(),
            }),
        ));
    }

    let _ = app_state
        .events
        .save_event(Event {
            uuid: Uuid::new_v4(),
            user_uuid,
            event_type: EventType::EditPassword,
            metadata: format!("Password name: {}, IP address: {}", request.name, addr),
            timestamp: Utc::now(),
        })
        .await;

    match app_state
        .passwords
        .update_password(map_password(user_uuid, request))
        .await
    {
        Ok(password) => Ok(Json(password)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update password".to_string(),
            }),
        )),
    }
}

async fn remove_password(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(uuid): Path<String>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;
    let password_uuid = Uuid::parse_str(&uuid).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid password UUID format".to_string(),
            }),
        )
    })?;

    let owned = app_state
        .passwords
        .owns_password(password_uuid, user_uuid)
        .await;
    if !owned {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Password not found or unauthorized".to_string(),
            }),
        ));
    }

    let _ = app_state
        .events
        .save_event(Event {
            uuid: Uuid::new_v4(),
            user_uuid,
            event_type: EventType::DeletePassword,
            metadata: format!("IP addres: {}", addr),
            timestamp: Utc::now(),
        })
        .await;

    match app_state.passwords.delete_password(password_uuid).await {
        Ok(_) => Ok(()),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to delete password".to_string(),
            }),
        )),
    }
}
