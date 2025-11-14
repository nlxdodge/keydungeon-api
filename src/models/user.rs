use serde::Serialize;

#[derive(Serialize)]
pub struct User {
    pub uuid: String,
    pub email: String,
    pub password: String
}

impl User {
    pub fn new(email: String) -> User {
        User { uuid: "213".to_string(), email: email, password: "123".to_string()}
    }
}