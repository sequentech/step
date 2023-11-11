// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use tracing::instrument;

use crate::services::documents::upload_and_return_document;
use crate::hasura;


use crate::types::error::Result;

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

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn render_report(
    input: RenderTemplateBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    println!("auth headers: {:#?}", auth_headers);
    let hasura_response =
        hasura::tenant::get_tenant(auth_headers.clone(), tenant_id.clone()).await?;
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

    let _document_json = upload_and_return_document(
        bytes,
        "application/pdf".to_string(),
        auth_headers.clone(),
        tenant_id,
        election_event_id,
        input.name,
    )
    .await?;

    Ok(())
}
