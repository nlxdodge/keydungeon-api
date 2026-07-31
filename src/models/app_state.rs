use sqlx::PgPool;

use crate::repositories::{
    event_repository::EventRepository, password_repository::PasswordRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub events: EventRepository,
    pub passwords: PasswordRepository,
}
