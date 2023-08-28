// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use dotenv::dotenv;

mod connection;
mod hasura;
mod pdf;
mod routes;
mod s3;
mod services;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    rocket::build().mount(
        "/",
        routes![
            routes::fetch_document::fetch_document,
            routes::render_report::render_report,
            routes::scheduled_event::create_scheduled_event,
        ],
    )
}
