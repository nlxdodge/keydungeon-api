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
        .route("/passwords/:uuid", get(get_passwords))
        .route("/passwords/:user_uuid", post(save_password))
        .route("/passwords/:uuid", patch(edit_password))
        .route("/passwords/:uuid", delete(remove_password))
}

async fn get_passwords(
    State(pool): State<Pool<Postgres>>,
    Path(uuid): Path<String>,
) -> Json<Vec<Password>> {
    let passwords = sqlx::query_as::<_, Password>(
        "SELECT uuid, icon, url, name, username, password FROM passwords WHERE uuid = ?",
    )
    .bind(uuid)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch users");
    Json(passwords)
}

async fn save_password(
    State(pool): State<Pool<Postgres>>,
    Path(user_uuid): Path<String>,
    Json(payload): Json<Password>,
) -> Json<Vec<Password>> {
    let passwords = sqlx::query_as::<_, Password>(
        "insert into passwords(uuid=?, user_uuid=?, icon=?, url=?, name=?, username=?, password=?, timestamp=?)",
    )
    .bind(&payload.uuid)
    .bind(&user_uuid)
    .bind(&payload.icon)
    .bind(&payload.url)
    .bind(&payload.name)
    .bind(&payload.username)
    .bind(&payload.password)
    .bind(Local::now())
    .fetch_all(&pool)
    .await
    .expect("Failed to save new password");
    Json(passwords)
}

async fn edit_password() {}

async fn remove_password() {}
