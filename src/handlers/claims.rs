use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn with_duration(user_id: Uuid, hours: i64) -> Self {
        let now = Utc::now();
        let exp = (now + Duration::hours(hours)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        Self {
            sub: user_id,
            exp,
            iat,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp() as usize;
        self.exp < now
    }
}
