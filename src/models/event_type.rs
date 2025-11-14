use serde::Serialize;

#[derive(Serialize)]
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
    DeleteUser
}