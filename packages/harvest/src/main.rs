// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use sequent_core::util::init_log::init_log;
use dotenv::dotenv;

mod pdf;
mod routes;
mod s3;
mod services;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    init_log(true);
    rocket::build()
        .register(
            "/",
            catchers![
                routes::error_catchers::internal_error,
                routes::error_catchers::not_found,
                routes::error_catchers::default
            ],
        )
        .mount(
            "/",
            routes![
                routes::fetch_document::fetch_document,
                routes::scheduled_event::create_scheduled_event,
                routes::immudb_log_audit::list_pgaudit,
            ],
        )
}
