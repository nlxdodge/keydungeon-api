use axum::middleware;
use axum::Router;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::env;

use crate::auth::jwt::jwt_middleware;
use crate::resources::{auth_resource, event_resource, password_resources, user_resource};

mod auth;
mod config;
mod helpers;
mod models;
mod resources;

const PORT: i32 = 4321;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    env_logger::init();

    // Get database URL from environment variables
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/test".to_string());

    info!("Connecting to database: {}", database_url);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let shared_pool = pool.clone();

    // Public routes - no JWT middleware required
    let public_routes = Router::new().nest("/auth", auth_resource::routing());

    // Protected routes - JWT middleware required
    let protected_routes = Router::new()
        .nest("/users", user_resource::routing())
        .nest("/passwords", password_resources::routing())
        .nest("/events", event_resource::routing())
        .layer(middleware::from_fn(jwt_middleware));

    // Combine public and protected routes
    let app = public_routes
        .merge(protected_routes)
        .with_state(shared_pool);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{PORT}")).await;

    if listener.is_ok() {
        info!("Started server on port {PORT}");
        axum::serve(listener.unwrap(), app).await.unwrap();
    } else {
        error!("Error while listening to port {PORT}");
    }

    Ok(())
}
