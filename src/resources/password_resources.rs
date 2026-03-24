use crate::models::password::Password;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, patch, post},
};
use chrono::Local;
use sqlx::{Pool, Postgres};

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/:uuid", get(get_passwords))
        .route("/:user_uuid", post(save_password))
        .route("/:uuid", patch(edit_password))
        .route("/:uuid", delete(remove_password))
}

async fn get_passwords(
    State(pool): State<Pool<Postgres>>,
    Path(uuid): Path<String>,
) -> Json<Vec<Password>> {
    let passwords = sqlx::query_as::<_, Password>(
        "SELECT uuid, user_uuid, icon, url, name, username, password, timestamp FROM passwords WHERE user_uuid = $1",
    )
    .bind(uuid)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch passwords");
    Json(passwords)
}

async fn save_password(
    State(pool): State<Pool<Postgres>>,
    Path(user_uuid): Path<String>,
    Json(payload): Json<Password>,
) -> Json<Password> {
    let password = sqlx::query_as::<_, Password>(
        "INSERT INTO passwords(uuid, user_uuid, icon, url, name, username, password, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
    )
    .bind(&payload.uuid)
    .bind(&user_uuid)
    .bind(&payload.icon)
    .bind(&payload.url)
    .bind(&payload.name)
    .bind(&payload.username)
    .bind(&payload.password)
    .bind(Local::now())
    .fetch_one(&pool)
    .await
    .expect("Failed to save new password");
    Json(password)
}

async fn edit_password(
    State(pool): State<Pool<Postgres>>,
    Path(uuid): Path<String>,
    Json(payload): Json<Password>,
) -> Json<Password> {
    let password = sqlx::query_as::<_, Password>(
        "UPDATE passwords SET icon = $1, url = $2, name = $3, username = $4, password = $5 WHERE uuid = $6 RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
    )
    .bind(&payload.icon)
    .bind(&payload.url)
    .bind(&payload.name)
    .bind(&payload.username)
    .bind(&payload.password)
    .bind(&uuid)
    .fetch_one(&pool)
    .await
    .expect("Failed to update password");
    Json(password)
}

async fn remove_password(State(pool): State<Pool<Postgres>>, Path(uuid): Path<String>) {
    sqlx::query_as::<_, Password>("DELETE FROM passwords WHERE uuid = $1")
        .bind(&uuid)
        .fetch_one(&pool)
        .await
        .expect("Failed to update password");
}
