use sqlx::SqlitePool;

use crate::handlers::jwt::TokenManager;
use crate::repositories::{
    event_repository::EventRepository, password_repository::PasswordRepository,
    user_repository::UserRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub events: EventRepository,
    pub passwords: PasswordRepository,
    pub users: UserRepository,
    pub jwt: TokenManager,
}
