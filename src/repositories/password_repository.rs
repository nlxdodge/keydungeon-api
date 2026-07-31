use sqlx::Error;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::password::Password;

#[derive(Clone)]
pub struct PasswordRepository {
    pub pool: PgPool,
}

impl PasswordRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_passwords(&self, user_uuid: Uuid) -> Result<Vec<Password>, Error> {
        sqlx::query_as::<_, Password>(
            "SELECT uuid, user_uuid, icon, url, name, username, password, timestamp FROM passwords WHERE user_uuid = $1",
        )
        .bind(user_uuid)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_password(
        &self,
        user_uuid: Uuid,
        password_uuid: Uuid,
    ) -> Result<String, Error> {
        sqlx::query_scalar::<_, String>(
            "SELECT password FROM passwords WHERE uuid = $1 AND user_uuid = $2 LIMIT 1",
        )
        .bind(password_uuid)
        .bind(user_uuid)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn save_password(&self, password: Password) -> Result<Password, Error> {
        sqlx::query_as::<_, Password>(
        "INSERT INTO passwords(uuid, user_uuid, icon, url, name, username, password, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
        )
        .bind(&password.uuid)
        .bind(&password.user_uuid)
        .bind(&password.icon)
        .bind(&password.url)
        .bind(&password.name)
        .bind(&password.username)
        .bind(&password.password)
        .bind(&password.timestamp)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn owns_password(&self, password_uuid: Uuid, user_uuid: Uuid) -> bool {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM passwords WHERE uuid = $1 AND user_uuid = $2 LIMIT 1",
        )
        .bind(password_uuid)
        .bind(user_uuid)
        .fetch_one(&self.pool)
        .await;
        result.is_ok() && !result.unwrap_or(String::new()).is_empty()
    }

    pub async fn update_password(&self, password: Password) -> Result<Password, Error> {
        sqlx::query_as::<_, Password>(
        "UPDATE passwords SET icon = $1, url = $2, name = $3, username = $4, password = $5 WHERE uuid = $6 RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
        )
        .bind(&password.icon)
        .bind(&password.url)
        .bind(&password.name)
        .bind(&password.username)
        .bind(&password.password)
        .bind(&password.uuid)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete_password(&self, password_uuid: Uuid) -> Result<bool, Error> {
        let deleted = sqlx::query("DELETE FROM passwords WHERE uuid = $1")
            .bind(password_uuid)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(deleted > 0)
    }
}
