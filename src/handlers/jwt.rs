use axum::{
    extract::Request,
    http::{StatusCode, header::HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::handlers::claims::Claims;

/// Load JWT secret from environment variable or use default
fn get_jwt_secret() -> Vec<u8> {
    dotenvy::var("JWT_SECRET")
        .unwrap_or_else(|_| {
            log::warn!(
                "JWT_SECRET not found in .env file, using default. Change this in production!"
            );
            "your-secret-key-change-in-production".to_string()
        })
        .into_bytes()
}

/// Token manager for creating and validating JWT tokens
#[derive(Clone)]
pub struct TokenManager {
    secret: Vec<u8>,
}

impl TokenManager {
    /// Create a new TokenManager with the default secret (loaded from JWT_SECRET env var)
    pub fn new() -> Self {
        Self {
            secret: get_jwt_secret(),
        }
    }

    /// Generate a JWT token with a custom expiration duration
    pub fn generate_token_with_duration(
        &self,
        user_id: Uuid,
        hours: i64,
    ) -> Result<String, TokenError> {
        let claims = Claims::with_duration(user_id, hours);
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|_| TokenError::TokenGenerationFailed)?;

        Ok(token)
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims, TokenError> {
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )
        .map_err(|_| TokenError::InvalidToken)?;

        if data.claims.is_expired() {
            return Err(TokenError::TokenExpired);
        }

        Ok(data.claims)
    }

    /// Extract token from Authorization header (Bearer scheme)
    pub fn extract_token_from_header(&self, headers: &HeaderMap) -> Result<String, TokenError> {
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(TokenError::MissingAuthHeader)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(TokenError::InvalidAuthHeader);
        }

        Ok(auth_header[7..].to_string())
    }
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}

/// JWT Token Errors
#[derive(Debug, Serialize, Deserialize)]
pub enum TokenError {
    MissingAuthHeader,
    InvalidAuthHeader,
    InvalidToken,
    TokenExpired,
    TokenGenerationFailed,
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            TokenError::MissingAuthHeader => {
                (StatusCode::UNAUTHORIZED, "Missing authorization header")
            }
            TokenError::InvalidAuthHeader => (
                StatusCode::UNAUTHORIZED,
                "Invalid authorization header format",
            ),
            TokenError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            TokenError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token has expired"),
            TokenError::TokenGenerationFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate token",
            ),
        };

        let body = serde_json::json!({
            "error": error_message,
        });

        (status, axum::Json(body)).into_response()
    }
}

/// JWT Authentication Middleware
pub async fn jwt_middleware(mut request: Request, next: Next) -> Result<Response, TokenError> {
    let token_manager = TokenManager::new();

    // Extract token from header
    let token = token_manager.extract_token_from_header(request.headers())?;

    // Validate token
    let claims = token_manager.validate_token(&token)?;

    // Insert claims into request extensions for downstream handlers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
