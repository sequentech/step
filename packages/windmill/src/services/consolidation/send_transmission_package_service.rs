use std::cmp::Ordering;

// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_generator::{
    find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
    MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
};
use crate::{
    postgres::{
        document::get_document, election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::{database::get_hasura_pool, date::ISO8601, documents::get_document_as_temp_file},
    types::miru_plugin::{MiruTallySessionData, MiruTransmissionPackageData},
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str,
    types::hasura::core::{ElectionEvent, TallySession},
};
use tracing::{info, instrument};

#[instrument(err)]
pub async fn find_transmission_area_election(
    tally_session: &TallySession,
    election_event: &ElectionEvent,
    election_id: &str,
    area_id: &str,
) -> Result<Option<MiruTransmissionPackageData>> {
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
    Ok(transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }))
}

#[instrument(err)]
pub async fn send_transmission_package_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

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

    let Some(transmission_area_election) =
        find_transmission_area_election(&tally_session, &election_event, election_id, area_id)
            .await?
    else {
        info!("transmission package not found, skipping");
        return Ok(());
    };

    let mut documents = transmission_area_election.documents;
    documents.sort_by(|a, b| {
        let Ok(a_date) = ISO8601::to_date(&a.created_at) else {
            return Ordering::Equal;
        };
        let Ok(b_date) = ISO8601::to_date(&b.created_at) else {
            return Ordering::Equal;
        };
        if a_date > b_date {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });
    let Some(miru_document) = documents.first().cloned() else {
        info!("transmission package document not found, skipping");
        return Ok(());
    };

    let document = get_document(
        &hasura_transaction,
        tenant_id,
        Some(election_event.id.clone()),
        &miru_document.document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", miru_document.document_id))?;

    let compressed_xml = get_document_as_temp_file(tenant_id, &document).await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
