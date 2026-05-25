use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{delete, get, patch},
    Json, Router,
};
use sqlx::{Pool, Postgres};

use crate::auth::claims::Claims;
use crate::helpers::password;
use crate::models::user::User;

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/{uuid}", get(get_user))
        .route("/", patch(update_user))
        .route("/{uuid}", delete(remove_user))
}

async fn get_user(
    State(pool): State<Pool<Postgres>>,
    Path(uuid): Path<String>,
    Extension(claims): Extension<Claims>,
) -> Json<Vec<User>> {
    let _authenticated_user_id = claims.sub;

    let users = sqlx::query_as::<_, User>(
        "SELECT uuid, username, password, timestamp FROM users WHERE uuid = $1",
    )
    .bind(uuid)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch user");

    Json(users)
}

async fn update_user(
    State(pool): State<Pool<Postgres>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<User>,
) -> Result<Json<User>, (StatusCode, Json<ErrorResponse>)> {
    let _authenticated_user_id = claims.sub;

    // Hash the new password using bcrypt
    let hashed_password = password::hash_password(&payload.password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to hash password".to_string(),
            }),
        )
    })?;

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET username = $2, password = $3 WHERE uuid = $1 RETURNING uuid, username, password, timestamp"
    )
        .bind(payload.uuid)
        .bind(payload.username)
        .bind(&hashed_password)
        .fetch_one(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update user".to_string(),
                }),
            )
        })?;

    Ok(Json(user))
}

async fn remove_user(
    State(pool): State<Pool<Postgres>>,
    Path(request_uuid): Path<String>,
    Extension(claims): Extension<Claims>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let authenticated_user_id = claims.sub;

    check_correct_user_id(&authenticated_user_id.to_string(), &request_uuid)?;

    sqlx::query("DELETE FROM users WHERE uuid=$1")
        .bind(request_uuid)
        .execute(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete user".to_string(),
                }),
            )
        })?;

    Ok(())
}

fn check_correct_user_id(
    authenticated_user_id: &str,
    request_uuid: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if request_uuid != authenticated_user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Unauthorized: you can only delete your own account".to_string(),
            }),
        ));
    }
    Ok(())
}
