use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Health {
    pub endpoints: bool,
    pub database: bool,
}
