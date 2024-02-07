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

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn create_vote_receipt(
    element_id: String,
    ballot_id: String,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    dbg!(&tenant_id);
    dbg!(&election_event_id);
    // let auth_headers = keycloak::get_client_credentials().await?;
    //
    // dbg!(&element_id);
    // dbg!(&ballot_id);
    //
    // let mut map = Map::new();
    // map.insert(
    //     "map".to_string(),
    //     serde_json::from_str("{\"name\": \"Kevin le bg\"}").map_err(|err| anyhow!("{}", err))?,
    // );
    //
    // // render handlebars template
    // let render = reports::render_template_text("<h1>Bonjour, {{map.name}}</h1>", map)
    //     .map_err(|err| anyhow!("{}", err))?;
    //
    // dbg!(&render);
    //
    // let bytes_pdf = pdf::html_to_pdf(render).map_err(|err| anyhow!("{}", err))?;
    //
    // dbg!(&bytes_pdf);
    //
    // let (_temp_path, temp_path_string, file_size) =
    //     write_into_named_temp_file(&bytes_pdf, "vote-receipt-", ".html")
    //         .with_context(|| "Error writing to file")?;
    //
    // dbg!(&file_size);
    // dbg!(&temp_path_string);
    // dbg!(&_temp_path);

    // let _document = upload_and_return_document(
    //     temp_path_string,
    //     file_size,
    //     "application/pdf".to_string(),
    //     auth_headers.clone(),
    //     tenant_id,
    //     election_event_id,
    //     "vote-receipt".to_string(),
    // )
    // .await?;

    Ok(())
}
