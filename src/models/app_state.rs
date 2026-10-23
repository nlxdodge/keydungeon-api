use sqlx::SqlitePool;

use crate::repositories::{
    event_repository::EventRepository, password_repository::PasswordRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub events: EventRepository,
    pub passwords: PasswordRepository,
}
