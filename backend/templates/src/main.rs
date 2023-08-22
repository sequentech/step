// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use rocket::response::Debug;
use reqwest;
use rocket::serde::Deserialize;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use handlebars::Handlebars;
use headless_chrome::types::PrintToPdfOptions;
use std::time::Duration;
use tempfile::tempdir;
use std::io::Write;
use std::fs::File;
use either::*;

mod pdf;
mod s3;

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
enum FormatType {
    TEXT,
    PDF
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Body {
    template: String,
    variables: Value, //JSON
    format: FormatType // html|text|pdf
}

#[post("/render-template", format = "json", data="<body>")]
async fn render_template(body: Json<Body>) -> Result<Either<String, Vec<u8>>, Debug<reqwest::Error>> {
    let input = body.into_inner();

    s3::upload_to_s3().await.unwrap();

    // render handlebars template
    let reg = Handlebars::new();
    let render = reg.render_template(input.template.as_str(), &input.variables).unwrap();

    // if output format is text/html, just return that
    if FormatType::TEXT == input.format {
        return Ok(Left(render))
    }

    // Create temp html file
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("index.html");
    let mut file = File::create(file_path.clone()).unwrap();
    let file_path_str = file_path.to_str().unwrap();
    file.write_all(render.as_bytes()).unwrap();
    let url_path = format!("file://{}", file_path_str);

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
        Some(Duration::new(1, 0))
    ).unwrap();

    Ok(Right(bytes))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![render_template])
}