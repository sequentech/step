// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::{documents::upload_and_return_document, temp_path::write_into_named_temp_file},
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TemplateData {
    ballot_id: String,
    ballot_tracker_url: String,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn create_vote_receipt(
    element_id: String,
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;

    let mut map = Map::new();
    map.insert(
        "data".to_string(),
        serde_json::to_value(TemplateData {
            ballot_id: ballot_id.clone(),
            ballot_tracker_url,
        })
        .map_err(|err| anyhow!("{}", err))?,
    );

    let default_html_template = include_str!("../resources/vote_receipt.hbs");

    let render = reports::render_template_text(default_html_template, map)
        .map_err(|err| anyhow!("{}", err))?;

    let bytes_pdf = pdf::html_to_pdf(render).map_err(|err| anyhow!("{}", err))?;

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "vote-receipt-", ".html")
            .with_context(|| "Error writing to file")?;

    let _document = upload_and_return_document(
        temp_path_string,
        file_size,
        "application/pdf".to_string(),
        auth_headers.clone(),
        tenant_id,
        election_event_id,
        format!("vote-receipt-{ballot_id}.pdf"),
        Some(element_id),
        true,
    )
    .await?;

    Ok(())
}
