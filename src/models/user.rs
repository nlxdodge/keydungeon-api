pub struct User {
    pub uuid: String,
    pub email: String,
    password: String
}

impl User {
    pub fn new(email: String) -> User {
        User { email: email, uuid: "213".to_string(), password: "123".to_string()}
    }
}