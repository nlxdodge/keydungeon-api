use axum::Router;
use log::{info, error};

use crate::resources::{password_resources, user_resource};

mod models;
mod resources;

const PORT:i32 = 3000;

#[tokio::main]
async fn main() {
    let mut router = Router::new();
    router = user_resource::routing(router);
    router = password_resources::routing(router);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{PORT}")).await;
    if listener.is_ok() {
        info!("Started server on port {PORT}");
        axum::serve(listener.unwrap(), router).await.unwrap();
    } else {
        error!("Error while listening to port {PORT}");
    }
}
