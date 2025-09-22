// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::acm_json::get_acm_key_pair;
use super::acm_transaction::generate_transaction_id;
use super::eml_generator::{
    find_miru_annotation, prepend_miru_annotation, MiruAreaAnnotations, MiruElectionAnnotations,
    MiruElectionEventAnnotations, ValidateAnnotations, MIRU_AREA_CCS_SERVERS, MIRU_AREA_STATION_ID,
    MIRU_AREA_THRESHOLD, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
};
use super::logs::create_transmission_package_log;
use super::transmission_package::{
    create_logs_package, create_transmission_package, generate_base_compressed_xml,
};
use super::zip::compress_folder_to_zip;
use crate::postgres::area::get_area_by_id;
use crate::postgres::document::get_document;
use crate::postgres::election::get_election_by_id;
use crate::postgres::results_election::get_results_election_by_results_event_id;
use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session::{get_tally_session_by_id, update_tally_session_annotation};
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::ceremonies::velvet_tally::generate_initial_state;
use crate::services::compress::extract_archive_to_temp_dir;
use crate::services::consolidation::eml_types::ACMTrustee;
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use crate::services::documents::upload_and_return_document;
use crate::services::folders::list_files;
use crate::types::miru_plugin::{
    MiruCcsServer, MiruDocument, MiruDocumentIds, MiruTransmissionPackageData,
};
use crate::{
    postgres::election_event::get_election_event_by_election_area,
    types::miru_plugin::MiruTallySessionData,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::signatures::ecies_encrypt::generate_ecies_key_pair;
use sequent_core::types::ceremonies::Log;
use sequent_core::types::date_time::TimeZone;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::results::{ResultDocumentType, ResultDocuments};
use sequent_core::util::date_time::PHILIPPINO_TIMEZONE;
use sequent_core::util::temp_path::*;
use tempfile::{tempdir, NamedTempFile};
use tracing::{info, instrument};
use uuid::Uuid;
use velvet::pipes::generate_reports::ReportData;

#[instrument(skip(hasura_transaction), err)]
pub async fn download_tally_tar_gz_to_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<NamedTempFile> {
    let tally_session_execution = get_last_tally_session_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session executions")?
    .ok_or(anyhow!("No tally session execution found"))?;

    let results_event_id = tally_session_execution
        .results_event_id
        .clone()
        .ok_or_else(|| anyhow!("Missing results_event_id in tally session execution"))?;

    let result_event = get_results_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await
    .with_context(|| "Error fetching results event")?;

    let document_type = ResultDocumentType::TarGzOriginal;
    let document_id = result_event
        .documents
        .ok_or_else(|| anyhow!("Missing documents in results_event"))?
        .get_document_by_type(&document_type)
        .ok_or_else(|| anyhow!(format!("Missing {:?} in results_event", document_type)))?;

    let document = get_document(
        hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        &document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", document_id))?;

    get_document_as_temp_file(tenant_id, &document).await
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_transmission_package_annotations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    area_id: &str,
    election_id: &str,
    transmission_data: Vec<MiruTransmissionPackageData>,
    new_transmission_package_data: MiruTransmissionPackageData,
    tally_annotations: Annotations,
) -> Result<()> {
    let mut new_transmission_data: Vec<MiruTransmissionPackageData> = transmission_data
        .clone()
        .into_iter()
        .filter(|data| {
            data.area_id != area_id.to_string() || data.election_id != election_id.to_string()
        })
        .collect();
    new_transmission_data.push(new_transmission_package_data);
    let new_transmission_data_str = serde_json::to_string(&new_transmission_data)?;

    let mut new_tally_annotations = tally_annotations.clone();
    let annotation_key = prepend_miru_annotation(MIRU_TALLY_SESSION_DATA);
    new_tally_annotations.insert(annotation_key, new_transmission_data_str);
    let new_tally_annotations_value = serde_json::to_value(new_tally_annotations)?;

    info!(
        "Updating tally session annotations: {}",
        new_tally_annotations_value
    );

    update_tally_session_annotation(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
        new_tally_annotations_value,
    )
    .await?;

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn generate_all_servers_document(
    hasura_transaction: &Transaction<'_>,
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
) -> Result<Document> {
    let acm_key_pair = get_acm_key_pair(hasura_transaction, tenant_id, election_event_id).await?;
    let temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    let temp_dir_path = temp_dir.path();

    for ccs_server in ccs_servers {
        let server_path = temp_dir_path.join(&ccs_server.tag);
        std::fs::create_dir(server_path.clone())
            .with_context(|| format!("Error generating directory {:?}", server_path.clone()))?;
        let zip_file_path = server_path.join(format!("er_{}.zip", area_annotations.station_id));
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
        )
        .await?;
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
            )
            .await?;
        }
    }

    let dst_file = generate_temp_file("all_servers", ".zip")?;
    let dst_file_path = dst_file.path();
    let dst_file_string = dst_file_path.to_string_lossy().to_string();

    compress_folder_to_zip(temp_dir_path, dst_file.path())?;
    let file_size =
        get_file_size(dst_file_string.as_str()).with_context(|| "Error obtaining file size")?;

    let document = upload_and_return_document(
        &hasura_transaction,
        &dst_file_string,
        file_size,
        "applization/zip",
        tenant_id,
        Some(election_event_id.to_string()),
        "all_servers.zip",
        None,
        false,
    )
    .await?;

    Ok(document)
}

#[instrument(err)]
pub async fn create_transmission_package_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    force: bool,
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

    let tally_annotations: Annotations = tally_session
        .annotations
        .clone()
        .map(|value| deserialize_value(value))
        .transpose()?
        .unwrap_or_default();

    let transmission_data: MiruTallySessionData = tally_session.get_annotations().unwrap_or(vec![]);

    let found_package = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    });

    if found_package.is_some() && !force {
        info!("transmission package already found, skipping");
        return Ok(());
    }

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
    let area_annotations = area.get_annotations()?;

    let area_station_id = area_annotations.station_id.clone();

    let threshold = area_annotations.threshold.clone();

    let ccs_servers = area_annotations.ccs_servers.clone();

    let tar_gz_file = download_tally_tar_gz_to_file(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await?;

    let tally_path = extract_archive_to_temp_dir(tar_gz_file.path(), false)?;

    let tally_path_path = tally_path.into_path();

    list_files(&tally_path_path)?;

    let state = generate_initial_state(&tally_path_path, "decode-ballots")?;

    let results = state.get_results(true)?;

    let tally_id = tally_session_id;
    let transaction_id = generate_transaction_id().to_string();
    let time_zone = PHILIPPINO_TIMEZONE;
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);

    let election_event_annotations = election_event.get_annotations()?;
    let Some(result) = results
        .into_iter()
        .find(|result| result.election_id == election_id)
    else {
        info!("Can't find election report for election {}", election_id);
        return Ok(());
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
    .await?;

    // upload .xz
    let xz_name = format!("er_{}.xz", transaction_id);
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&base_compressed_xml, &xz_name, ".xz")?;
    let xz_document = upload_and_return_document(
        &hasura_transaction,
        &temp_path_string,
        file_size,
        "applization/xml",
        tenant_id,
        Some(election_event.id.to_string()),
        &xz_name,
        None,
        false,
    )
    .await?;

    // upload eml
    let eml_name = format!("er_{}.xml", transaction_id);
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&eml.as_bytes().to_vec(), &eml_name, ".eml")?;
    let eml_document = upload_and_return_document(
        &hasura_transaction,
        &temp_path_string,
        file_size,
        "applization/xml",
        tenant_id,
        Some(election_event.id.to_string()),
        &eml_name,
        None,
        false,
    )
    .await?;

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
        &hasura_transaction,
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
    )
    .await?;

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
            created_at: ISO8601::to_string(&now_local),
            signatures: vec![],
        }],
        logs,
        threshold: threshold,
    };
    update_transmission_package_annotations(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
        area_id,
        election_id,
        transmission_data.clone(),
        new_transmission_package_data,
        tally_annotations.clone(),
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
