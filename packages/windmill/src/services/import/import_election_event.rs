// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::application::insert_applications;
use crate::postgres::election_event::{get_election_event_by_id_if_exist, update_bulletin_board};
use crate::postgres::reports::insert_reports;
use crate::postgres::reports::Report;
use crate::postgres::trustee::get_all_trustees;
use crate::services::import::import_publications::import_ballot_publications;
use crate::services::import::import_scheduled_events::import_scheduled_events;
use crate::services::protocol_manager::get_event_board;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_key_pair;
use crate::services::tasks_execution::update_fail;
use crate::tasks::insert_election_event::CreateElectionEventInput;
use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::future::try_join_all;
use sequent_core::ballot::AllowTallyStatus;
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::PeriodDates;
use sequent_core::ballot::VotingPeriodDates;
use sequent_core::ballot::VotingStatus;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::mime::{get_mime_types, matches_mime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::Seek;
use std::io::{self, Read, Write};
use std::path::Path;
use std::str::FromStr;
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
use zip::read::ZipArchive;

use super::import_users::import_users_file;
use crate::postgres;
use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::candidate::insert_candidates;
use crate::postgres::contest::insert_contest;
use crate::postgres::election::insert_elections;
use crate::postgres::election_event::insert_election_event;
use crate::postgres::keys_ceremony;
use crate::postgres::scheduled_event::insert_scheduled_event;
use crate::services::consolidation::aes_256_cbc_encrypt::decrypt_file_aes_256_cbc;
use crate::services::documents;
use crate::services::documents::upload_and_return_document;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_board::BoardSerializable;
use crate::services::electoral_log::ElectoralLog;
use crate::services::import::import_bulletin_boards::*;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::get_election_board;
use crate::services::protocol_manager::get_protocol_manager_secret_path;
use crate::services::protocol_manager::{
    create_protocol_manager_keys, get_b3_pgsql_client, get_board_client,
};
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::types::documents::EDocuments;
use sequent_core::types::hasura::core::{Area, Candidate, Contest, Election, ElectionEvent};
use sequent_core::types::scheduled_event::*;
use sequent_core::util::temp_path::{generate_temp_file, get_file_size};

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
    pub scheduled_events: Option<Vec<ScheduledEvent>>,
    pub reports: Vec<Report>,
    pub keys_ceremonies: Option<Vec<KeysCeremony>>,
    pub applications: Option<Vec<Application>>,
}

#[instrument(err)]
pub async fn upsert_b3_and_elog(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
    dont_auto_generate_keys: bool, // avoid creating protocol manager keys
) -> Result<Value> {
    let board_name = get_event_board(tenant_id, election_event_id);
    // FIXME must also create the electoral log board here
    let mut immudb_client = get_board_client().await?;
    immudb_client.upsert_electoral_log_db(&board_name).await?;

    let mut board_client = get_b3_pgsql_client().await?;

    // Create board and protocol manager keys for election event (assert)
    let existing: Option<b3::client::pgsql::B3IndexRow> =
        board_client.get_board(board_name.as_str()).await?;
    // insert into the index of boards
    board_client.create_index_ine().await?;
    // create board table
    board_client.create_board_ine(board_name.as_str()).await?;

    if existing.is_none() && !dont_auto_generate_keys {
        event!(
            Level::INFO,
            "creating protocol manager keys for Election event {}",
            election_event_id
        );
        create_protocol_manager_keys(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &board_name,
        )
        .await?;
    }

    // board was created, checking it is now present
    let board = board_client
        .get_board(board_name.as_str())
        .await?
        .ok_or(anyhow!(
            "Unexpected error: could not retrieve created board '{}'",
            &board_name
        ))?;

    for election_id in election_ids.clone() {
        // Create board and protocol manager keys for election (insert, not asssert)
        let board_name = get_election_board(tenant_id, &election_id);

        let existing: Option<b3::client::pgsql::B3IndexRow> =
            board_client.get_board(board_name.as_str()).await?;

        // assert board table
        board_client.create_board_ine(board_name.as_str()).await?;
        // create board table

        if existing.is_none() && !dont_auto_generate_keys {
            event!(
                Level::INFO,
                "creating protocol manager keys for election {}",
                election_id
            );
            create_protocol_manager_keys(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &board_name,
            )
            .await?;
        }
        // board was created, checking it is now present
        board_client
            .get_board(board_name.as_str())
            .await?
            .ok_or(anyhow!(
                "Unexpected error: could not retrieve created board '{}'",
                &board_name
            ))?;
    }

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
            Some(election_event_id.to_string()),
        )
        .await?;
    upsert_realm_jwks(realm_name.as_str()).await?;
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_election_event_db(
    hasura_transaction: &Transaction<'_>,
    object: &CreateElectionEventInput,
) -> Result<()> {
    let election_event_id = object.id.clone().unwrap();
    let tenant_id = object.tenant_id.clone();
    // fetch election_event
    let found_election_event = get_election_event_by_id_if_exist(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
    )
    .await?;

    if found_election_event.is_some() {
        event!(
            Level::INFO,
            "Election event {} for tenant {} already exists",
            election_event_id,
            tenant_id
        );
        return Ok(());
    }

    let new_election_input = ElectionEvent {
        id: object.id.clone().unwrap(),
        tenant_id: object.tenant_id.clone(),
        name: object.name.clone(),
        description: object.description.clone(),
        public_key: object.public_key.clone(),
        status: object.status.clone(),
        created_at: None,
        updated_at: None,
        labels: object.labels.clone(),
        annotations: object.annotations.clone(),
        presentation: object.presentation.clone(),
        bulletin_board_reference: object.bulletin_board_reference.clone(),
        is_archived: object.is_archived.unwrap_or(false),
        voting_channels: object.voting_channels.clone(),
        user_boards: object.user_boards.clone(),
        encryption_protocol: object
            .encryption_protocol
            .clone()
            .unwrap_or("RSA256".to_string()),
        is_audit: object.is_audit.clone(),
        audit_election_event_id: object.audit_election_event_id.clone(),
        alias: object.alias.clone(),
        statistics: Some(json!({
            "num_emails_sent": 0,
            "num_sms_sent": 0
        })),
    };

    insert_election_event(&hasura_transaction, &new_election_input).await?;
    Ok(())
}

#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    id_opt: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
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

    let (mut new_data, replacement_map) = replace_uuids(data_str, keep);

    if let Some(id) = id_opt {
        new_data = new_data.replace(&original_data.election_event.id, &id);
    }
    if original_data.tenant_id.to_string() != tenant_id {
        new_data = new_data.replace(&original_data.tenant_id.to_string(), &tenant_id);
    }

    let data: ImportElectionEventSchema = deserialize_str(&new_data)?;
    Ok((data, replacement_map))
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
    data_str: &str,
    id: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    let original_data: ImportElectionEventSchema = deserialize_str(data_str)?;
    replace_ids(data_str, &original_data, id, tenant_id.clone())
}

#[instrument(err, skip_all)]
pub async fn process_election_event_file(
    hasura_transaction: &Transaction<'_>,
    document_type: &String,
    file_election_event_schema: &str,
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
    is_importing_keys: bool,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    let (mut data, replacement_map) = get_election_event_schema(
        file_election_event_schema,
        Some(election_event_id.clone()),
        tenant_id.clone(),
    )
    .await
    .with_context(|| format!("Error getting document for election event ID {election_event_id} and tenant ID {tenant_id}"))?;

    let election_ids: Vec<String> = data
        .elections
        .clone()
        .into_iter()
        .map(|election| election.id.clone())
        .collect();

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

            let mut status: ElectionStatus = clone
                .status
                .clone()
                .map(|value| deserialize_value::<ElectionStatus>(value))
                .transpose()
                .unwrap_or_default()
                .unwrap_or_default();

            status.voting_status = VotingStatus::default();
            status.kiosk_voting_status = VotingStatus::default();
            status.voting_period_dates = PeriodDates::default();
            status.kiosk_voting_period_dates = PeriodDates::default();

            clone.status = Some(
                serde_json::to_value(status)
                    .with_context(|| "Error serializing election status")?,
            );
            clone.initialization_report_generated = Some(false);

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

    insert_election_event(hasura_transaction, &data.election_event)
        .await
        .with_context(|| "Error inserting election event")?;

    // Upsert immutable board
    let board = upsert_b3_and_elog(hasura_transaction, tenant_id.as_str(), &election_event_id, &election_ids, is_importing_keys)
        .await
        .with_context(|| format!("Error upserting b3 board for tenant ID {tenant_id} and election event ID {election_event_id}"))?;

    update_bulletin_board(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        &board,
    )
    .await
    .with_context(|| {
        format!(
            "Error updating bulletin board reference for tenant ID {} and election event ID {}",
            tenant_id, election_event_id
        )
    })?;

    if let Some(keys_ceremonies) = data.keys_ceremonies.clone() {
        let trustees = get_all_trustees(&hasura_transaction, &tenant_id).await?;

        let trustee_map: HashMap<String, String> = trustees
            .into_iter()
            .map(|trustee| (trustee.name.clone().unwrap_or_default(), trustee.id.clone()))
            .collect();

        try_join_all(
            keys_ceremonies
                .into_iter()
                .map(|keys_ceremony| {
                    let trustee_ids = keys_ceremony
                        .trustee_ids
                        .into_iter()
                        .map(|trustee_id| trustee_map.get(&trustee_id).cloned().unwrap_or_default())
                        .collect();

                    keys_ceremony::insert_keys_ceremony(
                        hasura_transaction,
                        keys_ceremony.id,
                        keys_ceremony.tenant_id,
                        keys_ceremony.election_event_id,
                        trustee_ids,
                        /* threshold */ keys_ceremony.threshold as i32,
                        /* status */ keys_ceremony.status,
                        /* execution_status */ keys_ceremony.execution_status,
                        keys_ceremony.name,
                        keys_ceremony.settings,
                        keys_ceremony.is_default.clone().unwrap_or_default(),
                        keys_ceremony.permission_label.unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .await?;
    }

    insert_elections(hasura_transaction, &data)
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

    if let Some(applications) = data.applications.clone() {
        insert_applications(hasura_transaction, &applications)
            .await
            .with_context(|| "Error inserting applications")?;
    }

    Ok((data, replacement_map))
}

#[instrument(err, skip(hasura_transaction, temp_file))]
async fn process_voters_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    file_name: &String,
    election_event_id: Option<String>,
    tenant_id: String,
    is_admin: bool,
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
        is_admin,
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
    replacement_map: &HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let election_event_id =
        election_event_id.ok_or_else(|| anyhow!("Missing election event ID"))?;

    let mut reports = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let report = Report {
            id: Uuid::new_v4().to_string(),
            election_event_id: election_event_id.clone(),
            tenant_id: tenant_id.clone(),
            election_id: match record.get(1) {
                None => None,
                Some(election_id) if election_id.is_empty() => None,
                Some(election_id) => Some(
                    replacement_map
                        .get(election_id)
                        .ok_or_else(|| {
                            anyhow!("Can't find election_id={election_id:?} in replacement map")
                        })?
                        .clone(),
                ),
            },
            report_type: record
                .get(2)
                .ok_or_else(|| anyhow!("Missing Report Type"))?
                .to_string(),
            template_alias: record
                .get(3)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty()),
            cron_config: match record.get(4) {
                None => None,
                Some(cron_config_str) if cron_config_str.is_empty() => None,
                Some(cron_config_str) => deserialize_str(&cron_config_str).map_err(|err| {
                    anyhow!("Error parsing cron_config: {err:?}\nThe string: {cron_config_str}")
                })?,
            },
            encryption_policy: EReportEncryption::from_str(
                record
                    .get(5)
                    .ok_or_else(|| anyhow!("Missing encryption policy"))?,
            )
            .map_err(|err| anyhow!("Error parsing encryption_policy: {err:?}"))?,
            created_at: Utc::now(),
            permission_label: record.get(7).and_then(|permission_labels| {
                if permission_labels.is_empty() {
                    None
                } else {
                    Some(
                        permission_labels
                            .split("|")
                            .map(|label| label.to_string())
                            .collect(),
                    )
                }
            }),
        };

        if let Some(password) = record
            .get(6)
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
        {
            let cloned_report = report.clone();
            get_report_key_pair(
                hasura_transaction,
                cloned_report.tenant_id,
                cloned_report.election_event_id,
                Some(cloned_report.id),
                password,
            )
            .await
            .with_context(|| "Error creating secret for encrypted report")?;
        }

        reports.push(report);
    }

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

#[instrument(err, skip(temp_file))]
async fn process_activity_logs_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    election_event_id: &str,
    tenant_id: &str,
) -> Result<()> {
    let board_name = get_event_board(tenant_id, election_event_id);

    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        &tenant_id,
        Some(&election_event_id),
        board_name.as_str(),
    )
    .await?;
    electoral_log.import_from_csv(temp_file).await?;

    Ok(())
}

#[instrument(err, skip(hasura_transaction, temp_file_path))]
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
    let document_type = get_mime_types(file_suffix)[0];

    // Upload the file and return the document
    let _document = upload_and_return_document(
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

// return zip entries, and the original string of the json schema
#[instrument(err, skip(temp_file_path))]
pub async fn get_zip_entries(
    temp_file_path: NamedTempFile,
    document_type: &str,
) -> Result<(Vec<(String, Vec<u8>)>, String)> {
    let (mut zip_entries, election_event_schema) =
        if document_type == "application/ezip" || matches_mime("zip", document_type) {
            tokio::task::spawn_blocking(move || -> Result<(Vec<(String, Vec<u8>)>, String)> {
                let file = File::open(&temp_file_path)?;
                let mut zip = ZipArchive::new(file)?;
                let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

                let mut election_event_schema: Option<String> = None;
                for i in 0..zip.len() {
                    let mut file = zip.by_index(i)?;
                    let file_name = file.name().to_string();
                    if file_name.ends_with(".json") {
                        // Regular JSON document processing
                        let mut file_str = String::new();
                        file.read_to_string(&mut file_str)?;
                        election_event_schema = Some(file_str);
                    } else {
                        let mut file_contents = Vec::new();
                        file.read_to_end(&mut file_contents)?;
                        entries.push((file_name, file_contents));
                    }
                }
                if let Some(schema_str) = election_event_schema {
                    Ok((entries, schema_str))
                } else {
                    Err(anyhow!("No JSON file found in ZIP"))
                }
            })
            .await??
        } else {
            // Regular JSON document processing
            let mut file = File::open(temp_file_path)?;
            let mut data_str = String::new();
            file.read_to_string(&mut data_str)?;
            (vec![], data_str)
        };

    // sort it so that first we import the protocol manager keys files
    zip_entries.sort_by(|(file_name_a, _), (file_name_b, _)| {
        let is_a_target = file_name_a.contains(&EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name());
        let is_b_target = file_name_b.contains(&EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name());

        match (is_a_target, is_b_target) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });

    Ok((zip_entries, election_event_schema))
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

    let (zip_entries, file_election_event_schema) =
        get_zip_entries(temp_file_path, &document_type).await?;

    let is_importing_keys = zip_entries.iter().any(|(file_name, _)| {
        file_name.contains(&format!(
            "{}",
            EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name()
        ))
    });

    let election_event_id_clone = election_event_id.clone();
    let (election_event_schema, replacement_map) = process_election_event_file(
        hasura_transaction,
        &document_type,
        &file_election_event_schema,
        object,
        election_event_id.clone(),
        tenant_id.clone(),
        is_importing_keys,
    )
    .await
    .map_err(|err| anyhow!("Error processing election event file: {err}"))?;

    // Zip file processing
    if document_type == "application/ezip" || matches_mime("zip", &document_type) {
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
                    hasura_transaction,
                    &temp_file,
                    &election_event_id,
                    &tenant_id,
                )
                .await
                .context("Failed to import activity logs")?;
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
                    false,
                )
                .await
                .context("Failed to import voters")?;
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
                    &replacement_map,
                )
                .await
                .context("Failed to import reports")?;
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
                .await
                .context("Failed to import S3 files")?;
            }

            if file_name.contains(&format!("{}", EDocuments::BULLETIN_BOARDS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create bulletin boards temporary file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of bulletin boards file to temporary file")?;
                temp_file.as_file_mut().rewind()?;
                import_bulletin_boards(
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .context("Failed to import bulletin boards")?;
            }

            if file_name.contains(&format!("{}", EDocuments::SCHEDULED_EVENTS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create scheduled events temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of scheduled events file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;

                import_scheduled_events(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .with_context(|| "Error managing dates")?;
            }

            if file_name.contains(&format!("{}", EDocuments::PUBLICATIONS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create ballot publications temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of ballot publications file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;

                import_ballot_publications(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .with_context(|| "Error importing publications")?;
            }

            if file_name.contains(&format!(
                "{}",
                EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name()
            )) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create protocol manager keys temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of protocol manager keys file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;
                import_protocol_manager_keys(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .context("Failed to import protocol manager keys")?;
            }
        }
    };

    Ok(())
}
