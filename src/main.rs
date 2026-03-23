use axum::Router;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;

use crate::resources::{password_resources, user_resource};

mod models;
mod resources;

const PORT: i32 = 3000;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:password@localhost/test")
        .await?;

    let app = Router::new()
        .nest("/users", user_resource::routing())
        .nest("/passwords", password_resources::routing());

    let shared_pool = pool.clone();
    let app = app.with_state(shared_pool);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{PORT}")).await;

    if listener.is_ok() {
        info!("Started server on port {PORT}");
        axum::serve(listener.unwrap(), app).await.unwrap();
    } else {
        error!("Error while listening to port {PORT}");
    }

    Ok(())
}
