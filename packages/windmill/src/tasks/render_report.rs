// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use celery::prelude::*;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use sequent_core::services::openid;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use tracing::instrument;

use crate::hasura;
use crate::services::s3;
use crate::types::task_error::into_task_error;
use sequent_core::services::keycloak;
use crate::types::error::{Error, Result};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum FormatType {
    TEXT,
    PDF,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RenderTemplateBody {
    template: String,
    name: String,
    variables: Map<String, Value>,
    format: FormatType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
) -> Result<Json<RenderTemplateResponse>> {
    let size = bytes.len();

    let new_document = hasura::document::insert_document(
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

    let document_s3_key = s3::get_document_key(tenant_id, election_event_id, document_id);

    s3::upload_to_s3(&bytes, document_s3_key, "application/pdf".into()).await?;

    Ok(Json(RenderTemplateResponse {
        id: document.id.clone(),
        election_event_id: document.election_event_id.clone(),
        tenant_id: document.tenant_id.clone(),
        name: document.name.clone(),
        size: document.size.clone(),
        media_type: document.media_type.clone(),
    }))
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn render_report(
    input: RenderTemplateBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials()
        .await?;
    println!("auth headers: {:#?}", auth_headers);
    let hasura_response = hasura::tenant::get_tenant(auth_headers.clone(), tenant_id.clone())
        .await?;
    let username = hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_tenant[0]
        .username
        .clone();
    let mut variables_map = input.variables.clone();
    if !variables_map.contains_key("username") {
        variables_map.insert("username".to_string(), json!(username));
    }

    // render handlebars template
    let render = reports::render_template_text(input.template.as_str(), variables_map)?;

    // if output format is text/html, just return that
    if FormatType::TEXT == input.format {
        upload_and_return_document(
            render.into_bytes(),
            "text/plain".to_string(),
            auth_headers.clone(),
            tenant_id,
            election_event_id,
            input.name,
        )
        .await?;
        return Ok(());
    }

    let bytes = pdf::html_to_pdf(render)?;

    let document_json = upload_and_return_document(
        bytes,
        "application/pdf".to_string(),
        auth_headers.clone(),
        tenant_id,
        election_event_id,
        input.name,
    )
    .await?;

    let document = document_json.clone().into_inner();
    let _document_value = serde_json::to_value(document)?;

    Ok(())
}
