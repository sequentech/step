// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use sequent_core::util::integrity_check::{
    integrity_check, HashFileVerifyError,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;
use windmill::services;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_election_event::{
    get_document, get_zip_entries,
};
use windmill::services::tasks_execution::*;
use windmill::services::tasks_execution::{update_complete, update_fail};
use windmill::tasks::import_election_event;
use windmill::tasks::insert_election_event::{self, CreateElectionEventInput};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]

pub struct CreateElectionEventOutput {
    id: Option<String>,
    message: Option<String>,
    error: Option<String>,
    task_execution: Option<TasksExecution>,
}

#[instrument(skip(claims))]
#[post("/insert-election-event", format = "json", data = "<body>")]
pub async fn insert_election_event_f(
    body: Json<CreateElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<CreateElectionEventOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_CREATE],
    )?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let celery_app = get_celery_app().await;
    // always set an id;
    let object = body.into_inner().clone();
    let id = object.id.clone().unwrap_or(Uuid::new_v4().to_string());

    // Insert the task execution record
    let task_execution: TasksExecution = match post(
        &tenant_id,
        Some(&id),
        ETasksExecution::CREATE_ELECTION_EVENT,
        &executer_name,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(err) => {
            return Ok(Json(CreateElectionEventOutput {
                id: None,
                message: None,
                error: Some(format!(
                    "Failed to insert task execution record: {err:?}"
                )),
                task_execution: None,
            }))
        }
    };

    let _celery_task = match celery_app
        .send_task(insert_election_event::insert_election_event_t::new(
            object,
            id.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            return Ok(Json(CreateElectionEventOutput {
                id: Some(id),
                message: None,
                error: Some(format!(
                    "Error sending Insert Election Event task: ${err}"
                )),
                task_execution: Some(task_execution.clone()),
            }));
        }
    };

    info!("Sent INSERT_ELECTION_EVENT task {}", task_execution.id);
    Ok(Json(CreateElectionEventOutput {
        id: Some(id),
        message: None,
        error: None,
        task_execution: Some(task_execution.clone()),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportElectionEventOutput {
    id: Option<String>,
    message: Option<String>,
    error: Option<String>,
    task_execution: Option<TasksExecution>,
}

#[instrument(skip(claims))]
#[post("/import-election-event", format = "json", data = "<body>")]
pub async fn import_election_event_f(
    body: Json<import_election_event::ImportElectionEventBody>,
    claims: JwtClaims,
) -> Result<Json<ImportElectionEventOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let executer_id = claims.hasura_claims.user_id.clone();

    authorize(&claims, true, Some(input.tenant_id.clone()), vec![])?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction = hasura_db_client.transaction().await.map_err(
        |err: tokio_postgres::Error| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        },
    )?;

    // Insert the task execution record
    let task_execution: TasksExecution = post(
        &tenant_id,
        None,
        ETasksExecution::IMPORT_ELECTION_EVENT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let (temp_file_path, _document, document_type) =
        match get_document(&hasura_transaction, input.clone(), None).await {
            Ok((temp_file_path, document, document_type)) => {
                (temp_file_path, document, document_type)
            }
            Err(err) => {
                let _res = update_fail(
                    &task_execution,
                    &format!("Failed to get the document: {err:?}"),
                )
                .await;
                return Ok(Json(ImportElectionEventOutput {
                    id: None,
                    message: None,
                    error: Some(err.to_string()),
                    task_execution: Some(task_execution),
                }));
            }
        };

    match input.sha256.clone() {
        Some(hash) if !hash.is_empty() => {
            match integrity_check(&temp_file_path, hash) {
                Ok(_) => {
                    info!("Hash verified !");
                }
                Err(err) => {
                    let err_str = if let HashFileVerifyError::HashMismatch(
                        input_hash,
                        gen_hash,
                    ) = err
                    {
                        format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}")
                    } else {
                        format!("Failed to verify the integrity: {err:?}")
                    };
                    info!("Failed to verify the integrity!");
                    let _res = update_fail(&task_execution, &err_str).await;
                    return Ok(Json(ImportElectionEventOutput {
                        id: None,
                        message: None,
                        error: Some(err_str),
                        task_execution: Some(task_execution),
                    }));
                }
            }
        }
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    let zip_entries_result =
        get_zip_entries(temp_file_path, &document_type).await;

    let (_zip_entries, file_election_event_schema) = match zip_entries_result {
        Ok((zip_entries, file_election_event_schema)) => {
            (zip_entries, file_election_event_schema)
        }
        Err(err) => {
            let _res = update_fail(
                &task_execution,
                &format!("Error checking import: {err:?}"),
            )
            .await;
            return Ok(Json(ImportElectionEventOutput {
                id: None,
                message: None,
                error: Some(format!("Error checking import: {:?}", err)),
                task_execution: Some(task_execution),
            }));
        }
    };

    let document_result =
        services::import::import_election_event::get_election_event_schema(
            &file_election_event_schema,
            None,
            tenant_id.clone(),
        )
        .await;

    let (election_event_schema, _replacement_map) = match document_result {
        Ok((election_event_schema, replacement_map)) => {
            (election_event_schema, replacement_map)
        }
        Err(err) => {
            let _res = update_fail(
                &task_execution,
                &format!("Error checking import: {err:?}"),
            )
            .await;
            return Ok(Json(ImportElectionEventOutput {
                id: None,
                message: None,
                error: Some(format!("Error checking import: {:?}", err)),
                task_execution: Some(task_execution),
            }));
        }
    };

    let id = election_event_schema.election_event.id.clone();

    let check_only = input.check_only.unwrap_or(false);
    if check_only {
        let _res =
            update_complete(&task_execution, Some(input.document_id.clone()))
                .await;
        return Ok(Json(ImportElectionEventOutput {
            id: Some(id),
            message: Some("Import document checked".to_string()),
            error: None,
            task_execution: Some(task_execution),
        }));
    }

    let celery_app = get_celery_app().await;
    let _celery_task = match celery_app
        .send_task(import_election_event::import_election_event::new(
            input.clone(),
            id.clone(),
            input.tenant_id.clone(),
            task_execution.clone(),
            executer_id.clone(),
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            let _res = update_fail(
                &task_execution,
                &format!("Error sending Import Election Event task: {err:?}"),
            )
            .await;
            return Ok(Json(ImportElectionEventOutput {
                id: Some(id),
                message: Some(format!(
                    "Error sending Import Election Event task: ${err}"
                )),
                error: Some(format!("Failed to verify the integrity: {err:?}")),
                task_execution: Some(task_execution),
            }));
        }
    };

    info!("Sent IMPORT_ELECTION_EVENT task {}", task_execution.id);

    Ok(Json(ImportElectionEventOutput {
        id: Some(id),
        message: Some("Task created: import_election_event".to_string()),
        error: None,
        task_execution: Some(task_execution),
    }))
}
