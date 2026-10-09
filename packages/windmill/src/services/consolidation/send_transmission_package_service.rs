// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    signed_transmission_package::lock_transmission_data,
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
        documents::{get_document_as_temp_file, upload_and_return_document},
        signing::actions::transmission::{transmission_send_check, TransmissionRefusal},
        time_zones::event_time_zone,
    },
    types::miru_plugin::{
        MiruCcsServer, MiruDocument, MiruServerDocument, MiruServerDocumentStatus,
        MiruTallySessionData, MiruTransmissionPackageData,
    },
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use reqwest::multipart;
use sequent_core::services::translations::Name;
use sequent_core::util::temp_path::{generate_temp_file, get_file_size};
use sequent_core::{
    ballot::Annotations,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    types::{
        ceremonies::Log,
        hasura::core::{ElectionEvent, TallySession},
    },
};
use std::io::{Read, Seek};
use std::{cmp::Ordering, path::Path};
use tempfile::{tempdir, NamedTempFile};
use tracing::{info, instrument};
use uuid::Uuid;

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
        .danger_accept_invalid_certs(
            ccs_server
                .tls_verification_policy()
                .accepts_invalid_certificates(),
        )
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

/// A send refused because the package has fewer signatures than it needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmissionSignaturesShort {
    pub signatures: usize,
    pub threshold: i64,
}

impl std::fmt::Display for TransmissionSignaturesShort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The transmission package has {} of the {} signatures it needs; it can't be sent yet",
            self.signatures, self.threshold
        )
    }
}

impl std::error::Error for TransmissionSignaturesShort {}

/// Whether the package's latest document has the signatures its stored
/// threshold asks for (the 2025 check). A threshold of -1 asks for none.
pub fn check_transmission_signatures(
    package: &MiruTransmissionPackageData,
) -> std::result::Result<(), TransmissionSignaturesShort> {
    check_transmission_signatures_against(package, package.threshold)
}

/// Whether the package's latest document has `threshold` signatures; -1
/// asks for none, and a package without a document has none.
pub fn check_transmission_signatures_against(
    package: &MiruTransmissionPackageData,
    threshold: i64,
) -> std::result::Result<(), TransmissionSignaturesShort> {
    let signatures = get_latest_miru_document(&package.documents)
        .map_or(0, |document| document.signatures.len());
    if threshold > -1 && (signatures as i64) < threshold {
        return Err(TransmissionSignaturesShort {
            signatures,
            threshold,
        });
    }
    Ok(())
}

/// Why a send can't be queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendRefusal {
    /// There is no such tally session in the event.
    NoTallySession,
    Refused(TransmissionRefusal),
}

/// Why the package of a Post and country can't be sent, for the send route
/// to answer before it queues the send. `None` when it can go, or there is
/// no package to check.
#[instrument(err)]
pub async fn signed_transmission_refusal(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
) -> Result<Option<SendRefusal>> {
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
    let Ok(tally_session_uuid) = Uuid::parse_str(tally_session_id) else {
        return Ok(Some(SendRefusal::NoTallySession));
    };
    let exists = hasura_transaction
        .query_opt(
            "SELECT 1 FROM sequent_backend.tally_session
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[
                &Uuid::parse_str(tenant_id).with_context(|| "Error parsing the tenant id")?,
                &Uuid::parse_str(&election_event.id)
                    .with_context(|| "Error parsing the election event id")?,
                &tally_session_uuid,
            ],
        )
        .await?
        .is_some();
    if !exists {
        return Ok(Some(SendRefusal::NoTallySession));
    }
    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session")?;
    let Some(package) = tally_session
        .get_annotations()
        .unwrap_or_default()
        .into_iter()
        .find(|package| package.area_id == area_id && package.election_id == election_id)
    else {
        return Ok(None);
    };
    Ok(transmission_send_check(
        &hasura_transaction,
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing the tenant id")?,
        Uuid::parse_str(&election_event.id)
            .with_context(|| "Error parsing the election event id")?,
        &package,
    )
    .await?
    .err()
    .map(SendRefusal::Refused))
}

/// The package's latest document: the last one written. Documents are
/// only ever appended, so their order is the order they were made in,
/// whatever clock the workers that made them had.
pub fn get_latest_miru_document(input_documents: &[MiruDocument]) -> Option<MiruDocument> {
    input_documents.last().cloned()
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

    // Under the tally session's row lock, from the packages as they stand.
    let locked = lock_transmission_data(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    let tally_annotations = locked.annotations;
    let transmission_data = locked.packages;

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

    // Under the tally session's row lock, from the packages as they stand.
    let locked = lock_transmission_data(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    let tally_annotations = locked.annotations;
    let transmission_data = locked.packages;

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

    // One send of a package at a time: a second waits here, then reads
    // what the first one sent (each server's outcome commits as it goes).
    hasura_transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!(
                "transmission-send:{tenant_id}:{tally_session_id}:{election_id}:{area_id}"
            )],
        )
        .await
        .with_context(|| "Error waiting for another send of the package")?;
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

    // A package that needs signatures goes once its signing request ran,
    // with the request's signatures; otherwise with the Post's 2025 minimum.
    transmission_send_check(
        &hasura_transaction,
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing the tenant id")?,
        Uuid::parse_str(&election_event.id)
            .with_context(|| "Error parsing the election event id")?,
        &transmission_area_election,
    )
    .await?
    .map_err(anyhow::Error::new)?;
    // The log lines and send times are in the event's primary zone.
    let zone = event_time_zone(
        &hasura_transaction,
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing the tenant id")?,
        Uuid::parse_str(&election_event.id)
            .with_context(|| "Error parsing the election event id")?,
    )
    .await?;

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
        let election_name = election.get_name(&election.get_default_language());
        match send_package_to_ccs_server(&second_zip_path, ccs_server, false).await {
            Ok(_) => {
                let time_now = Utc::now().with_timezone(&zone);
                let new_log = send_transmission_package_to_ccs_log(
                    &time_now,
                    election_id,
                    &election_name,
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
                    sent_at: time_now.to_rfc3339(),
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
                let time_now = Utc::now().with_timezone(&zone);
                let new_log = error_sending_transmission_package_to_ccs_log(
                    &time_now,
                    election_id,
                    &election_name,
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
                    sent_at: time_now.to_rfc3339(),
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
                        &Utc::now().with_timezone(&zone),
                        election_id,
                        &election_name,
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
                        &Utc::now().with_timezone(&zone),
                        election_id,
                        &election_name,
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
