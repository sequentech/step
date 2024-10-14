// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::insert_reports;
use crate::postgres::reports::Report;
use crate::postgres::reports::ReportCronConfig;
use crate::services::protocol_manager::get_event_board;
use crate::services::tasks_execution::update_fail;
use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingPeriodDates;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::core::Template;
use sequent_core::util::mime::get_mime_type;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::Seek;
use std::io::{self, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
use zip::read::ZipArchive;

use super::consolidation::aes_256_cbc_encrypt::decrypt_file_aes_256_cbc;
use super::documents;
use super::documents::upload_and_return_document_postgres;
use super::election_event_board::get_election_event_board;
use super::electoral_log::ElectoralLog;
use super::import_users::import_users_file;
use super::temp_path::get_file_size;
use crate::hasura::election_event::get_election_event;
use crate::hasura::election_event::insert_election_event as insert_election_event_hasura;
use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::postgres;
use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::candidate::insert_candidates;
use crate::postgres::contest::insert_contest;
use crate::postgres::election::insert_election;
use crate::postgres::election_event::insert_election_event;
use crate::postgres::scheduled_event::insert_scheduled_event;
use crate::services::election_event_board::BoardSerializable;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::{
    create_protocol_manager_keys, get_b3_pgsql_client, get_board_client,
};
use crate::services::temp_path::generate_temp_file;
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::types::documents::EDocuments;
use sequent_core::types::hasura::core::{Area, Candidate, Contest, Election, ElectionEvent};
use sequent_core::types::scheduled_event::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportElectionEventSchema {
    pub tenant_id: Uuid,
    pub keycloak_event_realm: Option<RealmRepresentation>,
    pub election_event: ElectionEvent,
    pub elections: Vec<Election>,
    pub contests: Vec<Contest>,
    pub candidates: Vec<Candidate>,
    pub areas: Vec<Area>,
    pub area_contests: Vec<AreaContest>,
    pub scheduled_events: Vec<ScheduledEvent>,
    // pub reports: Vec<Report>,
}

#[instrument(err)]
pub async fn upsert_b3_and_elog(tenant_id: &str, election_event_id: &str) -> Result<Value> {
    let board_name = get_event_board(tenant_id, election_event_id);
    // FIXME must also create the electoral log board here
    let mut immudb_client = get_board_client().await?;
    immudb_client.upsert_electoral_log_db(&board_name).await?;

    let mut board_client = get_b3_pgsql_client().await?;
    let existing = board_client.get_board(board_name.as_str()).await?;
    board_client.create_index_ine().await?;
    board_client.create_board_ine(board_name.as_str()).await?;
    if existing.is_none() {
        event!(
            Level::INFO,
            "creating protocol manager keys for Election event {}",
            election_event_id
        );
        create_protocol_manager_keys(&board_name).await?;
    }
    let board = board_client.get_board(board_name.as_str()).await?;
    let board = board.ok_or(anyhow!(
        "Unexpected error: could not retrieve created board '{}'",
        &board_name
    ))?;

    let board_serializable: BoardSerializable = board.into();

    let board_value = serde_json::to_value(board_serializable.clone())?;
    Ok(board_value)
}

#[instrument(err)]
pub fn read_default_election_event_realm() -> Result<RealmRepresentation> {
    let realm_config_path = env::var("KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH").expect(&format!(
        "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH must be set"
    ));
    let realm_config = fs::read_to_string(&realm_config_path)
        .expect(&format!("Should have been able to read the configuration file in KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH={realm_config_path}"));

    deserialize_str(&realm_config)
        .map_err(|err| anyhow!("Error parsing KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH into RealmRepresentation: {err}"))
}

#[instrument(err, skip(keycloak_event_realm))]
pub async fn upsert_keycloak_realm(
    tenant_id: &str,
    election_event_id: &str,
    keycloak_event_realm: Option<RealmRepresentation>,
) -> Result<()> {
    let realm = if let Some(realm) = keycloak_event_realm.clone() {
        realm
    } else {
        let realm = read_default_election_event_realm()?;
        realm
    };
    let realm_config = serde_json::to_string(&realm)?;
    let client = KeycloakAdminClient::new().await?;
    let realm_name = get_event_realm(tenant_id, election_event_id);
    client
        .upsert_realm(
            realm_name.as_str(),
            &realm_config,
            tenant_id,
            keycloak_event_realm.is_none(),
            None,
        )
        .await?;
    upsert_realm_jwks(realm_name.as_str()).await?;
    Ok(())
}

#[instrument(skip(auth_headers), err)]
pub async fn insert_election_event_db(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> Result<()> {
    let election_event_id = object.id.clone().unwrap();
    let tenant_id = object.tenant_id.clone().unwrap();
    // fetch election_event
    let found_election_event = get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data".into())
    .sequent_backend_election_event;

    if found_election_event.len() > 0 {
        event!(
            Level::INFO,
            "Election event {} for tenant {} already exists",
            election_event_id,
            tenant_id
        );
        return Ok(());
    }

    let new_election_input = InsertElectionEventInput {
        statistics: Some(json!({
            "num_emails_sent": 0,
            "num_sms_sent": 0
        })),
        ..object.clone()
    };

    let _hasura_response =
        insert_election_event_hasura(auth_headers.clone(), new_election_input).await?;

    Ok(())
}

#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    id_opt: Option<String>,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
    let mut keep: Vec<String> = vec![];
    keep.push(original_data.tenant_id.clone().to_string());
    if id_opt.is_some() {
        keep.push(original_data.election_event.id.clone());
    }
    // find other ids to maintain
    if let Some(realm) = original_data.keycloak_event_realm.clone() {
        if let Some(authenticator_configs) = realm.authenticator_config.clone() {
            for authenticator_config in authenticator_configs {
                let Some(config) = authenticator_config.config.clone() else {
                    continue;
                };
                for (_key, value) in config {
                    if Uuid::parse_str(&value).is_ok() {
                        keep.push(value.clone());
                    }
                }
            }
        }
    }

    let mut new_data = replace_uuids(data_str, keep);

    if let Some(id) = id_opt {
        new_data = new_data.replace(&original_data.election_event.id, &id);
    }
    if original_data.tenant_id.to_string() != tenant_id {
        new_data = new_data.replace(&original_data.tenant_id.to_string(), &tenant_id);
    }

    let data: ImportElectionEventSchema = deserialize_str(&new_data)?;
    Ok(data.clone())
}

#[instrument(err, skip_all)]
pub async fn get_document(
    hasura_transaction: &Transaction<'_>,
    object: ImportElectionEventBody,
    election_event_id: Option<String>,
) -> Result<(NamedTempFile, Document, String)> {
    let document = postgres::document::get_document(
        hasura_transaction,
        &object.tenant_id,
        None,
        &object.document_id,
    )
    .await?
    .ok_or(anyhow!(
        "Error trying to get document id {}: not found",
        &object.document_id
    ))?;

    let mut temp_file = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))
        .unwrap();

    let document_type = document
        .clone()
        .media_type
        .unwrap_or("application/ezip".to_string());

    temp_file = decrypt_document(object.password.clone(), temp_file)
        .await
        .map_err(|err| anyhow!("error decrypting document {:?}", document.id))?;

    Ok((temp_file, document, document_type))
}

#[instrument(err, skip_all)]
pub async fn decrypt_document(
    password: Option<String>,
    mut temp_file_path: NamedTempFile,
) -> Result<NamedTempFile> {
    let password = password.unwrap_or_else(|| "".to_string());
    let is_encrypted = !password.is_empty();

    if is_encrypted {
        let decrypted_path = env::temp_dir().join("election-event.zip");

        decrypt_file_aes_256_cbc(
            &temp_file_path.path().to_string_lossy().to_string(),
            &decrypted_path.as_path().to_string_lossy().to_string(),
            &password,
        )
        .map_err(|err| anyhow!("Error generating decrypted file"))?;

        // Create a new NamedTempFile for the decrypted content
        let mut temp_file = NamedTempFile::new()?;
        let content = fs::read(decrypted_path)?;
        temp_file.write_all(&content)?;

        return Ok(temp_file);
    }

    Ok(temp_file_path)
}

#[instrument(err, skip_all)]
pub async fn get_election_event_schema(
    document_type: &String,
    temp_file_path: &NamedTempFile,
    object: ImportElectionEventBody,
    id: Option<String>,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
    if document_type == "application/ezip" || document_type == get_mime_type("zip") {
        // Handle the ZIP file case
        let file = temp_file_path.reopen()?;
        let mut zip = ZipArchive::new(&file)?;

        // Iterate through the files in the ZIP
        for i in 0..zip.len() {
            let mut zip_file = zip.by_index(i)?;
            let zip_file_name = zip_file.name().to_string();

            // Check for the JSON file inside the ZIP
            if zip_file_name.ends_with(".json") {
                let mut json_file_content = String::new();
                zip_file.read_to_string(&mut json_file_content)?;
                let original_data: ImportElectionEventSchema = deserialize_str(&json_file_content)?;

                let data = replace_ids(
                    &json_file_content,
                    &original_data,
                    id.clone(),
                    tenant_id.clone(),
                )?;

                return Ok(data);
            }
        }
        Err(anyhow!("No JSON file found in ZIP"))
    } else {
        // Regular JSON document processing
        let mut file = File::open(temp_file_path)?;
        let mut data_str = String::new();
        file.read_to_string(&mut data_str)?;

        let original_data: ImportElectionEventSchema = deserialize_str(&data_str)?;
        let data = replace_ids(&data_str, &original_data, id, tenant_id.clone())?;

        Ok(data)
    }
}

#[instrument(err, skip_all)]
pub async fn process_election_event_file(
    hasura_transaction: &Transaction<'_>,
    document_type: &String,
    temp_file_path: &NamedTempFile,
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
    let mut data = get_election_event_schema(
        document_type,
        temp_file_path,
        object,
        Some(election_event_id.clone()),
        tenant_id.clone(),
    )
    .await
    .with_context(|| format!("Error getting document for election event ID {election_event_id} and tenant ID {tenant_id}"))?;

    // Upsert immutable board
    let board = upsert_b3_and_elog(tenant_id.as_str(), &election_event_id)
        .await
        .with_context(|| format!("Error upserting b3 board for tenant ID {tenant_id} and election event ID {election_event_id}"))?;

    data.election_event.bulletin_board_reference = Some(board);
    data.election_event.public_key = None;
    data.election_event.statistics = Some(
        serde_json::to_value(ElectionEventStatistics::default())
            .with_context(|| "Error serializing election event statistics")?,
    );

    data.election_event.status = Some(
        serde_json::to_value(ElectionEventStatus::default())
            .with_context(|| "Error serializing election event status")?,
    );

    // Process elections
    data.elections = data
        .elections
        .into_iter()
        .map(|election| -> Result<Election> {
            let mut clone = election.clone();
            clone.statistics = Some(
                serde_json::to_value(ElectionStatistics::default())
                    .with_context(|| "Error serializing election statistics")?,
            );
            clone.status = Some(
                serde_json::to_value(ElectionStatus::default())
                    .with_context(|| "Error serializing election status")?,
            );
            Ok(clone)
        })
        .collect::<Result<Vec<Election>>>()
        .with_context(|| "Error processing elections")?;

    upsert_keycloak_realm(
        tenant_id.as_str(),
        &election_event_id,
        data.keycloak_event_realm.clone(),
    )
    .await
    .with_context(|| format!("Error upserting Keycloak realm for tenant ID {tenant_id} and election event ID {election_event_id}"))?;

    insert_election_event(hasura_transaction, &data)
        .await
        .with_context(|| "Error inserting election event")?;

    manage_dates(&data, hasura_transaction)
        .await
        .with_context(|| "Error managing dates")?;

    insert_election(hasura_transaction, &data)
        .await
        .with_context(|| "Error inserting election")?;

    insert_contest(hasura_transaction, &data)
        .await
        .with_context(|| "Error inserting contest")?;

    insert_candidates(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &data.candidates,
    )
    .await
    .with_context(|| "Error inserting candidates")?;

    insert_areas(hasura_transaction, &data.areas)
        .await
        .with_context(|| "Error inserting areas")?;

    insert_area_contests(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &data.area_contests,
    )
    .await
    .with_context(|| "Error inserting area contests")?;

    // insert_reports(
    //     hasura_transaction,
    //     &tenant_id,
    //     &election_event_id,
    //     &data.reports,
    // )
    // .await
    // .with_context(|| "Error inserting reports")?;

    Ok(data)
}

async fn process_voters_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    file_name: &String,
    election_event_id: Option<String>,
    tenant_id: String,
) -> Result<()> {
    let separator = if file_name.ends_with(".tsv") {
        b'\t'
    } else {
        b','
    };

    import_users_file(
        hasura_transaction,
        temp_file,
        separator,
        election_event_id,
        tenant_id,
    )
    .await
    .map_err(|err| anyhow!("Error importing users file: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_reports_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: String,
    election_event_id: Option<String>,
) -> Result<()> {
    let mut file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let election_event_id =
        election_event_id.ok_or_else(|| anyhow!("Missing election event ID"))?;

    let mut reports = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let report = Report {
            id: Uuid::new_v4().to_string(),
            election_event_id: record
                .get(1)
                .ok_or_else(|| anyhow!("Missing Election Event ID"))?
                .to_string(),
            tenant_id: record
                .get(2)
                .ok_or_else(|| anyhow!("Missing Tenant ID"))?
                .to_string(),
            election_id: record
                .get(3)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty()),
            report_type: record
                .get(4)
                .ok_or_else(|| anyhow!("Missing Report Type"))?
                .to_string(),
            template_id: record
                .get(5)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty()),
            cron_config: record.get(6).and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(ReportCronConfig {
                        email_recipients: Some(s.split(',').map(String::from).collect()),
                        is_active: true, // or false, depending on your logic
                        last_document_produced: None, // or Some(DateTime::<Utc>::now()), depending on your logic
                        cron_expression: Some("your_cron_expression".to_string())
                            .filter(|s| !s.is_empty())
                            .expect("REASON"), // or None, depending on your logic
                    })
                }
            }),
            created_at: record
                .get(7)
                .ok_or_else(|| anyhow!("Missing created_at"))?
                .parse::<DateTime<Utc>>()
                .map_err(|e| anyhow!("Error parsing created_at: {e}"))?,
        };

        reports.push(report);
    }
    info!("Reportssss: {:?}", reports);

    insert_reports(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        &reports,
    )
    .await
    .with_context(|| "Error inserting reports into the database")?;

    Ok(())
}

async fn process_activity_logs_file(
    temp_file: &NamedTempFile,
    election_event: ElectionEvent,
) -> Result<()> {
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "Missing bulletin board")?;

    let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    electoral_log.import_from_csv(temp_file).await?;

    Ok(())
}

pub async fn process_s3_files(
    hasura_transaction: &Transaction<'_>,
    temp_file_path: &NamedTempFile,
    file_name: &str,
    election_event_id: String,
    tenant_id: String,
) -> Result<()> {
    let file_path_string = temp_file_path.path().to_string_lossy().to_string();

    let file_size = get_file_size(file_path_string.as_str())
        .with_context(|| format!("Error obtaining file size for {}", file_path_string))?;

    let file_suffix = Path::new(&file_path_string)
        .extension()
        .unwrap()
        .to_str()
        .unwrap();
    let document_type = get_mime_type(file_suffix);

    // Upload the file and return the document
    let _document = upload_and_return_document_postgres(
        hasura_transaction,
        &file_path_string.clone(),
        file_size,
        &document_type,
        &tenant_id,
        Some(election_event_id.to_string()),
        file_name,
        None,
        false,
    )
    .await?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_document(
    hasura_transaction: &Transaction<'_>,
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
) -> Result<()> {
    let (temp_file_path, document, document_type) = get_document(
        hasura_transaction,
        object.clone(),
        Some(election_event_id.clone()),
    )
    .await
    .map_err(|err| anyhow!("Failed to get document: {err}"))?;

    let election_event_schema = process_election_event_file(
        hasura_transaction,
        &document_type,
        &temp_file_path,
        object,
        election_event_id,
        tenant_id,
    )
    .await
    .map_err(|err| anyhow!("Error processing election event file: {err}"))?;

    // Zip file processing
    if document_type == "application/ezip" || document_type == get_mime_type("zip") {
        let zip_entries = tokio::task::spawn_blocking(move || -> Result<Vec<_>> {
            let file = File::open(&temp_file_path)?;
            let mut zip = ZipArchive::new(file)?;
            let mut entries = Vec::new();
            for i in 0..zip.len() {
                let mut file = zip.by_index(i)?;
                let file_name = file.name().to_string();
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;
                entries.push((file_name, file_contents));
            }
            Ok(entries)
        })
        .await??;

        for (file_name, mut file_contents) in zip_entries {
            info!("Importing file: {:?}", file_name);

            let mut cursor = Cursor::new(&mut file_contents[..]);

            if file_name.contains(&format!("{}", EDocuments::ACTIVITY_LOGS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create activity logs temporary file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of activity logs to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_activity_logs_file(
                    &temp_file,
                    election_event_schema.election_event.clone(),
                )
                .await?;
            }

            if file_name.contains(&format!("{}", EDocuments::VOTERS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create activity logs temporary file")?;
                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of activity logs to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_voters_file(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    Some(election_event_schema.election_event.id.clone()),
                    election_event_schema.tenant_id.to_string(),
                )
                .await?;
            }

            if file_name.contains(&format!("{}", EDocuments::REPORTS.to_file_name())) {
                let mut temp_file =
                    NamedTempFile::new().context("Failed to create reports temporary file")?;
                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of reports to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                // Process the reports file
                process_reports_file(
                    &hasura_transaction,
                    &temp_file,
                    election_event_schema.tenant_id.to_string(),
                    Some(election_event_schema.election_event.id.clone()),
                )
                .await?;
            }

            if file_name.contains(&format!("/{}/", EDocuments::S3_FILES.to_file_name())) {
                let folder_path: Vec<_> = file_name.split("/").collect();
                // Skips the OS created files
                if (folder_path[1] == EDocuments::VOTERS.to_file_name()) {
                    continue;
                }

                // Write the file contents to a new file within this directory
                let mut temp_file =
                    generate_temp_file(&folder_path[1], &folder_path[folder_path.len() - 1])
                        .context("Error generating temp file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy S3 contents to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                // process the directory instead of a single file
                process_s3_files(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    election_event_schema.election_event.id.clone(),
                    election_event_schema.tenant_id.to_string(),
                )
                .await?;
            }
        }
    };

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn manage_dates(
    data: &ImportElectionEventSchema,
    hasura_transaction: &Transaction<'_>,
) -> Result<()> {
    //Manage election event
    let election_event_dates = generate_voting_period_dates(
        data.scheduled_events.clone(),
        data.tenant_id.to_string().as_str(),
        &data.election_event.id,
        None,
    )?;
    if let Some(start_date) = election_event_dates.start_date {
        maybe_create_scheduled_event(
            hasura_transaction,
            data.tenant_id.to_string().as_str(),
            &data.election_event.id,
            EventProcessors::START_VOTING_PERIOD,
            start_date,
            None,
        )
        .await?;
    }
    if let Some(end_date) = election_event_dates.end_date {
        maybe_create_scheduled_event(
            hasura_transaction,
            data.tenant_id.to_string().as_str(),
            &data.election_event.id,
            EventProcessors::END_VOTING_PERIOD,
            end_date,
            None,
        )
        .await?;
    }
    //Manage elections
    let elections = &data.elections;
    for election in elections {
        let dates = generate_voting_period_dates(
            data.scheduled_events.clone(),
            data.tenant_id.to_string().as_str(),
            &data.election_event.id,
            Some(&election.id),
        )?;
        if let Some(start_date) = dates.start_date {
            maybe_create_scheduled_event(
                hasura_transaction,
                data.tenant_id.to_string().as_str(),
                &data.election_event.id,
                EventProcessors::START_VOTING_PERIOD,
                start_date,
                Some(&election.id),
            )
            .await?;
        }
        if let Some(end_date) = dates.end_date {
            maybe_create_scheduled_event(
                hasura_transaction,
                data.tenant_id.to_string().as_str(),
                &data.election_event.id,
                EventProcessors::END_VOTING_PERIOD,
                end_date,
                Some(&election.id),
            )
            .await?;
        }
    }
    Ok(())
}

#[instrument(err, skip_all)]
pub async fn maybe_create_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    event_processor: EventProcessors,
    start_date: String,
    election_id: Option<&str>,
) -> Result<()> {
    let start_task_id =
        generate_manage_date_task_name(tenant_id, election_event_id, election_id, &event_processor);
    let payload = ManageElectionDatePayload {
        election_id: match election_id {
            Some(id) => Some(id.to_string()),
            None => None,
        },
    };
    let cron_config = CronConfig {
        cron: None,
        scheduled_date: Some(start_date.to_string()),
    };
    insert_scheduled_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
        event_processor,
        &start_task_id,
        cron_config,
        serde_json::to_value(payload)?,
    )
    .await?;

    Ok(())
}
