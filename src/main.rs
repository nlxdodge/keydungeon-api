use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::handlers::jwt::TokenManager;
use crate::helpers::vault::Vault;
use crate::models::app_state::AppState;
use crate::repositories::event_repository::EventRepository;
use crate::repositories::password_repository::PasswordRepository;
use crate::repositories::user_repository::UserRepository;

mod api;
mod handlers;
mod helpers;
mod models;
mod repositories;
mod router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    init_tracing();

    warn_on_default_secret("PASSWORD_PEPPER");
    warn_on_default_secret("JWT_SECRET");

    // Fail before touching the database when the vault key is missing or invalid.
    let vault = Vault::from_env()?;

    let port = env::var("PORT").unwrap_or_else(|_| "4321".to_string());
    let pool = get_pool().await?;

    sqlx::raw_sql(include_str!("../init.sql"))
        .execute(&pool)
        .await?;

    let state = AppState {
        db: pool.clone(),
        events: EventRepository::new(pool.clone()),
        passwords: PasswordRepository::new(pool.clone(), vault),
        users: UserRepository::new(pool.clone()),
        jwt: TokenManager::new(),
    };
    let app = router::create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("KeyDungeon API listening on http://0.0.0.0:{port}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn warn_on_default_secret(name: &str) {
    let insecure = match env::var(name) {
        Ok(value) => value.contains("change-me") || value.contains("replace-with"),
        Err(_) => true,
    };

    if insecure {
        tracing::warn!(
            variable = name,
            "environment variable is missing or uses a placeholder, using an insecure development value"
        );
    }
}

async fn get_pool() -> Result<SqlitePool, sqlx::Error> {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://keydungeon.db".to_string());

    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}
