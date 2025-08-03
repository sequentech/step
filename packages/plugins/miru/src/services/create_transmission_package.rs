// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use crate::{
    bindings::plugins_manager::transactions_manager::postgres_queries::{get_area_by_id, get_election_by_id, get_election_event_by_election_area, get_tally_session_by_id}, services::miru_plugin_types::MiruTallySessionData,
};
use sequent_core::{
    ballot::Annotations,
    serialization::deserialize_with_path::deserialize_value,
    types::hasura::core::{Area, Election, ElectionEvent, TallySession},
};
use tracing::instrument;

use super::eml_generator::{
    find_miru_annotation, prepend_miru_annotation, MiruAreaAnnotations, MiruElectionAnnotations,
    MiruElectionEventAnnotations, ValidateAnnotations, MIRU_AREA_CCS_SERVERS, MIRU_AREA_STATION_ID,
    MIRU_AREA_THRESHOLD, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
};

use super::miru_plugin_types::{
    MiruCcsServer, MiruDocument, MiruDocumentIds, MiruTransmissionPackageData,
};

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

    let election_event: ElectionEvent = serde_json::from_str::<ElectionEvent>(&election_event_json)
        .map_err(|e| e.to_string())?;

    // let tally_session_json =
    //     get_tally_session_by_id(tenant_id, &election_event.id, tally_session_id)
    //         .map_err(|e| format!("Failed to get tally session by id: {}", e))?;
    // let tally_session: TallySession = serde_json::from_str(&tally_session_json)
    //     .map_err(|e| format!("Failed to deserialize TallySession: {}", e))?;

    // let tally_annotations: Annotations = tally_session
    //         .annotations
    //         .clone()
    //         .map(|value| deserialize_value(value))
    //         .transpose()
    //         .map_err(|e| e.to_string())?
    //         .unwrap_or_default();

    // let transmission_data = tally_session.get_annotations().unwrap_or(vec![]);

    // let found_package = transmission_data.clone().into_iter().find(|data| {
    //     data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    // });

    // if found_package.is_some() && !force {
    //     // info!("transmission package already found, skipping");
    //     return Ok(());
    // }

    let Some(election_str) =
        get_election_by_id(tenant_id, &election_event.id, election_id).map_err(|e| e.to_string())?
    else {
        // info!("Election not found");
        return Err("Election not found".to_string());
    };

    let election: Election = serde_json::from_str::<Election>(&election_str)
        .map_err(|e| format!("Failed to deserialize ElectionEvent: {}", e))?;
    let election_annotations = election.get_annotations().map_err(|e| e.to_string())?;

    let Some(area_str) =
        get_area_by_id(tenant_id, area_id).map_err(|e| e.to_string())?
    else {
        // info!("Area not found");
        return Err("Area not found".to_string());
    };

    let area: Area = serde_json::from_str::<Area>(&area_str)
        .map_err(|e| format!("Failed to deserialize Area: {}", e))?;

    let area_annotations = area.get_annotations().map_err(|e| e.to_string())?;

    let area_station_id = area_annotations.station_id.clone();

    let threshold = area_annotations.threshold.clone();

    let ccs_servers = area_annotations.ccs_servers.clone();

    // let tar_gz_file = download_tally_tar_gz_to_file(
    //     &hasura_transaction,
    //     tenant_id,
    //     &election_event.id,
    //     tally_session_id,
    // )
    // .await?;

    // let tally_path = decompress_file(tar_gz_file.path())?;

    // let tally_path_path = tally_path.into_path();

    // list_files(&tally_path_path)?;

    // let state = generate_initial_state(&tally_path_path, "decode-ballots")?;

    // let results = state.get_results(true)?;

    // let tally_id = tally_session_id;
    // let transaction_id = generate_transaction_id().to_string();
    // let time_zone = PHILIPPINO_TIMEZONE;
    // let now_utc = Utc::now();
    // let now_local = now_utc.with_timezone(&Local);

    // let election_event_annotations = election_event.get_annotations()?;
    // let Some(result) = results
    //     .into_iter()
    //     .find(|result| result.election_id == election_id)
    // else {
    //     info!("Can't find election report for election {}", election_id);
    //     return Ok(());
    // };
    // let reports: Vec<ReportData> = result
    //     .reports
    //     .into_iter()
    //     .filter(|result| {
    //         let Some(basic_area) = result.area.clone() else {
    //             return false;
    //         };
    //         return basic_area.id == area_id;
    //     })
    //     .map(|report_computed| report_computed.into())
    //     .collect();
    // let (base_compressed_xml, eml, eml_hash) = generate_base_compressed_xml(
    //     tally_id,
    //     &transaction_id,
    //     time_zone.clone(),
    //     now_utc.clone(),
    //     &election_event_annotations,
    //     &election_annotations,
    //     &area_annotations,
    //     &reports,
    // )
    // .await?;

    // // upload .xz
    // let xz_name = format!("er_{}.xz", transaction_id);
    // let (temp_path, temp_path_string, file_size) =
    //     write_into_named_temp_file(&base_compressed_xml, &xz_name, ".xz")?;
    // let xz_document = upload_and_return_document(
    //     &hasura_transaction,
    //     &temp_path_string,
    //     file_size,
    //     "applization/xml",
    //     tenant_id,
    //     Some(election_event.id.to_string()),
    //     &xz_name,
    //     None,
    //     false,
    // )
    // .await?;

    // // upload eml
    // let eml_name = format!("er_{}.xml", transaction_id);
    // let (temp_path, temp_path_string, file_size) =
    //     write_into_named_temp_file(&eml.as_bytes().to_vec(), &eml_name, ".eml")?;
    // let eml_document = upload_and_return_document(
    //     &hasura_transaction,
    //     &temp_path_string,
    //     file_size,
    //     "applization/xml",
    //     tenant_id,
    //     Some(election_event.id.to_string()),
    //     &eml_name,
    //     None,
    //     false,
    // )
    // .await?;

    // let area_name = area.name.clone().unwrap_or("".into());
    // let mut logs = if let Some(package) = found_package {
    //     package.logs.clone()
    // } else {
    //     vec![]
    // };
    // logs.push(create_transmission_package_log(
    //     &now_local,
    //     election_id,
    //     &election.name,
    //     area_id,
    //     &area_name,
    // ));

    // let all_servers_document = generate_all_servers_document(
    //     &hasura_transaction,
    //     &eml_hash,
    //     &eml,
    //     base_compressed_xml.clone(),
    //     &ccs_servers,
    //     &area_annotations,
    //     &election_event_annotations,
    //     &election_event.id,
    //     tenant_id,
    //     time_zone.clone(),
    //     now_utc.clone(),
    //     vec![],
    //     &logs,
    //     &election_annotations,
    // )
    // .await?;

    // let new_transmission_package_data = MiruTransmissionPackageData {
    //     election_id: election_id.to_string(),
    //     area_id: area_id.to_string(),
    //     servers: ccs_servers.clone(),
    //     documents: vec![MiruDocument {
    //         document_ids: MiruDocumentIds {
    //             eml: eml_document.id.clone(),
    //             xz: xz_document.id.clone(),
    //             all_servers: all_servers_document.id.clone(),
    //         },
    //         transaction_id: transaction_id.clone(),
    //         servers_sent_to: vec![],
    //         created_at: ISO8601::to_string(&now_local),
    //         signatures: vec![],
    //     }],
    //     logs,
    //     threshold: threshold,
    // };
    // update_transmission_package_annotations(
    //     &hasura_transaction,
    //     tenant_id,
    //     &election_event.id,
    //     tally_session_id,
    //     area_id,
    //     election_id,
    //     transmission_data.clone(),
    //     new_transmission_package_data,
    //     tally_annotations.clone(),
    // )
    // .await?;

    // hasura_transaction
    //     .commit()
    //     .await
    //     .with_context(|| "error comitting transaction")?;
    Ok(format!(
        "Transmission package created for tenant: {}, election: {}, area: {}, tally session: {}, area_station_id: {:?}",
        tenant_id, election_id, area_id, tally_session_id, area_station_id
    ))
}
