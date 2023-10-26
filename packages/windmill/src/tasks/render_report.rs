// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use celery::error::TaskError;
use celery::prelude::*;
use handlebars::Handlebars;
use headless_chrome::types::PrintToPdfOptions;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use sequent_core::services::connection;
use sequent_core::services::{pdf, reports};
use serde_json::json;
use serde_json::{Map, Value};
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;
use tracing::instrument;

use crate::hasura;
use crate::hasura::event_execution::insert_event_execution_with_result;
use crate::services::s3;
use crate::types::scheduled_event::ScheduledEvent;
use sequent_core::services::openid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum FormatType {
    TEXT,
    PDF,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RenderTemplateBody {
    template: String,
    tenant_id: String,
    election_event_id: String,
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
#[celery::task(time_limit = 60000)]
pub async fn render_report(input: RenderTemplateBody, event: ScheduledEvent) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    println!("auth headers: {:#?}", auth_headers);
    let hasura_response = hasura::tenant::get_tenant(auth_headers.clone(), input.tenant_id.clone())
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
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
    let render = reports::render_template_text(input.template.as_str(), variables_map)
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    // if output format is text/html, just return that
    if FormatType::TEXT == input.format {
        upload_and_return_document(
            render.into_bytes(),
            "text/plain".to_string(),
            auth_headers.clone(),
            input.tenant_id,
            input.election_event_id,
            input.name,
        )
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
        return Ok(());
    }

    let bytes =
        pdf::html_to_pdf(render).map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let document_json = upload_and_return_document(
        bytes,
        "application/pdf".to_string(),
        auth_headers.clone(),
        input.tenant_id,
        input.election_event_id,
        input.name,
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let document = document_json.clone().into_inner();
    let document_value = serde_json::to_value(document)
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    insert_event_execution_with_result(auth_headers, event, Some(document_value))
        .await
        .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;

    Ok(())
}
