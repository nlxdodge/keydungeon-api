use axum::routing::get;
use axum::{Json, Router};
use secrecy::SecretString;

use crate::models::password::Password;

pub fn routing(router: Router) -> Router {
    return router.route(
        "/passwords",
        get(get_passwords)
            .post(save_password)
            .put(edit_password)
            .delete(remove_password),
    );
}

async fn get_passwords() -> Json<Vec<Password>> {
    Json(vec![Password {
        uuid: "test".to_string(),
        url: "".to_string(),
        icon: "".to_string(),
        name: "".to_string(),
        password: SecretString::new("test".to_owned()),
    }])
}

async fn save_password() {}

async fn edit_password() {}

async fn remove_password() {}
