// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::user::{get_users_from_db, load_users};
use crate::types::user::User;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;

// For an async route in Rocket 0.5:
#[get("/users")]
pub async fn users_list() -> Result<Json<Vec<User>>, (Status, String)> {
    let users = get_users_from_db()
        .map_err(|err| (Status::InternalServerError, format!("DB error: {}", err)))?;
    Ok(Json(users))
}

#[derive(FromForm)]
pub struct CSVUpload<'r> {
    file: TempFile<'r>,
}

#[post("/upload-csv", data = "<csv_form>")]
pub async fn upload_csv(mut csv_form: Form<CSVUpload<'_>>) -> Result<String, Status> {
    let tmp_path = "/tmp/uploaded_voters.csv";
    if let Err(e) = csv_form.file.persist_to(tmp_path).await {
        eprintln!("Error saving file: {}", e);
        return Err(Status::InternalServerError);
    }
    info!("CSV file saved to: {}", tmp_path);
    let num_of_users = match load_users(tmp_path) {
        Ok(usize) => usize,
        Err(e) => {
            eprintln!("Failed to load CSV: {}", e);
            return Err(Status::InternalServerError);
        }
    };

    Ok(format!("Loaded {} users successfully", num_of_users))
}
