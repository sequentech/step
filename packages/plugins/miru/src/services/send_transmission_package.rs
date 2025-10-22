// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    bindings::plugins_manager::{
        documents_manager::documents::create_document_as_temp_file,
        extra_services_manager::request_service::send_zip,
        transactions_manager::{
            postgres_queries::{
                get_area_by_id, get_document, get_election_by_id,
                get_election_event_by_election_area, get_tally_session_by_id,
            },
            transaction::commit_hasura_transaction,
        },
    },
    services::{
        create_transmission_package::update_transmission_package_annotations,
        eml_generator::{
            find_miru_annotation, ValidateAnnotations, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
        },
        logs::{
            error_sending_logs_to_ccs_log, error_sending_transmission_package_to_ccs_log,
            send_logs_to_ccs_log, send_transmission_package_to_ccs_log,
        },
        miru_plugin_types::{
            MiruCcsServer, MiruDocument, MiruServerDocument, MiruServerDocumentStatus,
            MiruTallySessionData,
        },
        zip::unzip_file,
    },
};
use chrono::{DateTime, Local, Utc};
use sequent_core::{
    ballot::Annotations,
    plugins::{get_plugin_shared_dir, Plugins},
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    types::{
        ceremonies::Log,
        hasura::core::{Area, Election, ElectionEvent, TallySession},
    },
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::instrument;
use uuid::Uuid;

const SEND_ELECTION_RESULTS_API_PATH: &str = "/api/receiver/v1/acm/election-results";

const SEND_LOGS_API_PATH: &str = "/api/receiver/v1/acm/audit-logs";

#[instrument(err)]
fn send_package_to_ccs_server(
    transmission_package_path: &str,
    ccs_server: &MiruCcsServer,
    is_log: bool,
) -> Result<(), String> {
    let base_url = if is_log {
        SEND_LOGS_API_PATH
    } else {
        SEND_ELECTION_RESULTS_API_PATH
    };

    let uri = format!("{}{}", ccs_server.address, base_url);
    match send_zip(transmission_package_path, &uri) {
        Ok(_) => {
            println!(
                "Successfully sent package to CCS server '{}'",
                ccs_server.name
            );
        }
        Err(err) => {
            return Err(format!(
                "Failed to send package to CCS server '{}': {}",
                ccs_server.name, err
            )
            .into());
        }
    }

    Ok(())
}

fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => Some(dt.with_timezone(&Utc)),
        Err(e) => {
            println!("Failed to parse date string '{}': {}", date_str, e);
            None
        }
    }
}

#[instrument(skip_all)]
pub fn get_latest_miru_document(input_documents: &Vec<MiruDocument>) -> Option<MiruDocument> {
    input_documents
        .iter()
        .max_by(|a, b| {
            let a_date = parse_date(&a.created_at);
            let b_date = parse_date(&b.created_at);
            a_date.cmp(&b_date)
        })
        .cloned()
}

fn record_new_log(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    election_event_id: &str,
    log: Log,
    new_miru_document: Option<MiruDocument>,
) -> Result<(), String> {
    let tally_session_json =
        get_tally_session_by_id(tenant_id, &election_event_id, tally_session_id).map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to get tally session by id: {}",
                e
            );
            e.to_string()
        })?;

    let tally_session: TallySession = deserialize_str::<TallySession>(&tally_session_json)
        .map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize TallySession: {}",
                e
            );
            e.to_string()
        })?;

    let tally_annotations: Annotations = tally_session
        .annotations
        .clone()
        .map(
            |value| match deserialize_value::<HashMap<String, Value>>(value) {
                Ok(raw_map) => raw_map,
                Err(e) => {
                    println!(
                        "[Guest Plugin Error] Failed to deserialize tally annotations: {}",
                        e
                    );
                    HashMap::new()
                }
            },
        )
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| {
            let string_val = match value {
                Value::String(s) => s,
                _ => value.to_string(),
            };
            (key, string_val)
        })
        .collect();

    let transmission_data: MiruTallySessionData =
        find_miru_annotation(MIRU_TALLY_SESSION_DATA, &tally_annotations)
            .map_err(|e| {
                println!(
                    "[Guest Plugin Error] Missing tally session annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA
                );
                e.to_string()
            })
            .map(|tally_session_data_js| {
                deserialize_str(&tally_session_data_js).map_err(|err| format!("{}", err))
            })
            .flatten()
            .unwrap_or(vec![]);

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        return Err(format!("transmission package not found, unexpected"));
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
        tenant_id,
        election_event_id,
        tally_session_id,
        area_id,
        election_id,
        transmission_data.clone(),
        new_transmission_area_election,
        tally_annotations.clone(),
    )?;

    Ok(())
}

#[instrument(err)]
pub fn send_transmission_package_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
) -> Result<(), String> {
    let election_event_json = get_election_event_by_election_area(tenant_id, election_id, area_id)
        .map_err(|e| e.to_string())?;

    let election_event: ElectionEvent = deserialize_str::<ElectionEvent>(&election_event_json)
        .map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize ElectionEvent: {}",
                e
            );
            e.to_string()
        })?;

    let election_event_annotations = election_event.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get election event annotations: {}",
            e
        );
        e.to_string()
    })?;

    let Some(election_str) = get_election_by_id(tenant_id, &election_event.id, election_id)
        .map_err(|e| e.to_string())?
    else {
        return Err("Election not found".to_string());
    };

    let election: Election = deserialize_str::<Election>(&election_str).map_err(|e| {
        println!("[Guest Plugin Error] Failed to deserialize Election: {}", e);
        e.to_string()
    })?;

    let election_annotations = election.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get election annotations: {}",
            e
        );
        e.to_string()
    })?;

    let Some(area_str) = get_area_by_id(tenant_id, area_id).map_err(|e| e.to_string())? else {
        return Err("Area not found".to_string());
    };

    let area: Area = deserialize_str::<Area>(&area_str).map_err(|e| {
        println!("[Guest Plugin Error] Failed to deserialize Area: {}", e);
        e.to_string()
    })?;

    let area_name = area.name.clone().unwrap_or("".into());
    let area_annotations = area.get_annotations().map_err(|e| {
        println!("[Guest Plugin Error] Failed to get area annotations: {}", e);
        e.to_string()
    })?;

    let tally_session_json =
        get_tally_session_by_id(tenant_id, &election_event.id, tally_session_id).map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to get tally session by id: {}",
                e
            );
            e.to_string()
        })?;

    let tally_session: TallySession = deserialize_str::<TallySession>(&tally_session_json)
        .map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize TallySession: {}",
                e
            );
            e.to_string()
        })?;

    let transmission_data = tally_session.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get tally session annotations: {}",
            e
        );
        e.to_string()
    })?;

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        println!("transmission package not found, skipping");
        return Ok(());
    };

    let Some(miru_document) = get_latest_miru_document(&transmission_area_election.documents)
    else {
        println!("transmission package document not found, skipping");
        return Ok(());
    };

    let document = get_document(
        tenant_id,
        Some(&election_event.id.clone()),
        &miru_document.document_ids.all_servers,
    )
    .map_err(|e| {
        println!("[Guest Plugin Error] Failed to get document by id: {}", e);
        e.to_string()
    })?;

    if document.is_none() {
        return Err(format!(
            "Document with id {} not found",
            &miru_document.document_ids.all_servers
        ));
    }

    let document = document.unwrap();

    let dir_base_path = get_plugin_shared_dir(&Plugins::MIRU);
    let mut compressed_zip_file_name = create_document_as_temp_file(tenant_id, &document)?;
    let compressed_zip_path = PathBuf::from(&dir_base_path).join(&compressed_zip_file_name);

    let unique_dir_name = format!("temp-{}", Uuid::new_v4());
    let zip_output_temp_dir = PathBuf::from(&dir_base_path).join(&unique_dir_name);
    fs::create_dir_all(&zip_output_temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {:?}", e))?;

    unzip_file(compressed_zip_path.as_path(), zip_output_temp_dir.as_path())?;

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
        println!(
            "Can't send to servers as number of signatures {} is less than threshold {}",
            miru_document.signatures.len(),
            transmission_area_election.threshold
        );
        return Ok(());
    }

    for ccs_server in &transmission_area_election.servers {
        if servers_sent_to.contains(&ccs_server.name) {
            println!(
                "SHOULD BE skipping sending to server '{}' as already sent",
                ccs_server.name
            );
            continue;
        }
        let second_zip_folder_path = format!("{}/{}", unique_dir_name, ccs_server.tag);
        let second_zip_path = format!(
            "{}/er_{}.zip",
            second_zip_folder_path, area_annotations.station_id
        );
        match send_package_to_ccs_server(&second_zip_path, ccs_server, false) {
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
                )?;
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
                )?;
            }
        }
        let with_logs = ccs_server.send_logs.clone().unwrap_or_default();
        let logs_zip_path = format!(
            "{}/al_{}.zip",
            second_zip_folder_path, area_annotations.station_id
        );
        if with_logs {
            match send_package_to_ccs_server(&logs_zip_path, ccs_server, true) {
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
                    )?;
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
                    )?;
                }
            }
        }
    }

    match commit_hasura_transaction() {
        Ok(_) => Ok(()),
        Err(e) => return Err(format!("Error committing hasura transaction: {}", e)),
    }
}
