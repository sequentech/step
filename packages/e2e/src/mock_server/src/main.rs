// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
        .mount(
            "/",
            routes![
                index,
                routes::user::users_list,
                routes::inetum::transaction_new,
                routes::inetum::transaction_status_simple,
                routes::inetum::transaction_results,
                routes::user::upload_csv,
            ],
        )
        .launch()
        .await?;

    Ok(())
}
