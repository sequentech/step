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

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
enum FormatType {
    TEXT,
    PDF,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Body {
    template: String,
    tenant_id: String,
    format: FormatType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct RenderTemplateResponse {
    url: String,
}

#[post("/render-template", format = "json", data = "<body>")]
async fn render_template(
    body: Json<Body>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<RenderTemplateResponse>, Debug<reqwest::Error>> {
    let input = body.into_inner();

    println!("auth headers: {:#?}", auth_headers);
    let hasura_response =
        hasura::run_query(auth_headers, input.tenant_id)
            .await?;
    let username = hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_tenant[0]
        .username
        .clone();
    let variables = json!({ "username": username });

    // render handlebars template
    let reg = Handlebars::new();
    let render = reg
        .render_template(input.template.as_str(), &variables)
        .unwrap();

    // if output format is text/html, just return that
    if FormatType::TEXT == input.format {
        let url = s3::upload_to_s3(&render.into_bytes(), "text/plain".into())
            .await
            .unwrap();
        return Ok(Json(RenderTemplateResponse { url: url }));
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
        Some(Duration::new(1, 0)),
    )
    .unwrap();

    let url = s3::upload_to_s3(&bytes, "application/pdf".into())
        .await
        .unwrap();

    Ok(Json(RenderTemplateResponse { url: url }))
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    rocket::build().mount("/", routes![render_template])
}
