use sqlx::{Error as SqlxError, SqlitePool};
use uuid::Uuid;

use crate::helpers::vault::{Vault, VaultError};
use crate::models::password::Password;

#[derive(Debug)]
pub enum PasswordRepositoryError {
    Database(SqlxError),
    Vault(VaultError),
}

impl std::fmt::Display for PasswordRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordRepositoryError::Database(error) => write!(f, "Database error: {error}"),
            PasswordRepositoryError::Vault(error) => write!(f, "Vault error: {error}"),
        }
    }
}

impl std::error::Error for PasswordRepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PasswordRepositoryError::Database(error) => Some(error),
            PasswordRepositoryError::Vault(error) => Some(error),
        }
    }
}

impl From<SqlxError> for PasswordRepositoryError {
    fn from(error: SqlxError) -> Self {
        PasswordRepositoryError::Database(error)
    }
}

impl From<VaultError> for PasswordRepositoryError {
    fn from(error: VaultError) -> Self {
        PasswordRepositoryError::Vault(error)
    }
}

/// Owns every password-vault read and write. Values are encrypted with
/// ChaCha20-Poly1305 before they touch SQLite and decrypted before they are
/// returned to a handler.
#[derive(Clone)]
pub struct PasswordRepository {
    pub pool: SqlitePool,
    vault: Vault,
}

impl PasswordRepository {
    pub fn new(pool: SqlitePool, vault: Vault) -> Self {
        Self { pool, vault }
    }

    pub async fn get_passwords(
        &self,
        user_uuid: Uuid,
        limit: u32,
        offset: i64,
        search: Option<&str>,
    ) -> Result<Vec<Password>, PasswordRepositoryError> {
        let mut passwords = sqlx::query_as::<_, Password>(
            "SELECT uuid, user_uuid, icon, url, name, username, password, timestamp \
             FROM passwords \
             WHERE user_uuid = $1 AND ($2 IS NULL OR name LIKE $2 ESCAPE '\\') \
             ORDER BY timestamp DESC LIMIT $3 OFFSET $4",
        )
        .bind(user_uuid)
        .bind(search.map(like_pattern))
        .bind(i64::from(limit))
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        for password in &mut passwords {
            password.password = self.vault.decrypt(&password.password)?.to_string();
        }

        Ok(passwords)
    }

    pub async fn count_passwords(
        &self,
        user_uuid: Uuid,
        search: Option<&str>,
    ) -> Result<u64, PasswordRepositoryError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM passwords \
             WHERE user_uuid = $1 AND ($2 IS NULL OR name LIKE $2 ESCAPE '\\')",
        )
        .bind(user_uuid)
        .bind(search.map(like_pattern))
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    pub async fn get_password(
        &self,
        user_uuid: Uuid,
        password_uuid: Uuid,
    ) -> Result<Option<String>, PasswordRepositoryError> {
        let encrypted = sqlx::query_scalar::<_, String>(
            "SELECT password FROM passwords WHERE uuid = $1 AND user_uuid = $2 LIMIT 1",
        )
        .bind(password_uuid)
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await?;

        match encrypted {
            Some(encrypted) => Ok(Some(self.vault.decrypt(&encrypted)?.to_string())),
            None => Ok(None),
        }
    }

    pub async fn save_password(
        &self,
        password: Password,
    ) -> Result<Password, PasswordRepositoryError> {
        let encrypted = self.vault.encrypt(&password.password)?;
        let saved = sqlx::query_as::<_, Password>(
            "INSERT INTO passwords(uuid, user_uuid, icon, url, name, username, password, timestamp) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
        )
        .bind(password.uuid)
        .bind(password.user_uuid)
        .bind(&password.icon)
        .bind(&password.url)
        .bind(&password.name)
        .bind(&password.username)
        .bind(encrypted)
        .bind(password.timestamp)
        .fetch_one(&self.pool)
        .await?;

        self.decrypt_password(saved)
    }

    pub async fn update_password(
        &self,
        user_uuid: Uuid,
        password: Password,
    ) -> Result<Option<Password>, PasswordRepositoryError> {
        let encrypted = self.vault.encrypt(&password.password)?;
        let updated = sqlx::query_as::<_, Password>(
            "UPDATE passwords SET icon = $1, url = $2, name = $3, username = $4, password = $5 \
             WHERE uuid = $6 AND user_uuid = $7 \
             RETURNING uuid, user_uuid, icon, url, name, username, password, timestamp",
        )
        .bind(&password.icon)
        .bind(&password.url)
        .bind(&password.name)
        .bind(&password.username)
        .bind(encrypted)
        .bind(password.uuid)
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await?;

        match updated {
            Some(password) => Ok(Some(self.decrypt_password(password)?)),
            None => Ok(None),
        }
    }

    pub async fn delete_password(
        &self,
        user_uuid: Uuid,
        password_uuid: Uuid,
    ) -> Result<bool, PasswordRepositoryError> {
        let deleted = sqlx::query("DELETE FROM passwords WHERE uuid = $1 AND user_uuid = $2")
            .bind(password_uuid)
            .bind(user_uuid)
            .execute(&self.pool)
            .await?
            .rows_affected();

        Ok(deleted > 0)
    }

    fn decrypt_password(
        &self,
        mut password: Password,
    ) -> Result<Password, PasswordRepositoryError> {
        password.password = self.vault.decrypt(&password.password)?.to_string();
        Ok(password)
    }
}

/// Build a `%term%` pattern, escaping SQL `LIKE` wildcards so a search for
/// `100%` or `a_b` is matched literally.
fn like_pattern(search: &str) -> String {
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");

    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn wraps_a_plain_search_term() {
        assert_eq!(like_pattern("gmail"), "%gmail%");
    }

    #[test]
    fn escapes_like_wildcards() {
        assert_eq!(like_pattern("100%"), "%100\\%%");
        assert_eq!(like_pattern("a_b"), "%a\\_b%");
        assert_eq!(like_pattern("back\\slash"), "%back\\\\slash%");
    }
}
