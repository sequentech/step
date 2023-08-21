// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use rocket::response::Debug;
use reqwest;
use serde::Deserialize;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use handlebars::Handlebars;
use headless_chrome::types::PrintToPdfOptions;
use std::time::Duration;
use tempfile::NamedTempFile;
use std::io::{self, Write, Read};
use std::fs::File;

mod pdf;

#[derive(Deserialize, Debug)]
struct Body {
    template: String,
    variables: Value, //JSON
    format: String // html|text|pdf
}

#[post("/render-template", format = "json", data="<body>")]
async fn render_template(body: Json<Body>) -> Result<Vec<u8>, Debug<reqwest::Error>> {
    let input = body.into_inner();

    let reg = Handlebars::new();
    let render = reg.render_template(input.template.as_str(), &input.variables).unwrap();

    let five_seconds = Duration::new(5, 0);

    // Create a file inside of `std::env::temp_dir()`.
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(render.as_bytes()).unwrap();

    let file_path = file.path().to_str().expect("Path should be unicode");
    println!("path: {}", file_path.clone());
    let url_path = format!("file://{}", file_path);

/*
    let file_path = "/Users/felixrobles/workspace/backend-services/temp.html";
    let mut file = File::create(file_path).expect("Failed to create file");
    file.write_all(render.as_bytes()).expect("Failed to write to file");
*/
    let bytes = pdf::print_to_pdf(
        url_path.as_str(),
        PrintToPdfOptions {
            landscape: None,
            display_header_footer: None,
            print_background: None,
            scale: None,
            paper_width: None,
            paper_height: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            page_ranges: None,
            ignore_invalid_page_ranges: None,
            header_template: None,
            footer_template: None,
            prefer_css_page_size: None,
            transfer_mode: None,
        },
        Some(five_seconds)
    ).unwrap();

    Ok(bytes)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![render_template])
}