// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use tracing::instrument;

use crate::postgres::tenant::get_tenant_by_id;
use crate::services::documents::upload_and_return_document;
use crate::tasks::render_report::{FormatType, RenderTemplateBody};
use sequent_core::util::temp_path::write_into_named_temp_file;

#[instrument(err, skip(hasura_transaction))]
pub async fn render_report_task(
    hasura_transaction: &Transaction<'_>,
    input: RenderTemplateBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let tenant = get_tenant_by_id(hasura_transaction, &tenant_id).await?;

    let username = tenant.slug.clone();
    let mut variables_map = input.variables.clone();
    if !variables_map.contains_key("username") {
        variables_map.insert("username".to_string(), json!(username));
    }

    // render handlebars template
    let render = reports::render_template_text(input.template.as_str(), variables_map)
        .map_err(|err| anyhow!("{}", err))?;

    // if output format is text/html, just return that
    if FormatType::TEXT == input.format {
        let (_temp_path, temp_path_string, file_size) =
            write_into_named_temp_file(&render.into_bytes(), "reports-", ".html")
                .with_context(|| "Error writing to file")?;
        upload_and_return_document(
            &hasura_transaction,
            &temp_path_string,
            file_size,
            "text/plain",
            &tenant_id,
            Some(election_event_id),
            &input.name,
            None,
            false,
            false,
        )
        .await?;
    } else {
        let bytes = pdf::PdfRenderer::render_pdf(render, None)
            .await
            .with_context(|| "Error converting html to pdf format")?;
        let (_temp_path, temp_path_string, file_size) =
            write_into_named_temp_file(&bytes, "reports-", ".html")
                .with_context(|| "Error writing to file")?;

        let _document = upload_and_return_document(
            &hasura_transaction,
            &temp_path_string,
            file_size,
            "application/pdf",
            &tenant_id,
            Some(election_event_id),
            &input.name,
            None,
            false,
            false,
        )
        .await?;
    }
    Ok(())
}
