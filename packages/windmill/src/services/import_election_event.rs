// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::protocol_manager::get_event_board;
use crate::services::tasks_execution::update_fail;
use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingPeriodDates;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::core::Template;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use super::database::get_hasura_pool;
use super::date::ISO8601;
use super::documents;
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
use crate::tasks::import_election_event::ImportElectionEventBody;
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
    id: Option<String>,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
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

    let temp_file_path = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    let mut file = File::open(temp_file_path)?;

    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;

    let original_data: ImportElectionEventSchema = deserialize_str(&data_str)?;

    let data = replace_ids(&data_str, &original_data, id, tenant_id)?;

    Ok(data)
}

#[instrument(err, skip_all)]
pub async fn process(
    hasura_transaction: &Transaction<'_>,
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    // Get the document
    let mut data = get_document(
        hasura_transaction,
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
