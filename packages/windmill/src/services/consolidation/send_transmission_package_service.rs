// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    create_transmission_package_service::update_transmission_package_annotations,
    ecies_encrypt::generate_ecies_key_pair,
    eml_generator::{
        find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
        MIRU_AREA_STATION_ID, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
    },
    logs::{error_sending_transmission_package_to_ccs_log, send_transmission_package_to_ccs_log},
    transmission_package::create_transmission_package,
};
use crate::{
    postgres::{
        area::get_area_by_id, document::get_document, election::get_election_by_id,
        election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::{
        database::get_hasura_pool,
        date::ISO8601,
        documents::{get_document_as_temp_file, upload_and_return_document_postgres},
        temp_path::get_file_size,
    },
    types::miru_plugin::{
        MiruCcsServer, MiruServerDocument, MiruTallySessionData, MiruTransmissionPackageData,
    },
};
use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use deadpool_postgres::Client as DbClient;
use reqwest::multipart;
use sequent_core::{
    ballot::Annotations,
    serialization::deserialize_with_path::deserialize_str,
    types::hasura::core::{ElectionEvent, TallySession},
    util::date_time::get_system_timezone,
};
use std::cmp::Ordering;
use std::io::{Read, Seek};
use tempfile::NamedTempFile;
use tracing::{info, instrument};

const SEND_ELECTION_RESULTS_API_PATH: &str = "/api/receiver/v1/acm/election-results";

#[instrument(skip(transmission_package), err)]
async fn send_package_to_ccs_server(
    mut transmission_package: NamedTempFile,
    ccs_server: &MiruCcsServer,
) -> Result<NamedTempFile> {
    // transmission_package the file to the beginning so it can be read
    transmission_package.rewind()?;

    // Read the file contents into a Vec<u8>
    let mut transmission_package_bytes = Vec::new();
    transmission_package.read_to_end(&mut transmission_package_bytes)?;

    let uri = format!("{}{}", ccs_server.address, SEND_ELECTION_RESULTS_API_PATH);
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    // Create a multipart form
    let form = multipart::Form::new().part(
        "zip",
        multipart::Part::bytes(transmission_package_bytes)
            .file_name("file.zip")
            .mime_str("application/zip")?,
    );

    // Send the POST request
    let response = client
        .post(&uri)
        .multipart(form)
        .send()
        .await
        .map_err(|err| anyhow!("{:?}", err))?;
    let response_str = format!("{:?}", response);
    let is_success = response.status().is_success();
    let text = response.text().await?;

    // Check if the request was successful
    if !is_success {
        return Err(anyhow::anyhow!(
            "Failed to send package. Text: {}. Response: {}",
            text,
            response_str
        ));
    }
    Ok(transmission_package)
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

    let Some(election) = get_election_by_id(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        election_id,
    )
    .await?
    else {
        info!("Election not found");
        return Ok(());
    };
    let area = get_area_by_id(&hasura_transaction, tenant_id, &area_id)
        .await
        .with_context(|| format!("Error fetching area {}", area_id))?
        .ok_or_else(|| anyhow!("Can't find area {}", area_id))?;
    let area_name = area.name.clone().unwrap_or("".into());
    let area_annotations = area.get_valid_annotations()?;

    let area_station_id = find_miru_annotation(MIRU_AREA_STATION_ID, &area_annotations)
        .with_context(|| {
            format!(
                "Missing area annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_AREA_STATION_ID
            )
        })?;

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

    let mut documents = transmission_area_election.documents.clone();
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

    let acm_key_pair = generate_ecies_key_pair()?;
    let mut new_miru_document = miru_document.clone();
    let mut new_transmission_area_election = transmission_area_election.clone();

    let servers_sent_to: Vec<String> = miru_document
        .servers_sent_to
        .clone()
        .iter()
        .map(|value| value.name.clone())
        .collect();

    for ccs_server in &transmission_area_election.servers {
        let transmission_package = create_transmission_package(
            time_zone.clone(),
            now_utc.clone(),
            &election_event_annotations,
            compressed_xml_bytes.clone(),
            &acm_key_pair,
            &ccs_server.public_key_pem,
            &area_station_id,
        )
        .await?;
        match send_package_to_ccs_server(transmission_package, ccs_server).await {
            Ok(tmp_file_zip) => {
                let name = format!("er_{}.zip", miru_document.transaction_id);

                let temp_path = tmp_file_zip.into_temp_path();
                let temp_path_string = temp_path.to_string_lossy().to_string();
                let file_size = get_file_size(temp_path_string.as_str())
                    .with_context(|| "Error obtaining file size")?;

                let document = upload_and_return_document_postgres(
                    &hasura_transaction,
                    &temp_path_string,
                    file_size,
                    "applization/zip",
                    tenant_id,
                    &election_event.id,
                    &name,
                    None,
                    false,
                )
                .await?;
                new_transmission_area_election
                    .logs
                    .push(send_transmission_package_to_ccs_log(
                        &Local::now(),
                        election_id,
                        &election.name,
                        area_id,
                        &area_name,
                        &ccs_server.name,
                        &ccs_server.address,
                        new_miru_document
                            .signatures
                            .clone()
                            .into_iter()
                            .map(|signature| signature.trustee_name.clone())
                            .collect(),
                    ));
                new_miru_document.servers_sent_to.push(MiruServerDocument {
                    name: ccs_server.name.clone(),
                    document_id: document.id.clone(),
                    sent_at: ISO8601::to_string(&Local::now()),
                });
            }
            Err(err) => {
                let error_str = format!("{}", err);
                new_transmission_area_election.logs.push(
                    error_sending_transmission_package_to_ccs_log(
                        &Local::now(),
                        election_id,
                        &election.name,
                        area_id,
                        &area_name,
                        &ccs_server.name,
                        &ccs_server.address,
                        new_miru_document
                            .signatures
                            .clone()
                            .into_iter()
                            .map(|signature| signature.trustee_name.clone())
                            .collect(),
                        &error_str,
                    ),
                );
            }
        }
    }

    new_transmission_area_election.documents = new_transmission_area_election
        .documents
        .into_iter()
        .map(|value| {
            if value.document_id == new_miru_document.document_id {
                new_miru_document.clone()
            } else {
                value
            }
        })
        .collect();

    update_transmission_package_annotations(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
        area_id,
        election_id,
        transmission_data.clone(),
        new_transmission_area_election,
        tally_annotations.clone(),
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
