use log::debug;
use log::info;
use sqlx::Error;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;

use crate::models::app_state::AppState;
use crate::repositories::event_repository::EventRepository;
use crate::repositories::password_repository::PasswordRepository;

mod api;
mod bcrypt_config;
mod handlers;
mod helpers;
mod models;
mod repositories;
mod router;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let port = env::var("PORT").unwrap_or_else(|_| "4321".to_string());
    let pool = get_pool().await?;
    let state: AppState = AppState {
        db: pool.clone(),
        events: EventRepository { pool: pool.clone() },
        passwords: PasswordRepository { pool: pool.clone() },
    };
    let app = router::create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!("Started KeyDungeon api on port {port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn get_pool() -> Result<Pool<Postgres>, Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/test".to_string());

    debug!("Connecting to database: {}", database_url);

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}
