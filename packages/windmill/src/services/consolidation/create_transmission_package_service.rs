use super::acm_transaction::generate_transaction_id;
use super::ecies_encrypt::generate_ecies_key_pair;
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_generator::{
    find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
    MIRU_AREA_STATION_ID, MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA,
};
use super::logs::create_transmission_package_log;
use super::transmission_package::{create_transmission_package, generate_base_compressed_xml};
use super::zip::compress_folder_to_zip;
use crate::postgres::area::get_area_by_id;
use crate::postgres::document::get_document;
use crate::postgres::election::get_election_by_id;
use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session::{get_tally_session_by_id, update_tally_session_annotation};
use crate::postgres::tally_session_execution::get_tally_session_executions;
use crate::services::ceremonies::velvet_tally::generate_initial_state;
use crate::services::compress::decompress_file;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::services::documents::get_document_as_temp_file;
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::folders::list_files;
use crate::services::temp_path::{generate_temp_file, get_file_size, write_into_named_temp_file};
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
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::date_time::TimeZone;
use sequent_core::types::hasura::core::Document;
use sequent_core::util::date_time::get_system_timezone;
use tempfile::{tempdir, NamedTempFile};
use tracing::{info, instrument};
use uuid::Uuid;
use velvet::pipes::generate_reports::ReportData;

#[instrument(skip(hasura_transaction), err)]
pub async fn download_to_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<NamedTempFile> {
    let tally_session_executions = get_tally_session_executions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session executions")?;

    // the first execution is the latest one
    let tally_session_execution = tally_session_executions
        .first()
        .ok_or_else(|| anyhow!("No tally session executions found"))?;

    let results_event_id = tally_session_execution
        .results_event_id
        .clone()
        .ok_or_else(|| anyhow!("Missing results_event_id in tally session execution"))?;

    let results_event = get_results_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await
    .with_context(|| "Error fetching results event")?;

    let document_id = results_event
        .documents
        .ok_or_else(|| anyhow!("Missing documents in results_event"))?
        .tar_gz_original
        .ok_or_else(|| anyhow!("Missing tar_gz_original in results_event"))?;

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
            data.area_id != area_id.to_string() && data.election_id != election_id.to_string()
        })
        .collect();
    new_transmission_data.push(new_transmission_package_data);
    let new_transmission_data_str = serde_json::to_string(&new_transmission_data)?;

    let mut new_tally_annotations = tally_annotations.clone();
    let annotation_key = prepend_miru_annotation(MIRU_TALLY_SESSION_DATA);
    new_tally_annotations.insert(annotation_key, new_transmission_data_str);
    let new_tally_annotations_value = serde_json::to_value(new_tally_annotations)?;

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
    compressed_xml_bytes: Vec<u8>,
    ccs_servers: &Vec<MiruCcsServer>,
    area_station_id: &str,
    election_event_annotations: &Annotations,
    election_event_id: &str,
    tenant_id: &str,
    time_zone: TimeZone,
    now_utc: DateTime<Utc>,
) -> Result<Document> {
    let acm_key_pair = generate_ecies_key_pair()?;
    let temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    let temp_dir_path = temp_dir.path();

    for ccs_server in ccs_servers {
        let server_path = temp_dir_path.join(&ccs_server.address);
        std::fs::create_dir(server_path.clone())
            .with_context(|| format!("Error generating directory {:?}", server_path.clone()))?;
        let zip_file_path = server_path.join(format!("er_{}.zip", area_station_id));
        let transmission_package = create_transmission_package(
            time_zone.clone(),
            now_utc.clone(),
            election_event_annotations,
            compressed_xml_bytes.clone(),
            &acm_key_pair,
            &ccs_server.public_key_pem,
            area_station_id,
            &zip_file_path,
        )
        .await?;
    }

    let dst_file = generate_temp_file("all_servers", ".zip")?;
    let dst_file_path = dst_file.path();
    let dst_file_string = dst_file_path.to_string_lossy().to_string();

    compress_folder_to_zip(temp_dir_path, dst_file.path())?;
    let file_size =
        get_file_size(dst_file_string.as_str()).with_context(|| "Error obtaining file size")?;

    let document = upload_and_return_document_postgres(
        &hasura_transaction,
        &dst_file_string,
        file_size,
        "applization/zip",
        tenant_id,
        election_event_id,
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

    let None = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        info!("transmission package already found, skipping");
        return Ok(());
    };
    let area = get_area_by_id(&hasura_transaction, tenant_id, &area_id)
        .await
        .with_context(|| format!("Error fetching area {}", area_id))?
        .ok_or_else(|| anyhow!("Can't find area {}", area_id))?;
    let area_annotations = area.get_valid_annotations()?;

    let area_station_id = find_miru_annotation(MIRU_AREA_STATION_ID, &area_annotations)
        .with_context(|| {
            format!(
                "Missing area annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_AREA_STATION_ID
            )
        })?;

    let ccs_servers_js = find_miru_annotation(MIRU_AREA_CCS_SERVERS, &area_annotations)
        .with_context(|| {
            format!(
                "Missing area annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_AREA_CCS_SERVERS
            )
        })?;
    let ccs_servers: Vec<MiruCcsServer> =
        deserialize_str(&ccs_servers_js).map_err(|err| anyhow!("{}", err))?;

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
    let election_annotations = election.get_valid_annotations()?;
    let tar_gz_file = download_to_file(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await?;

    let tally_path = decompress_file(tar_gz_file.path())?;

    let tally_path_path = tally_path.into_path();

    list_files(&tally_path_path)?;

    let state = generate_initial_state(&tally_path_path)?;

    let results = state.get_results(true)?;

    let tally_id = tally_session_id;
    let transaction_id = generate_transaction_id().to_string();
    let time_zone = get_system_timezone();
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);

    let election_event_annotations = election_event.get_valid_annotations()?;
    let Some(result) = results
        .into_iter()
        .find(|result| result.election_id == election_id)
    else {
        info!("Can't find election report for election {}", election_id);
        return Ok(());
    };
    let Some(report_computed) = result.reports.into_iter().find(|result| {
        let Some(basic_area) = result.area.clone() else {
            return false;
        };
        return basic_area.id == area_id;
    }) else {
        info!("Can't find election report for area {}", area_id);
        return Ok(());
    };
    let report: ReportData = report_computed.into();
    let base_compressed_xml = generate_base_compressed_xml(
        tally_id,
        &transaction_id,
        time_zone.clone(),
        now_utc.clone(),
        &election_event_annotations,
        &election_annotations,
        &report,
    )
    .await?;

    let name = format!("er_{}", transaction_id);
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&base_compressed_xml, &name, ".xz")?;

    let document = upload_and_return_document_postgres(
        &hasura_transaction,
        &temp_path_string,
        file_size,
        "applization/xml",
        tenant_id,
        &election_event.id,
        &name,
        None,
        false,
    )
    .await?;

    let all_servers_document = generate_all_servers_document(
        &hasura_transaction,
        base_compressed_xml.clone(),
        &ccs_servers,
        &area_station_id,
        &election_event_annotations,
        &election_event.id,
        tenant_id,
        time_zone.clone(),
        now_utc.clone(),
    )
    .await?;

    let area_name = area.name.clone().unwrap_or("".into());
    let new_transmission_package_data = MiruTransmissionPackageData {
        election_id: election_id.to_string(),
        area_id: area_id.to_string(),
        servers: ccs_servers.clone(),
        documents: vec![MiruDocument {
            document_ids: MiruDocumentIds {
                xz: document.id.clone(),
                all_servers: all_servers_document.id.clone(),
            },
            transaction_id: transaction_id.clone(),
            servers_sent_to: vec![],
            created_at: ISO8601::to_string(&now_local),
            signatures: vec![],
        }],
        logs: vec![create_transmission_package_log(
            &now_local,
            election_id,
            &election.name,
            area_id,
            &area_name,
        )],
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
