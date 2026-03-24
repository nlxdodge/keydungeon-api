use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Serialize, Deserialize, Type)]
#[sqlx(type_name = "TEXT")]
pub enum EventType {
    SignIn,
    SignOut,
    CreatePassword,
    ShowPassword,
    EditPassword,
    DeletePassword,
    CreateUser,
    ShowUser,
    EditUser,
    DeleteUser,
}
