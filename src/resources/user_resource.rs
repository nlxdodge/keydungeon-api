use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};

use crate::models::user::User;

pub fn routing(router: Router) -> Router {
   router.route("/users/{uuid}", get(get_user))
}

async fn get_user(Path(uuid): Path<String>) -> Json<Vec<User>> {
    Json(vec![User { uuid: uuid, email: "".to_string(), password: "".to_string()}])
}
