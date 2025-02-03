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

// If you have a CSV load at startup or anything, you can do it in main or a fairing

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // For example, if you want to load CSV before launching, do it here:
    // load_csv_into_db("voters.csv").expect("Failed to load CSV");

    if let Err(e) = load_users("./voters.csv") {
        eprintln!("Failed to load CSV: {:?}", e);
        // Exit early if CSV loading is critical and you want to fail fast.
        std::process::exit(1);
    }

    let _rocket = rocket::build()
        .mount(
            "/",
            routes![
                index,
                routes::user::users_list,
                routes::inetum::transaction_new,
                routes::inetum::transaction_status_simple,
                routes::inetum::transaction_results,
            ],
        )
        .launch()
        .await?;

    Ok(())
}
