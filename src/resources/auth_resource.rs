use axum::{
    extract::{Extension, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use chrono::Utc;
use sqlx::{Pool, Postgres};
use std::env;
use uuid::Uuid;

use crate::auth::jwt::TokenManager;
use crate::helpers::password;
use crate::models::error::Error;
use crate::models::user::User;
use crate::{
    auth::claims::Claims,
    models::auth::{AuthRequest, AuthResponse},
};

fn get_jwt_valid_time() -> i64 {
    env::var("JWT_VALID_TIME")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24)
}

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/verify", post(verify))
}

async fn login(
    State(pool): State<Pool<Postgres>>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Error>)> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(Error {
                error: "Username and password are required".to_string(),
            }),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "SELECT uuid, username, password, timestamp FROM users WHERE username = $1",
    )
    .bind(&payload.username)
    .fetch_optional(&pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Error {
                error: "Database error".to_string(),
            }),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(Error {
                error: "Invalid username or password".to_string(),
            }),
        )
    })?;

    match password::verify_password(&payload.password, &user.password) {
        Ok(is_valid) => {
            if !is_valid {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(Error {
                        error: "Invalid username or password".to_string(),
                    }),
                ));
            }
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Error {
                    error: "Password verification failed".to_string(),
                }),
            ));
        }
    }

    match generate_new_token(user.uuid) {
        Ok(token) => Ok(Json(token)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(Error { error: e }))),
    }
}

/// Register endpoint - creates a new user and returns a JWT token
async fn register(
    State(pool): State<Pool<Postgres>>,
    Json(payload): Json<AuthRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<Error>)> {
    if payload.username.is_empty() || payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(Error {
                error: "Username required, password must be at least 8 characters".to_string(),
            }),
        ));
    }
    let existing_user = sqlx::query("SELECT uuid FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Error {
                    error: "Database error".to_string(),
                }),
            )
        })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(Error {
                error: "Username already exists".to_string(),
            }),
        ));
    }

    let hashed_password = match password::hash_password(&payload.password) {
        Ok(hash) => hash,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Error {
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
        .execute(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Error {
                    error: "Failed to create user".to_string(),
                }),
            )
        })?;

    match generate_new_token(user_id) {
        Ok(token) => Ok((StatusCode::CREATED, Json(token))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(Error { error: e }))),
    }
}

async fn verify(
    Extension(claims): Extension<Claims>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Error>)> {
    if claims.is_expired() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(Error {
                error: "Token has expired, login again!".to_string(),
            }),
        ));
    }

    match generate_new_token(claims.sub) {
        Ok(token) => Ok(Json(token)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(Error { error: e }))),
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
