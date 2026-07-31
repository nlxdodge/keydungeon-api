use sqlx::Error;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::password::Password;

#[derive(Clone)]
pub struct UserRepository {
    pub pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
