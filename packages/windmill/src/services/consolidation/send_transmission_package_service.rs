// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    ecies_encrypt::generate_ecies_key_pair,
    eml_generator::{
        find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
        MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
    },
    transmission_package::create_transmission_package,
};
use crate::{
    postgres::{
        document::get_document, election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::{database::get_hasura_pool, date::ISO8601, documents::get_document_as_temp_file},
    types::miru_plugin::{MiruCcsServer, MiruTallySessionData, MiruTransmissionPackageData},
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str,
    types::hasura::core::{ElectionEvent, TallySession},
    util::date_time::get_system_timezone,
};
use std::cmp::Ordering;
use std::io::{Read, Seek};
use tempfile::NamedTempFile;
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

#[instrument(skip(transmission_package), err)]
async fn send_package_to_ccs_server(
    mut transmission_package: NamedTempFile,
    ccs_server: &MiruCcsServer,
) -> Result<()> {
    // transmission_package the file to the beginning so it can be read
    transmission_package.rewind()?;

    // Read the file contents into a Vec<u8>
    let mut transmission_package_bytes = Vec::new();
    transmission_package.read_to_end(&mut transmission_package_bytes)?;

    let uri = format!("{}/", ccs_server.address);

    let client = reqwest::Client::new();
    Ok(())
}

#[instrument(err)]
pub async fn send_transmission_package_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
) -> Result<()> {
    let time_zone = get_system_timezone();
    let now_utc = Utc::now();
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
    let election_event_annotations = election_event.get_valid_annotations()?;

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
            Ordering::Less
        } else if a_date < b_date {
            Ordering::Greater
        } else {
            Ordering::Equal
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

    let mut compressed_xml = get_document_as_temp_file(tenant_id, &document).await?;
    // Rewind the file to the beginning so it can be read
    compressed_xml.rewind()?;

    // Read the file contents into a Vec<u8>
    let mut compressed_xml_bytes = Vec::new();
    compressed_xml.read_to_end(&mut compressed_xml_bytes)?;

    let (private_key_pem_str, acm_public_key_pem_str) = generate_ecies_key_pair()?;
    for ccs_server in &transmission_area_election.servers {
        let mut transmission_package = create_transmission_package(
            time_zone.clone(),
            now_utc.clone(),
            &election_event_annotations,
            compressed_xml_bytes.clone(),
            &acm_public_key_pem_str,
            &ccs_server.public_key_pem,
        )
        .await?;
        send_package_to_ccs_server(transmission_package, ccs_server).await?;
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
