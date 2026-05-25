use log::info;
use std::env;

#[derive(Debug, Clone)]
pub struct BcryptConfig {
    pub cost: u32,
}

impl BcryptConfig {
    /// Load bcrypt configuration from environment variables
    pub fn from_env() -> Self {
        let cost = env::var("BCRYPT_COST")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_else(|| {
                info!("BCRYPT_COST not set or invalid, using default: 12");
                12
            });

        // Validate cost is within acceptable range
        let cost = if cost < 4 {
            info!("BCRYPT_COST too low ({}), using minimum: 4", cost);
            4
        } else if cost > 31 {
            info!("BCRYPT_COST too high ({}), using maximum: 31", cost);
            31
        } else {
            cost
        };

        info!("Bcrypt cost configured: {}", cost);

        BcryptConfig { cost }
    }

    /// Get the default cost
    #[allow(dead_code)]
    pub fn default_cost() -> u32 {
        12
    }
}
