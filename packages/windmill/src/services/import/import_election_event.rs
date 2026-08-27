// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::application::insert_applications;
use crate::postgres::election_event::{get_election_event_by_id_if_exist, update_bulletin_board};
use crate::postgres::reports::insert_reports;
use crate::postgres::reports::Report;
use crate::postgres::trustee::get_all_trustees;
use crate::services::import::import_publications::{
    import_ballot_publications, import_election_event_config_file,
};
use crate::services::import::import_scheduled_events::import_scheduled_events;
use crate::services::import::import_tally::process_tally_file;
use crate::services::keycloak::read_realm_config_from_s3;
use crate::services::protocol_manager::get_event_board;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_key_pair;
use crate::services::tasks_execution::update_fail;
use crate::tasks::insert_election_event::CreateElectionEventInput;
use crate::types::documents::ETallyDocuments;
use ::keycloak::types::{ComponentExportRepresentation, RealmRepresentation};
use anyhow::{anyhow, Context, Result};
use chrono::format;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::future::try_join_all;
use keycloak::types::RealmEventsConfigRepresentation;
use once_cell::sync::Lazy;
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::PeriodDates;
use sequent_core::ballot::VotingPeriodDates;
use sequent_core::ballot::VotingStatus;
use sequent_core::ballot::{AllowTallyStatus, LanguageDetectionPolicy};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::connection;
use sequent_core::services::keycloak::{
    generate_client_secret, get_client_credentials, get_event_realm, replace_realm_ids,
    KeycloakAdminClient,
};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::locale::iso_639_2t_to_bcp47;
use sequent_core::util::mime::{get_mime_types, matches_mime};
use sequent_core::util::version::{
    check_version_compatibility, DEV_APP_VERSION, ENV_VAR_APP_VERSION, HISTORICAL_DEFAULT_VERSION,
    VERSION_KEY,
};
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

const KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY: &str =
    "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY";

use super::import_users::import_users_file;
use crate::postgres;
use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::candidate::insert_candidates;
use crate::postgres::certificate_authority::{
    insert_certificate_authority, CertificateAuthorityRecord,
};
use crate::postgres::contest::insert_contest;
use crate::postgres::election::insert_elections;
use crate::postgres::election_event::insert_election_event;
use crate::postgres::keys_ceremony;
use crate::postgres::scheduled_event::insert_scheduled_event;
use crate::services::certificate_authority::{parse_certificate_pem, split_pem_bundle};
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
use regex::Regex;
use sequent_core::types::hasura::core::{Area, Candidate, Contest, Election, ElectionEvent};
use sequent_core::types::keycloak::{
    CERTIFICATES_IDP_ALIAS, DEFAULT_IVR_SERVICE_CLIENT_ID, IVR_VOTING_CLIENT_ID,
};
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
    #[serde(default = "default_version")]
    pub version: String,
}

/// Set the default version of an imported election event to be compatible with version 9, which is the first version to include this feature.
fn default_version() -> String {
    HISTORICAL_DEFAULT_VERSION.to_string()
}

#[instrument(err)]
pub async fn upsert_b3_and_elog(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
    dont_auto_generate_keys: bool, // avoid creating protocol manager keys
) -> Result<Value> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
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
        let board_name = get_election_board(tenant_id, &election_id, &slug);

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
pub async fn read_default_election_event_realm() -> Result<RealmRepresentation> {
    read_realm_config_from_s3(KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY).await
}

#[instrument(skip(realm))]
pub fn remove_keycloak_realm_secrets(realm: &RealmRepresentation) -> Result<RealmRepresentation> {
    let mut realm_copy = realm.clone();

    // Collect well-known clients and their secrets to set.
    let keycloak_client_id = env::var("KEYCLOAK_CLIENT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .with_context(|| "KEYCLOAK_CLIENT_ID can't be empty")?;
    let keycloak_client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .with_context(|| "KEYCLOAK_CLIENT_SECRET can't be empty")?;
    let mut known_clients = vec![
        // Keycloak client is always known and required, as checked (and failed if required) above.
        (keycloak_client_id, Some(keycloak_client_secret)),
        // IVR specific clients are optional, their secrets may or may not be configured during deployment.
        // This is not considered a failure, it will be ignored, and a new secret will be generated.
        (
            env::var("KEYCLOAK_IVR_SERVICE_CLIENT_ID")
                .unwrap_or(DEFAULT_IVR_SERVICE_CLIENT_ID.to_string()),
            env::var("KEYCLOAK_IVR_SERVICE_CLIENT_SECRET").ok(),
        ),
        (
            IVR_VOTING_CLIENT_ID.to_string(),
            env::var("KEYCLOAK_IVR_VOTING_CLIENT_SECRET").ok(),
        ),
    ];

    // For each IDP that has both clientId and clientSecret configured,
    // look if it is the special CERTIFICATES_IDP_ALIAS, then generate a
    // new secret, update the IDP config, and record (clientId -> newSecret) in the known_clients so the
    // matching Keycloak client can be given the same credential in the client's loop below.
    if let Some(identity_providers) = realm_copy.identity_providers.clone() {
        let new_identity_providers = identity_providers
            .iter()
            .map(|idp| {
                let mut idp_copy = idp.clone();
                match idp_copy.config.clone() {
                    Some(config) if idp.alias.as_deref() == Some(CERTIFICATES_IDP_ALIAS) => {
                        let mut new_config = config.clone();
                        if let Some(idp_client_id) = new_config.get("clientId").cloned() {
                            if new_config.contains_key("clientSecret") {
                                let new_secret = generate_client_secret();
                                new_config.insert("clientSecret".to_string(), new_secret.clone());
                                known_clients.push((idp_client_id, Some(new_secret)));
                            }
                        }
                        idp_copy.config = Some(new_config);
                    }
                    _ => {
                        // no config, nothing to do
                    }
                }
                idp_copy
            })
            .collect();
        realm_copy.identity_providers = Some(new_identity_providers);
    }

    // For each client, assign its secret:
    // 1. The known Keycloak clients (such as service-account, or ivr clients) get the configured env var secret.
    // 2. The client that was configured in IDP CERTIFICATES_IDP_ALIAS gets the generated client secret (becomes a "known client").
    // 3. All others have their secret cleared so Keycloak regenerates it.
    realm_copy.clients = realm_copy.clients.map(|clients| {
        clients
            .iter()
            .map(|client| {
                let mut client_copy = client.clone();
                // Check if the client matches with any known client.
                // If not, we'll leave None, and Keycloak will regenerate it.
                client_copy.secret = client.client_id.as_deref().and_then(|client_id| {
                    known_clients
                        .iter()
                        .find(|(known_id, _)| client_id == known_id)
                        .and_then(|(known_id, known_secret)| {
                            // Don't accept empty values
                            let secret = known_secret.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
                            if secret.is_none() {
                                tracing::warn!("Known client '{known_id}' had no secret configured, regenerating");
                            }
                            secret
                        })
                });
                client_copy
            })
            .collect()
    });
    // remove certificates, only leaving their algorithm/priority
    let valid_keys: Vec<String> = vec!["priority".to_string(), "algorithm".to_string()];
    if let Some(components) = realm_copy.components.clone() {
        let mut newcomponents = components.clone();
        let key: &'static str = "org.keycloak.keys.KeyProvider";
        if let Some(val) = components.get(key) {
            let newval: Vec<ComponentExportRepresentation> = val
                .iter()
                .map(|el| {
                    let mut elnew = el.clone();
                    if let Some(config) = elnew.config.clone() {
                        let mut newconfig = config.clone();
                        for k in config.keys() {
                            if !valid_keys.contains(&k) {
                                info!("Removing key {} from {}", k, key);
                                newconfig.remove(k);
                            }
                        }
                        elnew.config = Some(newconfig);
                    }
                    elnew
                })
                .collect();
            newcomponents.insert(key.to_string(), newval.clone());
        }
        realm_copy.components = Some(newcomponents);
    }
    Ok(realm_copy)
}

#[instrument(err, skip(keycloak_event_realm))]
pub async fn upsert_keycloak_realm(
    tenant_id: &str,
    election_event_id: &str,
    keycloak_event_realm: Option<RealmRepresentation>,
    default_locale: Option<String>,
) -> Result<()> {
    let mut realm = if let Some(realm) = keycloak_event_realm.clone() {
        realm
    } else {
        let realm = read_default_election_event_realm().await?;
        realm
    };

    if let Some(default_language) = default_locale {
        // Keycloak uses BCP 47 locale codes; convert from ISO 639-2/T if needed
        let keycloak_locale = iso_639_2t_to_bcp47(&default_language).to_string();
        realm.default_locale = Some(keycloak_locale);
        let mut attrs = realm.attributes.clone().unwrap_or_default();
        attrs.insert(
            "language_detection_policy".to_string(),
            "force-default".to_string(),
        );
        // Store the internal locale code so the login template can set USER_LANGUAGE correctly
        attrs.insert("forced_language_code".to_string(), default_language.clone());
        realm.attributes = Some(attrs);
    }

    realm = remove_keycloak_realm_secrets(&realm)?;
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
    let election_event_id = object
        .id
        .clone()
        .ok_or(anyhow!("Empty election event id"))?;
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
        id: election_event_id.clone(),
        tenant_id: object.tenant_id.clone(),
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
        statistics: Some(json!({
            "num_emails_sent": 0,
            "num_sms_sent": 0
        })),
        external_id: None,
    };

    insert_election_event(&hasura_transaction, &new_election_input).await?;
    Ok(())
}

/// Replaces UUIDs in the import data while preserving specific IDs that should remain unchanged.
///
/// This function is a thin wrapper around `replace_realm_ids()` that processes the election event
/// import schema and replaces most UUIDs with new ones, while keeping certain IDs unchanged
/// (like tenant_id and optionally election_event_id). It also automatically preserves UUIDs
/// referenced in Keycloak authenticator configurations.
///
/// # Arguments
/// * `data_str` - The original JSON string representation of the import data
/// * `original_data` - The parsed ImportElectionEventSchema structure
/// * `event_id` - Optional election event ID to use. If None, a new UUID will be generated
/// * `tenant_id` - The tenant ID to use (may differ from the original)
///
/// # Returns
/// A tuple containing:
/// * The modified ImportElectionEventSchema with replaced UUIDs
/// * A HashMap mapping old UUIDs to their new replacements
#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    event_id: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    // Prepare tenant_id replacement - always replace to ensure consistency
    let tenant_id_replacement = Some((original_data.tenant_id.to_string(), tenant_id.clone()));

    // Prepare election_event_id replacement if a specific one was provided
    let election_event_id_replacement = event_id
        .as_ref()
        .map(|new_id| (original_data.election_event.id.clone(), new_id.clone()));

    // Use replace_realm_ids which handles:
    // - Preserving UUIDs in Keycloak authenticator configurations
    // - Preserving tenant_id and election_event_id in the keep list before UUID replacement
    // - Applying explicit tenant_id and election_event_id replacements after UUID replacement
    let (new_data, replacement_map) = replace_realm_ids(
        data_str,
        vec![], // Empty keep list - replace_realm_ids will populate it automatically
        tenant_id_replacement,
        election_event_id_replacement,
    )?;

    // Parse the modified JSON string back into the structured format
    let data: ImportElectionEventSchema = deserialize_str(&new_data)?;

    // Return both the modified schema and the UUID replacement mapping
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
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

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

/// Get the election event schma and also:
/// - Check version compatibility
/// - Replace IDs and return a mapping of old to new IDs (for preserving references in other documents like voters)
#[instrument(err, skip_all)]
pub async fn get_election_event_schema(
    data_str: &str,
    event_id: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    // Catch a version missmatch early and return a clear error message about it, rather than having it fail later on
    // with a more obscure error when trying to deserialize data that is incompatible with the current version.
    let raw: serde_json::Value = serde_json::from_str(data_str)
        .map_err(|e| anyhow!("Failed to parse import data as JSON: {e}"))?;
    let default_ver = default_version();
    let imported_version = raw
        .get(VERSION_KEY)
        .and_then(|v| v.as_str())
        .unwrap_or(&default_ver);
    let current_version = std::env::var(ENV_VAR_APP_VERSION)
        .map_err(|_| anyhow!("Environment variable {ENV_VAR_APP_VERSION} should be set"))?;
    check_version_compatibility(imported_version, &current_version)?;
    let original_data: ImportElectionEventSchema = deserialize_str(data_str)?;
    replace_ids(data_str, &original_data, event_id, tenant_id.clone())
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
    .map_err(|err| anyhow!("Error getting document for election event ID {election_event_id} and tenant ID {tenant_id}: {err}"))?;

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
            status.telephone_voting_status = VotingStatus::default();
            status.voting_period_dates = PeriodDates::default();
            status.kiosk_voting_period_dates = PeriodDates::default();
            status.telephone_voting_period_dates = PeriodDates::default();

            clone.status = Some(
                serde_json::to_value(status)
                    .with_context(|| "Error serializing election status")?,
            );
            clone.initialization_report_generated = Some(false);

            Ok(clone)
        })
        .collect::<Result<Vec<Election>>>()
        .with_context(|| "Error processing elections")?;

    let language_detection_policy = data.election_event.get_language_detection_policy();
    let mut default_language = None;
    if language_detection_policy == LanguageDetectionPolicy::FORCE_DEFAULT {
        default_language = Some(data.election_event.get_default_language());
    }

    upsert_keycloak_realm(
        tenant_id.as_str(),
        &election_event_id,
        data.keycloak_event_realm.clone(),
        default_language
    )
    .await
    .with_context(|| format!("Error upserting Keycloak realm for tenant ID {tenant_id} and election event ID {election_event_id}"))?;

    insert_election_event(hasura_transaction, &data.election_event)
        .await
        .with_context(|| "Error inserting election event")?;

    manage_dates(&data, hasura_transaction)
        .await
        .with_context(|| "Error managing dates")?;

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
    may_write_secret_attributes: bool,
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
        may_write_secret_attributes,
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
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);

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

async fn extract_document_uuid(filename: &str) -> Result<Option<&str>> {
    // Regex to match the UUID after "document_"
    let re = Regex::new(
        r"document_([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
    )
    .ok()
    .ok_or_else(|| anyhow!("Invalid regex"))?;

    let uuid = re
        .captures(filename)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()));
    Ok(uuid)
}

async fn extract_document_name(filename: &str) -> Result<Option<&str>> {
    let re = Regex::new(
        r"document_[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}_(.+)"
    )
    .ok()
    .ok_or_else(|| anyhow!("Invalid regex"))?;

    let name = re
        .captures(filename)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()));
    Ok(name)
}

static UUID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap()
});

pub fn replace_ids_in_filename(
    file_name: &str,
    replacement_map: &HashMap<String, String>,
) -> String {
    UUID_RE
        .replace_all(file_name, |caps: &regex::Captures| {
            let id = caps.get(0).unwrap().as_str();
            replacement_map
                .get(id)
                .map(String::as_str)
                .unwrap_or(id)
                .to_owned()
        })
        .into_owned()
}

#[instrument(err, skip(hasura_transaction, temp_file_path, replacement_map))]
pub async fn process_s3_file(
    hasura_transaction: &Transaction<'_>,
    temp_file_path: &NamedTempFile,
    file_name: &str,
    election_event_id: Option<String>,
    tenant_id: String,
    replacement_map: HashMap<String, String>,
    is_public: bool,
) -> Result<()> {
    let file_path_string = temp_file_path.path().to_string_lossy().to_string();

    let file_size = get_file_size(file_path_string.as_str())
        .with_context(|| format!("Error obtaining file size for {}", file_path_string))?;

    let file_suffix = Path::new(&file_path_string)
        .extension()
        .ok_or(anyhow!("Empty extension"))?
        .to_str()
        .ok_or(anyhow!("Empty file suffix"))?;
    let document_type = get_mime_types(file_suffix)[0];

    let document_uuid = extract_document_uuid(file_name)
        .await
        .map_err(|e| anyhow!("Error extracting document UUID from filename: {e}"))?
        .ok_or_else(|| anyhow!("Error extracting document UUID as str"))?;

    let new_document_id = replacement_map
        .get(document_uuid)
        .ok_or_else(|| anyhow!("Error finding document UUID in replacement map"))?;

    let file_name = extract_document_name(file_name)
        .await
        .map_err(|e| anyhow!("Error extracting document name from filename: {e}"))?
        .ok_or_else(|| anyhow!("Error getting document name as str"))?;

    let new_file_name = replace_ids_in_filename(&file_name, &replacement_map);
    // Upload the file and return the document
    let _document = upload_and_return_document(
        hasura_transaction,
        &file_path_string.clone(),
        file_size,
        &document_type,
        &tenant_id,
        election_event_id,
        &new_file_name,
        Some(new_document_id.to_string()),
        is_public.clone(),
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
                    if file_name.contains(EDocuments::ELECTION_EVENT.to_file_name())
                        && file_name.ends_with(".json")
                    {
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

    // Sort the ZIP entries by importance:
    // 1. Protocol Manager keys are imported first (rank 0)
    // 2. Regular files come next (rank 1)
    // 3. Inside the TALLY directory:
    //    - TALLY_SESSION and RESULTS_EVENT files are imported just before others (rank 2)
    //    - All other TALLY files come last (rank 3)
    zip_entries.sort_by_key(|(file_name, _)| {
        let rank = if file_name.contains(EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name()) {
            0
        } else if file_name.contains(EDocuments::TALLY.to_file_name()) {
            if file_name.contains(ETallyDocuments::TALLY_SESSION.to_file_name())
                || file_name.contains(ETallyDocuments::RESULTS_EVENT.to_file_name())
            {
                2
            } else {
                3
            }
        } else {
            1
        };

        (rank, file_name.clone()) // rank first, then alphabetically within rank
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

    let tally_session_file = zip_entries
        .iter()
        .find(|(name, _)| name.contains(ETallyDocuments::TALLY_SESSION.to_file_name()));
    let results_event_file = zip_entries
        .iter()
        .find(|(name, _)| name.contains(ETallyDocuments::RESULTS_EVENT.to_file_name()));

    let mut tally_files_content: Option<String> = None;
    if let (Some(tally_session_file), Some(results_event_file)) =
        (tally_session_file, results_event_file)
    {
        let tally_session_file_content = String::from_utf8(tally_session_file.1.clone())?;
        let results_event_file_content = String::from_utf8(results_event_file.1.clone())?;
        tally_files_content = Some(format!(
            "\n{}\n{}",
            tally_session_file_content, results_event_file_content
        ));
    }
    let file_election_event_schema = match tally_files_content {
        Some(tally_files_content) => {
            format!("{}\n{}", file_election_event_schema, tally_files_content)
        }
        None => file_election_event_schema,
    };

    let may_write_secret_attributes = object.may_write_secret_attributes;
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
                    may_write_secret_attributes,
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

            if file_name.contains(&format!("{}/", EDocuments::S3_FILES.to_file_name())) {
                let folder_path: Vec<_> = file_name.split("/").collect();
                // Skips the OS created files
                if folder_path[1] == EDocuments::VOTERS.to_file_name() {
                    continue;
                }

                // Write the file contents to a new file within this directory
                let mut temp_file =
                    generate_temp_file(&folder_path[1], &folder_path[folder_path.len() - 1])
                        .context("Error generating temp file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy S3 contents to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_s3_file(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    Some(election_event_schema.election_event.id.clone()),
                    election_event_schema.tenant_id.to_string(),
                    replacement_map.clone(),
                    false,
                )
                .await
                .context("Failed to import S3 files")?;
            }
            if file_name.contains(&format!("{}/", EDocuments::IMAGES.to_file_name())) {
                let folder_path: Vec<_> = file_name.split("/").collect();

                // Write the file contents to a new file within this directory
                let mut temp_file =
                    generate_temp_file(&folder_path[1], &folder_path[folder_path.len() - 1])
                        .context("Error generating temp file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy S3 contents to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_s3_file(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    None,
                    election_event_schema.tenant_id.to_string(),
                    replacement_map.clone(),
                    true,
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
                EDocuments::ELECTION_EVENT_CONFIG.to_file_name()
            )) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create election event config temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of election event config file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;

                import_election_event_config_file(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .with_context(|| "Error importing election event config file")?;
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

            if file_name.contains(&format!("{}/", EDocuments::TALLY.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create ballot publications temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of ballot publications file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;
                let tally_file_name = file_name
                    .split("/")
                    .last()
                    .ok_or(anyhow!("Unexpected, tally without filename"))?
                    .split(".")
                    .next()
                    .ok_or(anyhow!("Unexpected tally without extension"))?;

                process_tally_file(
                    hasura_transaction,
                    &temp_file,
                    tally_file_name.to_string(),
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    replacement_map.clone(),
                )
                .await
                .context("Failed to import tally_file")?;
            }

            if file_name.contains(EDocuments::CERTIFICATES.to_file_name()) {
                let pem_content = String::from_utf8(file_contents.clone())
                    .context("Failed to decode certificates PEM as UTF-8")?;
                let tenant_uuid = election_event_schema.tenant_id;
                let election_event_uuid = Uuid::parse_str(&election_event_schema.election_event.id)
                    .context("Failed to parse election event UUID")?;
                let pem_chunks = split_pem_bundle(&pem_content);
                for pem_chunk in pem_chunks {
                    let pem_chunk_owned = pem_chunk.clone();
                    let parsed = tokio::task::spawn_blocking(move || {
                        parse_certificate_pem(&pem_chunk_owned)
                    })
                    .await
                    .context("Failed to spawn blocking task for cert parsing")?
                    .context("Failed to parse certificate PEM")?;
                    let record = CertificateAuthorityRecord {
                        id: Uuid::new_v4(),
                        tenant_id: tenant_uuid,
                        election_event_id: election_event_uuid,
                        common_name: parsed.common_name,
                        subject: parsed.subject,
                        issuer_common_name: parsed.issuer_common_name,
                        issuer: parsed.issuer,
                        not_before: parsed.not_before,
                        not_after: parsed.not_after,
                        fingerprint_sha256: parsed.fingerprint_sha256,
                        serial_number: parsed.serial_number,
                        pem: parsed.pem,
                    };
                    insert_certificate_authority(hasura_transaction, record)
                        .await
                        .context("Failed to insert certificate authority")?;
                }
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
    let Some(scheduled_events) = data.scheduled_events.clone() else {
        return Ok(());
    };

    //Manage election event
    let election_event_dates = generate_voting_period_dates(
        scheduled_events.clone(),
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
            scheduled_events.clone(),
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
