use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, patch, post},
};
use sqlx::{Pool, Postgres};

use crate::models::user::User;

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new()
        .route("/:uuid", get(get_user))
        .route("/", post(save_user))
        .route("/", patch(update_user))
        .route("/:uuid", delete(remove_user))
}

async fn get_user(State(pool): State<Pool<Postgres>>, Path(uuid): Path<String>) -> Json<Vec<User>> {
    let users =
        sqlx::query_as::<_, User>("SELECT uuid, email, password FROM users WHERE uuid = $1")
            .bind(uuid)
            .fetch_all(&pool)
            .await
            .expect("Failed to fetch user");
    Json(users)
}

async fn save_user(State(pool): State<Pool<Postgres>>, Json(payload): Json<User>) -> Json<User> {
    let user =
        sqlx::query_as::<_, User>("INSERT INTO users(uuid, username, password, timestamp) VALUES ($1, $2, $3, $4) RETURNING uuid, username, password, timestamp")
            .bind(payload.uuid)
            .bind(payload.uuid)
            .fetch_one(&pool)
            .await
            .expect("Failed to save user");
    Json(user)
}

async fn update_user(State(pool): State<Pool<Postgres>>, Json(payload): Json<User>) -> Json<User> {
    let user =
        sqlx::query_as::<_, User>("INSERT INTO users(uuid, username, password, timestamp) VALUES ($1, $2, $3, $4) RETURNING uuid, username, password, timestamp")
            .bind(payload.uuid)
            .bind(payload.username)
            .bind(payload.password)
            .fetch_one(&pool)
            .await
            .expect("Failed to save user");
    Json(user)
}

async fn remove_user(State(pool): State<Pool<Postgres>>, Path(uuid): Path<String>) {
    sqlx::query_as::<_, User>("DELETE FROM users WHERE uuid=$1")
        .bind(uuid)
        .fetch_one(&pool)
        .await
        .expect("Failed to save user");
}
