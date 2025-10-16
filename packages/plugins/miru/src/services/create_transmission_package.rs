// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::convert::Into;
use std::{collections::HashMap, env, fs, hash::Hash};

use crate::{
    bindings::plugins_manager::{
        documents_manager::documents::{
            create_document_as_temp_file, get_tally_results, upload_and_return_document,
        },
        transactions_manager::{
            postgres_queries::{
                get_area_by_id, get_document, get_election_by_id,
                get_election_event_by_election_area, get_last_tally_session_execution,
                get_results_event_by_id, get_tally_session_by_id, update_tally_session_annotation,
            },
            transaction::commit_hasura_transaction,
        },
    },
    services::{
        acm_json::get_acm_key_pair,
        acm_transaction::generate_transaction_id,
        eml_types::ACMTrustee,
        logs::create_transmission_package_log,
        transmission_package::{
            create_logs_package, create_transmission_package, generate_base_compressed_xml,
        },
        zip::compress_folder_to_zip,
    },
};
use chrono::{DateTime, Local, Utc};
use sequent_core::{
    ballot::Annotations,
    plugins::{get_plugin_shared_dir, Plugins},
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    std_temp_path::{create_temp_file, get_file_size, write_into_named_temp_file},
    types::{
        ceremonies::Log,
        date_time::TimeZone,
        hasura::core::{
            Area, Document, Election, ElectionEvent, TallySession, TallySessionExecution,
        },
        results::{ResultDocumentType, ResultsEvent},
        velvet::{ElectionReportDataComputed, ReportData},
    },
    util::date_time::PHILIPPINO_TIMEZONE,
};
use serde_json::Value;
use tracing::instrument;
use wit_bindgen_rt::async_support::futures::TryFutureExt;

use super::eml_generator::{
    find_miru_annotation, prepend_miru_annotation, MiruAreaAnnotations, MiruElectionAnnotations,
    MiruElectionEventAnnotations, ValidateAnnotations, MIRU_AREA_CCS_SERVERS, MIRU_AREA_STATION_ID,
    MIRU_AREA_THRESHOLD, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
};

use super::miru_plugin_types::{
    MiruCcsServer, MiruDocument, MiruDocumentIds, MiruTransmissionPackageData,
};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Entries};
use uuid::Uuid;

pub fn decompress_file(input_file_name: &str) -> Result<(String, PathBuf), String> {
    // --- Setup and File Open ---
    let dir_base_path = get_plugin_shared_dir(&Plugins::MIRU);
    let unique_dir_name = format!("temp-{}", Uuid::new_v4());
    let temp_dir_path = PathBuf::from(&dir_base_path).join(&unique_dir_name);
    fs::create_dir_all(&temp_dir_path)
        .map_err(|e| format!("Failed to create temporary directory: {}", e))?;

    // let temp_dir_path_buf = temp_dir.path().to_path_buf();

    // let output_path = temp_dir_path.as_path();
    // println!("[Guest Plugin] Created temporary directory at: {}", output_path.display());

    let input_dir = PathBuf::from(&dir_base_path);
    let input_path = input_dir.join(input_file_name);
    let file = File::open(&input_path).map_err(|e| e.to_string())?;

    println!(
        "[Guest Plugin] Opened file for decompression: {:?}",
        file.metadata()
    );

    println!("[Guest Plugin] Starting to decompress archive into directory...");

    // let output_path = temp_dir_path.clone(); // Clone the PathBuf

    println!("[Guest Plugin] use new method to decompress");

    let mut archive = Archive::new(file); // Archive takes ownership of file

    let entries = archive.entries().map_err(|e| {
        println!("[Guest Plugin] Error reading archive entries: {}", e);
        format!("Error reading archive entries: {}", e)
    })?;

    for entry_result in entries {
        let mut entry = entry_result.map_err(|e| {
            println!("[Guest Plugin Error] Error reading next entry: {}", e);
            format!("Error reading next entry: {}", e)
        })?;

        // Use the entry header path for the destination file/directory name
        let entry_path = entry.path().map_err(|e| {
            println!(
                "[Guest Plugin Error] Error extracting path from tar entry: {}",
                e
            );
            format!("Error extracting path from tar entry: {}", e)
        })?;

        // Construct the full path within the destination directory.
        // The previously selected line `let temp_dir_path = base_path.join(&unique_dir_name);`
        // ensures `dest_dir_path` is a clean, unique base, making this join safe.
        let file_path = temp_dir_path.join(&entry_path);

        if entry.header().entry_type().is_dir() {
            // Manually create directories.
            fs::create_dir_all(&file_path)
                .map_err(|e| format!("Failed to create directory {:?}: {}", file_path, e))?;
        } else if entry.header().entry_type().is_file() {
            // Ensure parent directories exist before writing the file.
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create parent directory {:?}: {}", parent, e)
                })?;
            }

            // Copy the file contents, ignoring complex metadata and permissions.
            let mut dest_file = File::create(&file_path).map_err(|e| {
                format!(
                    "[Guest Plugin Error] Failed to create file {:?}: {}",
                    file_path, e
                )
            })?;

            io::copy(&mut entry, &mut dest_file).map_err(|e| {
                format!(
                    "[Guest Plugin Error] Failed to copy data for file {:?}: {}",
                    file_path, e
                )
            })?;
        }
        // Symlinks and hard links are ignored as they often fail in Wasm/WASI sandboxes.
    }

    println!(
        "Decompressed file to temporary directory at: {}",
        temp_dir_path.display()
    );
    Ok((unique_dir_name, temp_dir_path))
}
fn list_all_temp_files_directly(dir: &PathBuf) -> Result<(), String> {
    println!(
        "[Guest Plugin] Listing all files recursively in directory: {:?}",
        dir.display()
    );

    let mut file_names = Vec::new();
    let mut dirs_to_visit = vec![dir.clone()];

    while let Some(current_dir) = dirs_to_visit.pop() {
        match fs::read_dir(&current_dir) {
            Ok(entries) => {
                for entry in entries {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            return Err(format!(
                                "[Guest Plugin] Error reading directory entry: {}",
                                e
                            ))
                        }
                    };

                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name() {
                            file_names.push(path.to_string_lossy().into_owned());
                        }
                    } else if path.is_dir() {
                        dirs_to_visit.push(path);
                    }
                }
            }
            Err(e) => {
                return Err(format!(
                    "[Guest Plugin] Failed to read directory {:?}: {}",
                    current_dir, e
                ))
            }
        }
    }

    Ok(())
}

pub fn download_tally_tar_gz_to_file(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<String, String> {
    let Some(tally_session_execution_json) =
        get_last_tally_session_execution(tenant_id, election_event_id, tally_session_id)
            .map_err(|e| e.to_string())?
    else {
        return Err("Tally session execution not found".to_string());
    };

    let tally_session_execution: TallySessionExecution =
        deserialize_str::<TallySessionExecution>(&tally_session_execution_json)
            .map_err(|e| e.to_string())?;

    let results_event_id = tally_session_execution
        .results_event_id
        .clone()
        .ok_or_else(|| format!("Missing results_event_id in tally session execution"))?;

    let result_event_json =
        get_results_event_by_id(tenant_id, election_event_id, &results_event_id)
            .map_err(|e| e.to_string())?;

    let result_event: ResultsEvent =
        deserialize_str::<ResultsEvent>(&result_event_json).map_err(|e| e.to_string())?;

    let document_type = ResultDocumentType::TarGzOriginal;
    let document_id = result_event
        .documents
        .ok_or_else(|| format!("Missing documents in results_event"))?
        .get_document_by_type(&document_type)
        .ok_or_else(|| format!("Missing {:?} in results_event", document_type))?;

    let document = get_document(tenant_id, Some(election_event_id), &document_id)
        .map_err(|e| e.to_string())?;

    if document.is_none() {
        return Err(format!("Document with id {} not found", document_id));
    }

    let document = document.unwrap();

    let tally_tr_gz_file_name =
        create_document_as_temp_file(&tenant_id, &document).map_err(|e| e.to_string())?;
    println!(
        "[Guest Plugin] Document created at: {}",
        tally_tr_gz_file_name
    );

    Ok(tally_tr_gz_file_name)
}

#[instrument(skip_all, err)]
pub fn create_transmission_package_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    force: bool,
) -> Result<String, String> {
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

    println!(
        "[Guest Plugin] Successfully retrieved tally session: {:?}",
        tally_session
    );

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

    let transmission_data = tally_session.get_annotations().unwrap_or(vec![]);

    let found_package = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    });

    if found_package.is_some() && !force {
        // info!("transmission package already found, skipping");
        println!("[Guest Plugin] Transmission package already found, skipping");
        return Ok("Transmission package already found".to_string());
    }

    let Some(election_str) = get_election_by_id(tenant_id, &election_event.id, election_id)
        .map_err(|e| e.to_string())?
    else {
        // info!("Election not found");
        return Err("Election not found".to_string());
    };

    let election: Election = deserialize_str::<Election>(&election_str).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to deserialize ElectionEvent: {}",
            e
        );
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
        // info!("Area not found");
        return Err("Area not found".to_string());
    };

    let area: Area = deserialize_str::<Area>(&area_str).map_err(|e| {
        println!("[Guest Plugin Error] Failed to deserialize Area: {}", e);
        e.to_string()
    })?;

    let area_annotations = area.get_annotations().map_err(|e| {
        println!("[Guest Plugin Error] Failed to get area annotations: {}", e);
        e.to_string()
    })?;

    let area_station_id = area_annotations.station_id.clone();

    let threshold = area_annotations.threshold.clone();

    let ccs_servers = area_annotations.ccs_servers.clone();

    let tally_tr_gz_file_name =
        download_tally_tar_gz_to_file(tenant_id, &election_event.id, &tally_session.id)
            .map_err(|e| e.to_string())?;

    println!(
        "[Guest Plugin] Downloaded tally .tar.gz file: {}",
        tally_tr_gz_file_name
    );
    let (tally_file_name, tally_path) = decompress_file(&tally_tr_gz_file_name)?;

    println!(
        "[Guest Plugin] After decompression, tally file name: {}",
        tally_file_name
    );

    list_all_temp_files_directly(&tally_path)?;

    let tally_results_str = get_tally_results(&tally_file_name).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get tally results from decompressed file: {}",
            e
        );
        e.to_string()
    })?;

    let tally_results: Vec<ElectionReportDataComputed> =
        deserialize_str::<Vec<ElectionReportDataComputed>>(&tally_results_str).map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize tally results: {}",
                e
            );
            e.to_string()
        })?;

    println!(
        "[Guest Plugin] Retrieved tally results: {} entries",
        tally_results.len()
    );

    let tally_id = tally_session_id;
    let transaction_id = generate_transaction_id().to_string();
    let time_zone = PHILIPPINO_TIMEZONE.clone();
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);

    let election_event_annotations = election_event
        .get_annotations()
        .map_err(|e| e.to_string())?;
    let Some(result) = tally_results
        .into_iter()
        .find(|result| result.election_id == election_id)
    else {
        println!(
            "[Guest Plugin] Tally result not found for election_id: {}",
            election_id
        );
        return Ok("".to_string());
    };
    let reports: Vec<ReportData> = result
        .reports
        .into_iter()
        .filter(|result| {
            let Some(basic_area) = result.area.clone() else {
                return false;
            };
            return basic_area.id == area_id;
        })
        .map(|report_computed| report_computed.into())
        .collect();

    println!(
        "[Guest Plugin] Filtered reports for area_id {}: {} reports",
        area_id,
        reports.len()
    );

    let (base_compressed_xml, eml, eml_hash) = generate_base_compressed_xml(
        tally_id,
        &transaction_id,
        time_zone.clone(),
        now_utc.clone(),
        &election_event_annotations,
        &election_annotations,
        &area_annotations,
        &reports,
    )
    .map_err(|e| e.to_string())?;

    println!(
        "[Guest Plugin] Generated base compressed XML and EML, sizes: {} bytes (XML), {} bytes (EML)",
        base_compressed_xml.len(),
        eml.len()
    );

    let dir_base_path = get_plugin_shared_dir(&Plugins::MIRU);

    // upload .xz
    let xz_name = format!("er_{}.xz", transaction_id);
    let (_temp_file, xz_file_name, _temp_path_string, file_size) =
        write_into_named_temp_file(&base_compressed_xml, &xz_name, ".xz", &dir_base_path)?;
    let xz_document_str = upload_and_return_document(
        file_size,
        "applization/xml",
        tenant_id,
        Some(election_event.id.as_ref()),
        &xz_file_name,
        None,
        false,
    )?;
    let xz_document = deserialize_str::<Document>(&xz_document_str).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to deserialize XZ Document: {}",
            e
        );
        e.to_string()
    })?;

    // upload eml
    let eml_name = format!("er_{}.xml", transaction_id);
    let (_temp_file, eml_file_name, _temp_path_string, file_size) =
        write_into_named_temp_file(&eml.as_bytes().to_vec(), &eml_name, ".eml", &dir_base_path)?;
    let eml_document_str = upload_and_return_document(
        file_size,
        "applization/xml",
        tenant_id,
        Some(election_event.id.as_ref()),
        &eml_file_name,
        None,
        false,
    )?;
    let eml_document = deserialize_str::<Document>(&eml_document_str).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to deserialize EML Document: {}",
            e
        );
        e.to_string()
    })?;

    let area_name = area.name.clone().unwrap_or("".into());
    let mut logs = if let Some(package) = found_package {
        package.logs.clone()
    } else {
        vec![]
    };
    logs.push(create_transmission_package_log(
        &now_local,
        election_id,
        &election.name,
        area_id,
        &area_name,
    ));

    let all_servers_document = generate_all_servers_document(
        &eml_hash,
        &eml,
        base_compressed_xml.clone(),
        &ccs_servers,
        &area_annotations,
        &election_event_annotations,
        &election_event.id,
        tenant_id,
        time_zone.clone(),
        now_utc.clone(),
        vec![],
        &logs,
        &election_annotations,
        &dir_base_path,
    )?;

    let new_transmission_package_data = MiruTransmissionPackageData {
        election_id: election_id.to_string(),
        area_id: area_id.to_string(),
        servers: ccs_servers.clone(),
        documents: vec![MiruDocument {
            document_ids: MiruDocumentIds {
                eml: eml_document.id.clone(),
                xz: xz_document.id.clone(),
                all_servers: all_servers_document.id.clone(),
            },
            transaction_id: transaction_id.clone(),
            servers_sent_to: vec![],
            created_at: now_local.to_rfc3339(),
            signatures: vec![],
        }],
        logs,
        threshold: threshold,
    };
    update_transmission_package_annotations(
        tenant_id,
        &election_event.id,
        tally_session_id,
        area_id,
        election_id,
        transmission_data.clone(),
        new_transmission_package_data,
        tally_annotations.clone(),
    )?;

    match commit_hasura_transaction() {
        Ok(_) => Ok(("Transmission package created successfully".to_string())),
        Err(e) => return Err(format!("Error creating hasura transaction: {}", e)),
    }
}

#[instrument(skip_all, err)]
pub fn generate_all_servers_document(
    eml_hash: &str,
    eml: &str,
    compressed_xml_bytes: Vec<u8>,
    ccs_servers: &Vec<MiruCcsServer>,
    area_annotations: &MiruAreaAnnotations,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_event_id: &str,
    tenant_id: &str,
    time_zone: TimeZone,
    now_utc: DateTime<Utc>,
    server_signatures: Vec<ACMTrustee>,
    logs: &Vec<Log>,
    election_annotations: &MiruElectionAnnotations,
    dir_base_path: &str,
) -> Result<Document, String> {
    println!("[Guest Plugin] Generating all servers document");
    let acm_key_pair = get_acm_key_pair(tenant_id, election_event_id).map_err(|e| e.to_string())?;

    println!(
        "[Guest Plugin] Retrieved ACM key pair with public key: {}",
        acm_key_pair.public_key_pem
    );

    let temp_dir_path = PathBuf::new();

    println!(
        "[Guest Plugin] Using temporary directory for server packages: {:?}",
        temp_dir_path.display()
    );

    fs::create_dir_all(&temp_dir_path).map_err(|e| e.to_string())?;

    for ccs_server in ccs_servers {
        let server_path = temp_dir_path.join(&ccs_server.tag);
        std::fs::create_dir(server_path.clone()).map_err(|e| {
            format!(
                "Error generating directory {:?}: {}",
                server_path.clone(),
                e
            )
        })?;
        let zip_file_path = server_path.join(format!("er_{}.zip", area_annotations.station_id));
        println!(
            "[Guest Plugin] Creating transmission package for server: {}",
            ccs_server.tag
        );
        create_transmission_package(
            eml_hash,
            eml,
            time_zone.clone(),
            now_utc.clone(),
            election_event_annotations,
            compressed_xml_bytes.clone(),
            &acm_key_pair,
            &ccs_server.public_key_pem,
            area_annotations,
            &zip_file_path,
            &server_signatures,
            &election_annotations,
            dir_base_path,
        )?;

        let with_logs = ccs_server.send_logs.clone().unwrap_or_default();
        if with_logs {
            let zip_file_path = server_path.join(format!("al_{}.zip", area_annotations.station_id));
            create_logs_package(
                time_zone.clone(),
                now_utc.clone(),
                election_event_annotations,
                &election_annotations,
                &acm_key_pair,
                &ccs_server.public_key_pem,
                area_annotations,
                &zip_file_path,
                &server_signatures,
                logs,
                dir_base_path,
            )?;
        }
    }

    let (dst_file, dst_file_name) = create_temp_file("all_servers", ".zip", dir_base_path)?;
    let dst_file_path = dst_file.path();
    let dst_file_string = dst_file_path.to_string_lossy().to_string();

    compress_folder_to_zip(temp_dir_path.as_path(), dst_file_path)?;
    let file_size =
        get_file_size(dst_file_path).map_err(|e| format!("Error obtaining file size: {}", e))?;

    let document_str = upload_and_return_document(
        file_size,
        "applization/zip",
        tenant_id,
        Some(election_event_id),
        &dst_file_name,
        None,
        false,
    )?;

    let document = deserialize_str::<Document>(&document_str)
        .map_err(|e| format!("Failed to deserialize Document: {}", e))?;

    Ok(document)
}

#[instrument(err)]
pub fn update_transmission_package_annotations(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    area_id: &str,
    election_id: &str,
    transmission_data: Vec<MiruTransmissionPackageData>,
    new_transmission_package_data: MiruTransmissionPackageData,
    tally_annotations: Annotations,
) -> Result<(), String> {
    let mut new_transmission_data: Vec<MiruTransmissionPackageData> = transmission_data
        .clone()
        .into_iter()
        .filter(|data| {
            data.area_id != area_id.to_string() || data.election_id != election_id.to_string()
        })
        .collect();
    new_transmission_data.push(new_transmission_package_data);
    let new_transmission_data_str = serde_json::to_string(&new_transmission_data)
        .map_err(|e| format!("Error serializing new transmission data: {}", e))?;

    let mut new_tally_annotations = tally_annotations.clone();
    let annotation_key = prepend_miru_annotation(MIRU_TALLY_SESSION_DATA);
    new_tally_annotations.insert(annotation_key, new_transmission_data_str);

    let new_tally_annotations_str = serde_json::to_string(&new_tally_annotations)
        .map_err(|e| format!("Error serializing new tally annotations map: {}", e))?;

    println!(
        "Updating tally session annotations: {}",
        new_tally_annotations_str
    );

    update_tally_session_annotation(
        tenant_id,
        election_event_id,
        tally_session_id,
        &new_tally_annotations_str,
    )?;

    Ok(())
}
