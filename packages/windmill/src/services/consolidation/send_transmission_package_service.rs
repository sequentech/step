// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    create_transmission_package_service::update_transmission_package_annotations,
    eml_generator::{
        find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
        MIRU_AREA_STATION_ID, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
    },
    logs::{
        error_sending_logs_to_ccs_log, error_sending_transmission_package_to_ccs_log,
        send_logs_to_ccs_log, send_transmission_package_to_ccs_log,
    },
    transmission_package::create_transmission_package,
    zip::unzip_file,
};
use crate::{
    postgres::{
        area::get_area_by_id, document::get_document, election::get_election_by_id,
        election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::{
        database::get_hasura_pool,
        documents::{get_document_as_temp_file, upload_and_return_document_postgres},
    },
    types::miru_plugin::{
        MiruCcsServer, MiruDocument, MiruServerDocument, MiruServerDocumentStatus,
        MiruTallySessionData, MiruTransmissionPackageData,
    },
};
use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use deadpool_postgres::Client as DbClient;
use reqwest::multipart;
use sequent_core::util::temp_path::{generate_temp_file, get_file_size};
use sequent_core::{
    ballot::Annotations,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    services::date::ISO8601,
    types::{
        ceremonies::Log,
        hasura::core::{ElectionEvent, TallySession},
    },
};
use std::io::{Read, Seek};
use std::{cmp::Ordering, path::Path};
use tempfile::{tempdir, NamedTempFile};
use tracing::{info, instrument};

const SEND_ELECTION_RESULTS_API_PATH: &str = "/api/receiver/v1/acm/election-results";

const SEND_LOGS_API_PATH: &str = "/api/receiver/v1/acm/audit-logs";

#[instrument(err)]
async fn send_package_to_ccs_server(
    transmission_package_path: &Path,
    ccs_server: &MiruCcsServer,
    is_log: bool,
) -> Result<()> {
    // Read the file contents into a Vec<u8>
    let transmission_package_bytes = std::fs::read(transmission_package_path)?;

    let base_url = if is_log {
        SEND_LOGS_API_PATH
    } else {
        SEND_ELECTION_RESULTS_API_PATH
    };

    let uri = format!("{}{}", ccs_server.address, base_url);
    info!("Sending package to url {}", uri);
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
    info!(
        "Response code: {}. Response: '{}'",
        response.status(),
        response_str
    );
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
    Ok(())
}

#[instrument(skip_all)]
pub fn get_latest_miru_document(input_documents: &Vec<MiruDocument>) -> Option<MiruDocument> {
    let mut documents = input_documents.clone();
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
    documents.first().cloned()
}

async fn update_miru_document(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    election_event_id: &str,
    new_miru_document: MiruDocument,
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

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session")?;

    let tally_annotations_js = tally_session
        .annotations
        .clone()
        .ok_or_else(|| anyhow!("Missing tally session annotations"))?;

    let tally_annotations: Annotations = deserialize_value(tally_annotations_js)?;

    let transmission_data = tally_session.get_annotations()?;

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        return Err(anyhow!("transmission package not found, unexpected"));
    };
    let mut new_transmission_area_election = transmission_area_election.clone();

    new_transmission_area_election.documents = new_transmission_area_election
        .documents
        .into_iter()
        .map(|value| {
            if value.document_ids.all_servers == new_miru_document.document_ids.all_servers {
                new_miru_document.clone()
            } else {
                value
            }
        })
        .collect();

    update_transmission_package_annotations(
        &hasura_transaction,
        tenant_id,
        election_event_id,
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

async fn record_new_log(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    election_event_id: &str,
    log: Log,
    new_miru_document: Option<MiruDocument>,
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

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session")?;

    let tally_annotations_js = tally_session
        .annotations
        .clone()
        .ok_or_else(|| anyhow!("Missing tally session annotations"))?;

    let tally_annotations: Annotations = deserialize_value(tally_annotations_js)?;

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
        return Err(anyhow!("transmission package not found, unexpected"));
    };
    let mut new_transmission_area_election = transmission_area_election.clone();
    new_transmission_area_election.logs.push(log);
    if let Some(miru_document) = new_miru_document.clone() {
        new_transmission_area_election.documents = new_transmission_area_election
            .documents
            .into_iter()
            .map(|value| {
                if value.document_ids.all_servers == miru_document.document_ids.all_servers {
                    miru_document.clone()
                } else {
                    value
                }
            })
            .collect();
    }

    update_transmission_package_annotations(
        &hasura_transaction,
        tenant_id,
        election_event_id,
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
    let election_event_annotations = election_event.get_annotations()?;

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
    let election_annotations = election.get_annotations()?;
    let area = get_area_by_id(&hasura_transaction, tenant_id, &area_id)
        .await
        .with_context(|| format!("Error fetching area {}", area_id))?
        .ok_or_else(|| anyhow!("Can't find area {}", area_id))?;
    let area_name = area.name.clone().unwrap_or("".into());
    let area_annotations = area.get_annotations()?;

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session")?;
    let transmission_data = tally_session.get_annotations()?;

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        info!("transmission package not found, skipping");
        return Ok(());
    };
    let Some(miru_document) = get_latest_miru_document(&transmission_area_election.documents)
    else {
        info!("transmission package document not found, skipping");
        return Ok(());
    };

    let document = get_document(
        &hasura_transaction,
        tenant_id,
        Some(election_event.id.clone()),
        &miru_document.document_ids.all_servers,
    )
    .await?
    .ok_or_else(|| {
        anyhow!(
            "Can't find document {}",
            miru_document.document_ids.all_servers
        )
    })?;

    let mut compressed_zip = get_document_as_temp_file(tenant_id, &document).await?;

    let zip_output_temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    unzip_file(compressed_zip.path(), zip_output_temp_dir.path())?;

    let mut new_miru_document = miru_document.clone();
    let mut new_transmission_area_election = transmission_area_election.clone();

    let servers_sent_to: Vec<String> = miru_document
        .servers_sent_to
        .clone()
        .iter()
        .map(|value| value.name.clone())
        .collect();

    if transmission_area_election.threshold > -1
        && (miru_document.signatures.len() as i64) < transmission_area_election.threshold
    {
        info!(
            "Can't send to servers as number of signatures {} is less than threshold {}",
            miru_document.signatures.len(),
            transmission_area_election.threshold
        );
        return Ok(());
    }

    for ccs_server in &transmission_area_election.servers {
        if servers_sent_to.contains(&ccs_server.name) {
            info!(
                "SHOULD BE skipping sending to server '{}' as already sent",
                ccs_server.name
            );
            continue;
        }
        let second_zip_folder_path = zip_output_temp_dir.path().join(&ccs_server.tag);
        let second_zip_path =
            second_zip_folder_path.join(format!("er_{}.zip", area_annotations.station_id));
        match send_package_to_ccs_server(&second_zip_path, ccs_server, false).await {
            Ok(_) => {
                let time_now = Local::now();
                let new_log = send_transmission_package_to_ccs_log(
                    &time_now,
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
                        .map(|signature| signature.sbei_miru_id.clone())
                        .collect(),
                );
                new_miru_document.servers_sent_to.push(MiruServerDocument {
                    name: ccs_server.name.clone(),
                    sent_at: ISO8601::to_string(&time_now),
                    status: MiruServerDocumentStatus::SUCCESS,
                });
                record_new_log(
                    tenant_id,
                    election_id,
                    area_id,
                    tally_session_id,
                    &election_event.id,
                    new_log,
                    Some(new_miru_document.clone()),
                )
                .await?;
            }
            Err(err) => {
                let error_str = format!("{}", err);
                let time_now = Local::now();
                let new_log = error_sending_transmission_package_to_ccs_log(
                    &time_now,
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
                        .map(|signature| signature.sbei_miru_id.clone())
                        .collect(),
                    &error_str,
                );
                new_miru_document.servers_sent_to.push(MiruServerDocument {
                    name: ccs_server.name.clone(),
                    sent_at: ISO8601::to_string(&time_now),
                    status: MiruServerDocumentStatus::ERROR,
                });
                record_new_log(
                    tenant_id,
                    election_id,
                    area_id,
                    tally_session_id,
                    &election_event.id,
                    new_log,
                    Some(new_miru_document.clone()),
                )
                .await?;
            }
        }
        let with_logs = ccs_server.send_logs.clone().unwrap_or_default();
        let logs_zip_path =
            second_zip_folder_path.join(format!("al_{}.zip", area_annotations.station_id));
        if with_logs {
            match send_package_to_ccs_server(&logs_zip_path, ccs_server, true).await {
                Ok(_) => {
                    let new_log = send_logs_to_ccs_log(
                        &Local::now(),
                        election_id,
                        &election.name,
                        area_id,
                        &area_name,
                        &ccs_server.name,
                        &ccs_server.address,
                    );
                    record_new_log(
                        tenant_id,
                        election_id,
                        area_id,
                        tally_session_id,
                        &election_event.id,
                        new_log,
                        None,
                    )
                    .await?;
                }
                Err(err) => {
                    let error_str = format!("{}", err);
                    let new_log = error_sending_logs_to_ccs_log(
                        &Local::now(),
                        election_id,
                        &election.name,
                        area_id,
                        &area_name,
                        &ccs_server.name,
                        &ccs_server.address,
                        &error_str,
                    );
                    record_new_log(
                        tenant_id,
                        election_id,
                        area_id,
                        tally_session_id,
                        &election_event.id,
                        new_log,
                        None,
                    )
                    .await?;
                }
            }
        }
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
