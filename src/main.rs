mod models;

use models::user::User; 

fn main() {
    let user = User::new("test".to_string());


    println!("Hello {}!", user.email);

}
