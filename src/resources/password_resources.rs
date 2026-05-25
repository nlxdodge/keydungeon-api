use crate::{auth::claims::Claims, models::event::EventType};

use crate::models::password::Password;
use crate::resources::event_resource::log_event;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct CreatePasswordRequest {
    pub uuid: Uuid,
    pub icon: String,
    pub url: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordRequest {
    pub icon: String,
    pub url: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/", get(get_passwords))
        .route("/", post(save_password))
        .route("/{uuid}", patch(edit_password))
        .route("/{uuid}", delete(remove_password))
}

/// Get all passwords for the authenticated user
async fn get_passwords(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<Password>>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;

    let passwords = sqlx::query_as::<_, Password>(
        "SELECT uuid, user_uuid, icon, url, name, username, password, timestamp FROM passwords WHERE user_uuid = $1",
    )
    .bind(user_uuid)
    .fetch_all(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to fetch passwords".to_string(),
            }),
        )
    })?;

    Ok(Json(passwords))
}

/// Save a new password for the authenticated user
async fn save_password(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreatePasswordRequest>,
) -> Result<Json<Password>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;

    let password = sqlx::query_as::<_, Password>(
        "INSERT INTO passwords(uuid, user_uuid, icon, url, name, username, password, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
    )
    .bind(payload.uuid)
    .bind(user_uuid)
    .bind(&payload.icon)
    .bind(&payload.url)
    .bind(&payload.name)
    .bind(&payload.username)
    .bind(&payload.password)
    .bind(Local::now())
    .fetch_one(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to save password".to_string(),
            }),
        )
    })?;

    // Log the create event
    let _ = log_event(
        &pool,
        user_uuid,
        EventType::CreatePassword,
        Some(&format!("password_uuid: {}", password.uuid)),
    )
    .await;

    Ok(Json(password))
}

/// Edit an existing password - user can only edit their own passwords
async fn edit_password(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
    Path(uuid): Path<String>,
    Json(payload): Json<UpdatePasswordRequest>,
) -> Result<Json<Password>, (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = claims.sub;
    let password_uuid = Uuid::parse_str(&uuid).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid password UUID format".to_string(),
            }),
        )
    })?;

    // Verify the password belongs to the authenticated user
    let existing = sqlx::query("SELECT uuid FROM passwords WHERE uuid = $1 AND user_uuid = $2")
        .bind(password_uuid)
        .bind(user_uuid)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?;

    if existing.is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Password not found or unauthorized".to_string(),
            }),
        ));
    }

    let password = sqlx::query_as::<_, Password>(
        "UPDATE passwords SET icon = $1, url = $2, name = $3, username = $4, password = $5 WHERE uuid = $6 RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
    )
    .bind(&payload.icon)
    .bind(&payload.url)
    .bind(&payload.name)
    .bind(&payload.username)
    .bind(&payload.password)
    .bind(password_uuid)
    .fetch_one(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update password".to_string(),
            }),
        )
    })?;

    // Log the update event
    let _ = log_event(
        &pool,
        user_uuid,
        EventType::EditPassword,
        Some(&format!("password_uuid: {}", password.uuid)),
    )
    .await;

    Ok(Json(password))
}

async fn remove_password(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
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

    let existing = sqlx::query("SELECT uuid FROM passwords WHERE uuid = $1 AND user_uuid = $2")
        .bind(password_uuid)
        .bind(user_uuid)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?;

    if existing.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Password not found".to_string(),
            }),
        ));
    }

    sqlx::query("DELETE FROM passwords WHERE uuid = $1")
        .bind(password_uuid)
        .execute(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete password".to_string(),
                }),
            )
        })?;

    // Log the delete event
    let _ = log_event(
        &pool,
        user_uuid,
        EventType::DeletePassword,
        Some(&format!("password_uuid: {}", password_uuid)),
    )
    .await;

    Ok(())
}
