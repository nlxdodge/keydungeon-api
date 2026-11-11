use chrono::Utc;
use sqlx::{Error as SqlxError, SqlitePool};
use uuid::Uuid;

use crate::helpers::password::{self, PasswordError};
use crate::models::user::User;

#[derive(Debug)]
pub enum UserRepositoryError {
    Database(SqlxError),
    Password(PasswordError),
    UsernameTaken,
}

impl std::fmt::Display for UserRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRepositoryError::Database(error) => write!(f, "Database error: {error}"),
            UserRepositoryError::Password(error) => write!(f, "Password error: {error}"),
            UserRepositoryError::UsernameTaken => write!(f, "Username already exists"),
        }
    }
}

impl std::error::Error for UserRepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UserRepositoryError::Database(error) => Some(error),
            UserRepositoryError::Password(error) => Some(error),
            UserRepositoryError::UsernameTaken => None,
        }
    }
}

impl From<SqlxError> for UserRepositoryError {
    fn from(error: SqlxError) -> Self {
        UserRepositoryError::Database(error)
    }
}

impl From<PasswordError> for UserRepositoryError {
    fn from(error: PasswordError) -> Self {
        UserRepositoryError::Password(error)
    }
}

/// Owns all user account mutations, including signup and global password
/// changes. Password hashing happens here so callers never handle raw hashes.
#[derive(Clone)]
pub struct UserRepository {
    pub pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, UserRepositoryError> {
        sqlx::query_as::<_, User>(
            "SELECT uuid, username, password, timestamp FROM users WHERE uuid = $1 LIMIT 1",
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        sqlx::query_as::<_, User>(
            "SELECT uuid, username, password, timestamp FROM users WHERE username = $1 LIMIT 1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool, UserRepositoryError> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE username = $1")
            .bind(username)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    /// Create a new account. The Argon2 hash is generated on the blocking
    /// thread pool.
    pub async fn create(
        &self,
        username: &str,
        password: String,
    ) -> Result<User, UserRepositoryError> {
        let hashed_password = password::hash_async(password).await?;
        let user = User {
            uuid: Uuid::new_v4(),
            username: username.to_string(),
            password: hashed_password,
            timestamp: Utc::now().naive_utc(),
        };

        sqlx::query_as::<_, User>(
            "INSERT INTO users (uuid, username, password, timestamp) VALUES ($1, $2, $3, $4) \
             RETURNING uuid, username, password, timestamp",
        )
        .bind(user.uuid)
        .bind(&user.username)
        .bind(&user.password)
        .bind(user.timestamp)
        .fetch_one(&self.pool)
        .await
        .map_err(map_user_write_error)
    }

    /// Update a user's username and/or global master password.
    ///
    /// `None` leaves the existing value untouched.
    pub async fn update(
        &self,
        uuid: Uuid,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Option<User>, UserRepositoryError> {
        let Some(current) = self.find_by_uuid(uuid).await? else {
            return Ok(None);
        };

        let username = username.unwrap_or(current.username);
        let password = match password {
            Some(password) => password::hash_async(password).await?,
            None => current.password,
        };

        sqlx::query_as::<_, User>(
            "UPDATE users SET username = $2, password = $3 WHERE uuid = $1 \
             RETURNING uuid, username, password, timestamp",
        )
        .bind(uuid)
        .bind(&username)
        .bind(&password)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_user_write_error)
    }

    pub async fn delete(&self, uuid: Uuid) -> Result<bool, UserRepositoryError> {
        let deleted = sqlx::query("DELETE FROM users WHERE uuid = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await?
            .rows_affected();

        Ok(deleted > 0)
    }
}

fn map_user_write_error(error: SqlxError) -> UserRepositoryError {
    match error {
        SqlxError::Database(database_error) if database_error.is_unique_violation() => {
            UserRepositoryError::UsernameTaken
        }
        other => UserRepositoryError::Database(other),
    }
}
