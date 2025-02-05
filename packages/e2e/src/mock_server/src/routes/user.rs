// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::http::Status;
use rocket::serde::json::Json;

use crate::services::user::get_users_from_db;
use crate::types::user::User;

// For an async route in Rocket 0.5:
#[get("/users")]
pub async fn users_list() -> Result<Json<Vec<User>>, (Status, String)> {
    let users = get_users_from_db()
        .map_err(|err| (Status::InternalServerError, format!("DB error: {}", err)))?;
    Ok(Json(users))
}

#[derive(FromForm)]
struct CSVUpload<'r> {
    file: TempFile<'r>,
}

#[post("/upload_csv", data = "<csv_form>")]
async fn upload_csv(mut csv_form: Form<CSVUpload<'_>>) -> Result<&'static str, Status> {
    // 1. Save the uploaded file to a temporary location
    let tmp_path = "/tmp/uploaded_voters.csv"; // or generate a random path if you prefer
    if let Err(e) = csv_form.file.persist_to(tmp_path).await {
        eprintln!("Error saving file: {}", e);
        return Err(Status::InternalServerError);
    }

    // 2. Call your existing load_users() using the saved file path
    if let Err(e) = load_users(tmp_path) {
        eprintln!("Failed to load CSV: {}", e);
        return Err(Status::InternalServerError);
    }

    // 3. Return a success response
    Ok("CSV uploaded and processed successfully!")
}
