// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::env;

use super::s3;
use crate::postgres::{self, communication_template, election};
use crate::services::{
    documents::upload_and_return_document, temp_path::write_into_named_temp_file,
};
use anyhow::{anyhow, Context, Result};
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;

use deadpool_postgres::Transaction;
use uuid::Uuid;

#[instrument(skip(hasura_transaction), err)]
pub async fn get_manual_verification_pdf(
    hasura_transaction: &Transaction<'_>,
    element_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<()> {
    Ok(())
}
