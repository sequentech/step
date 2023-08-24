// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use dotenv::dotenv;
use either::*;
use handlebars::Handlebars;
use headless_chrome::types::PrintToPdfOptions;
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;

mod connection;
mod hasura;
mod pdf;
mod s3;
mod routes;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    rocket::build().mount("/", routes![
        routes::fetch_document::fetch_document,
        routes::render_report::render_report
    ])
}
