use sqlx::Error;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::password::Password;

#[derive(Clone)]
pub struct UserRepository {
    pub pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
