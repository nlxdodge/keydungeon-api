use serde::Serialize;
use secrecy::SecretString;

#[derive(Serialize)]
pub struct Password {
    pub uuid: String,
    pub icon: String,
    pub url: String,
    pub name: String,
    pub password: SecretString
}