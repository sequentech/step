// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

mod routes;
mod services;
mod types;

use services::user::load_users;

// Import your routes

#[get("/")]
fn index() -> &'static str {
    "Server is running!"
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket::build()
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
