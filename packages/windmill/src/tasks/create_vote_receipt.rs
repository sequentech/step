// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
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
    // tenant_id: String,
    // election_event_id: String,
) -> Result<()> {
    dbg!(&element_id);
    dbg!(&ballot_id);
    Ok(())
}
