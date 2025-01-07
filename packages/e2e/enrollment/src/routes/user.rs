
use rocket::http::Status;
use rocket::serde::json::Json;

use crate::services::user::get_users_from_db;
use crate::types::user::User;


// For an async route in Rocket 0.5:
#[get("/users")]
pub async fn users_list() -> Result<Json<Vec<User>>, (Status, String)> {
    let users = get_users_from_db().map_err(|err| {
        (Status::InternalServerError, format!("DB error: {}", err))
    })?;
    Ok(Json(users))
}
