// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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

use crate::connection;
use crate::hasura;
use crate::pdf;
use crate::s3;

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
pub enum FormatType {
    TEXT,
    PDF,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct RenderTemplateBody {
    template: String,
    tenant_id: String,
    election_event_id: String,
    name: String,
    format: FormatType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct RenderTemplateResponse {
    id: String,
    election_event_id: Option<String>,
    tenant_id: Option<String>,
    name: Option<String>,
    size: Option<i64>,
    media_type: Option<String>,
}

async fn upload_and_return_document(
    bytes: Vec<u8>,
    media_type: String,
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    name: String,
) -> Result<Json<RenderTemplateResponse>, Debug<reqwest::Error>> {
    let size = bytes.len();

    let new_document = hasura::insert_document(
        auth_headers,
        tenant_id.clone(),
        election_event_id.clone(),
        name,
        media_type,
        size as i64,
    )
    .await?;

    let document = &new_document
        .data
        .expect("expected data".into())
        .insert_sequent_backend_document
        .unwrap()
        .returning[0];

    let document_id = document.id.clone();

    let document_s3_key =
        s3::get_document_key(tenant_id, election_event_id, document_id);

    s3::upload_to_s3(&bytes, document_s3_key, "application/pdf".into())
        .await
        .unwrap();

    Ok(Json(RenderTemplateResponse {
        id: document.id.clone(),
        election_event_id: document.election_event_id.clone(),
        tenant_id: document.tenant_id.clone(),
        name: document.name.clone(),
        size: document.size.clone(),
        media_type: document.media_type.clone(),
    }))
}

#[post("/render-report", format = "json", data = "<body>")]
pub async fn render_report(
    body: Json<RenderTemplateBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<RenderTemplateResponse>, Debug<reqwest::Error>> {
    let input = body.into_inner();

    println!("auth headers: {:#?}", auth_headers);
    let hasura_response =
        hasura::get_tenant(auth_headers.clone(), input.tenant_id.clone())
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
        return upload_and_return_document(
            render.into_bytes(),
            "text/plain".to_string(),
            auth_headers.clone(),
            input.tenant_id,
            input.election_event_id,
            input.name,
        )
        .await;
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

    upload_and_return_document(
        bytes,
        "application/pdf".to_string(),
        auth_headers.clone(),
        input.tenant_id,
        input.election_event_id,
        input.name,
    )
    .await
}