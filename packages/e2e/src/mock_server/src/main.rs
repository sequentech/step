// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

mod routes;
mod services;
mod types;

use rocket::data::Limits;
use services::user::load_users;

// Import your routes

#[get("/")]
fn index() -> &'static str {
    "Server is running!"
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let figment = rocket::Config::figment().merge((
        "limits",
        Limits::new().limit("file", rocket::data::ByteUnit::Megabyte(600)),
    ));

    let _rocket = rocket::custom(figment)
        .manage(routes::scanovate::MockSessions::default())
        .mount(
            "/",
            routes![
                index,
                routes::user::users_list,
                routes::scanovate::auth_token,
                routes::scanovate::flow_link,
                routes::scanovate::flow_page,
                routes::scanovate::flow_complete,
                routes::scanovate::session_token,
                routes::scanovate::results_with_image_names,
                routes::user::upload_csv,
            ],
        )
        .launch()
        .await?;

    Ok(())
}
