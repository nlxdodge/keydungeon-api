use axum::{
    Json, Router,
    extract::{Extension, State},
    http::StatusCode,
    routing::post,
};
use chrono::Utc;
use std::env;
use uuid::Uuid;

use crate::models::user::User;
use crate::{
    handlers::claims::Claims,
    models::auth::{AuthRequest, AuthResponse},
};
use crate::{handlers::jwt::TokenManager, models::app_state::AppState};
use crate::{helpers::password, models::error_response::ErrorResponse};

fn get_jwt_valid_time() -> i64 {
    env::var("JWT_VALID_TIME")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24)
}

pub fn routing() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/verify", post(verify))
}

async fn login(
    State(app_state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Username and password are required".to_string(),
            }),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "SELECT uuid, username, password, timestamp FROM users WHERE username = $1",
    )
    .bind(&payload.username)
    .fetch_optional(&app_state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Database error".to_string(),
            }),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid username or password".to_string(),
            }),
        )
    })?;

    match password::verify_password(&payload.password, &user.password) {
        Ok(is_valid) => {
            if !is_valid {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid username or password".to_string(),
                    }),
                ));
            }
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Password verification failed".to_string(),
                }),
            ));
        }
    }

    match generate_new_token(user.uuid) {
        Ok(token) => Ok(Json(token)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

/// Register endpoint - creates a new user and returns a JWT token
async fn register(
    State(app_state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    if payload.username.is_empty() || payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Username required, password must be at least 8 characters".to_string(),
            }),
        ));
    }
    let existing_user = sqlx::query("SELECT uuid FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&app_state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Username already exists".to_string(),
            }),
        ));
    }

    let hashed_password = match password::hash_password(&payload.password) {
        Ok(hash) => hash,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to hash password".to_string(),
                }),
            ));
        }
    };

    let user_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    sqlx::query("INSERT INTO users (uuid, username, password, timestamp) VALUES ($1, $2, $3, $4)")
        .bind(user_id)
        .bind(&payload.username)
        .bind(&hashed_password)
        .bind(now)
        .execute(&app_state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create user".to_string(),
                }),
            )
        })?;

    match generate_new_token(user_id) {
        Ok(token) => Ok((StatusCode::CREATED, Json(token))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

async fn verify(
    Extension(claims): Extension<Claims>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    if claims.is_expired() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Token has expired, login again!".to_string(),
            }),
        ));
    }

    match generate_new_token(claims.sub) {
        Ok(token) => Ok(Json(token)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )),
    }
}

fn generate_new_token(user_uuid: Uuid) -> Result<AuthResponse, String> {
    let token_manager = TokenManager::new();
    let valid_time = get_jwt_valid_time();

    match token_manager.generate_token_with_duration(user_uuid, valid_time) {
        Ok(token) => {
            let claims = Claims::with_duration(user_uuid, valid_time);
            Ok(AuthResponse { token, claims })
        }
        Err(_) => Err("Failed to generate token".to_string()),
    }
}
