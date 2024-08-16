// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_generator::find_miru_annotation;
use super::eml_generator::ValidateAnnotations;
use crate::{
    postgres::{
        election_event::get_election_event_by_election_area, tally_session::get_tally_session_by_id,
    },
    services::consolidation::eml_generator::{MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA},
    types::miru_plugin::MiruTallySessionData,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use tracing::{info, instrument};

#[instrument(err)]
pub async fn upload_transmission_package_signature_service(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    public_key: &str,
) -> Result<()> {
    let election_event =
        get_election_event_by_election_area(&hasura_transaction, tenant_id, election_id, area_id)
            .await
            .with_context(|| "Error fetching election event")?;

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session")?;
    let tally_annotations = tally_session.get_valid_annotations()?;

    let transmission_data: MiruTallySessionData =
        find_miru_annotation(MIRU_TALLY_SESSION_DATA, &tally_annotations)
            .with_context(|| {
                format!(
                    "Missing tally session annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA
                )
            })
            .map(|tally_session_data_js| {
                deserialize_str(&tally_session_data_js).map_err(|err| anyhow!("{}", err))
            })
            .flatten()
            .unwrap_or(vec![]);

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        info!("transmission package not found, skipping");
        return Ok(());
    };
    Ok(())
}
