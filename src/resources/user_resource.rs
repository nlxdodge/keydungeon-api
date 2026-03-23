use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::{Pool, Postgres};

use crate::models::user::User;

pub fn routing() -> Router<Pool<Postgres>> {
    Router::new().route("/users/:uuid", get(get_user))
}

async fn get_user(Path(uuid): Path<String>, State(pool): State<Pool<Postgres>>) -> Json<Vec<User>> {
    let users = sqlx::query_as::<_, User>("SELECT uuid, email, password FROM users WHERE uuid = ?")
        .bind(uuid)
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch users");
    Json(users)
}
