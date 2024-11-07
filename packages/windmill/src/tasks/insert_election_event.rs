// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use keycloak::types::RealmRepresentation;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use serde_json::{json, Value};
use std::env;
use std::fs;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};

use crate::services::tasks_execution::{update_complete, update_fail};
use sequent_core::types::hasura::core::TasksExecution;

use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::hasura::election_event::{get_election_event, insert_election_event};
use crate::services::election_event_board::BoardSerializable;
use crate::services::import::import_election_event::insert_election_event_db;
use crate::services::import::import_election_event::upsert_b3_and_elog;
use crate::services::import::import_election_event::upsert_keycloak_realm;
use crate::types::error::Result;

#[instrument(err)]
pub async fn insert_election_event_anyhow(
    object: InsertElectionEventInput,
    id: String,
    task_execution: TasksExecution
) -> AnyhowResult<()> {
    let mut final_object = object.clone();
    final_object.id = Some(id.clone());
    let tenant_id = object.tenant_id.clone().unwrap();

    let board = upsert_b3_and_elog(tenant_id.as_str(), &id.as_ref(), &vec![], false).await?;
    final_object.bulletin_board_reference = Some(board);
    final_object.id = Some(id.clone());

	match upsert_keycloak_realm(tenant_id.as_str(), &id.as_ref(), None).await {
		Ok(realm) => Some(realm),
        Err(err) => {
            update_fail(&task_execution, "Failed to update task execution status to COMPLETED").await?;
            return Err(anyhow!("Failed to update task execution status to COMPLETED {err}"));        
        }
    };

	let auth_headers = match get_client_credentials().await {
        Ok(auth_headers) => auth_headers,
        Err(err) => {
            update_fail(&task_execution, "Failed to update task execution status to COMPLETED").await?;
            return Err(anyhow!("Failed to update task execution status to COMPLETED {err}").into());
        }
    };

	match insert_election_event_db(&auth_headers, &final_object).await {
		Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Failed to update task execution status to COMPLETED").await?;
            return Err(anyhow!("Failed to update task execution status to COMPLETED {err}").into());
        }
    };

	update_complete(&task_execution)
        .await
        .context("Failed to update task execution status to COMPLETED")
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(
    object: InsertElectionEventInput,
    id: String,
    task_execution: TasksExecution
) -> Result<()> {
    insert_election_event_anyhow(
        object, id, task_execution
    ).await?;

    Ok(())
}
