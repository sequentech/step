// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::election_config::{import_problems, Problem};
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
use windmill::services::electoral_log::ElectoralLogAdminContext;
use windmill::services::import::configuration_package::typed_checksum;
use windmill::services::import::import_election_event::{
    get_document, get_zip_entries,
};
use windmill::services::import::rejection::problems_of;
use windmill::services::tasks_execution::*;
use windmill::services::tasks_execution::{
    update_complete, update_fail, update_fail_with_problems,
};
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

fn authorize_insert_election_event(
    claims: &JwtClaims,
    object: &CreateElectionEventInput,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(object.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_CREATE],
    )
}

#[instrument(skip(claims))]
#[post("/insert-election-event", format = "json", data = "<body>")]
pub async fn insert_election_event_f(
    body: Json<CreateElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<CreateElectionEventOutput>, (Status, String)> {
    let object = body.into_inner();
    authorize_insert_election_event(&claims, &object)?;

    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let celery_app = get_celery_app().await;
    // always set an id;
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
    /// Why the file was refused, when the importer knows: each problem with a
    /// code, a path, a stable id and the specifics, so the Admin Portal can say
    /// it in the operator's language. `error` still carries the English line for
    /// anything that reads only that. Absent, not empty, when there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    problems: Option<Vec<Problem>>,
}

/// Why an import was refused: the line for the task's log, the line for the
/// response, and the named problems when the importer knows them.
#[derive(Debug, PartialEq)]
struct Refusal {
    log: String,
    error: String,
    problems: Option<Vec<Problem>>,
}

impl Refusal {
    /// The file could not be read at all; a wrong password is the usual reason,
    /// and the one worth saying.
    fn of_document(error: &anyhow::Error) -> Self {
        Refusal {
            log: format!("Failed to get the document: {error:?}"),
            error: error.to_string(),
            problems: problems_of(error),
        }
    }

    /// An import check that failed, with its problems if any.
    fn of_check(error: &anyhow::Error) -> Self {
        let (line, problems) = match problems_of(error) {
            // The whole chain, `a: b: c`, so the listing of what the bundle
            // lacks survives the context around it; no backtrace.
            Some(problems) => {
                (format!("Error checking import: {error:#}"), Some(problems))
            }
            None => (format!("Error checking import: {error:?}"), None),
        };
        Refusal {
            log: line.clone(),
            error: line,
            problems,
        }
    }

    /// The file is not the one whose checksum was given.
    fn of_integrity(error: HashFileVerifyError) -> Self {
        let (line, problems) = match error {
            HashFileVerifyError::HashMismatch(input_hash, gen_hash) => (
                format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}"),
                Some(vec![import_problems::checksum_mismatch(
                    &input_hash,
                    &gen_hash,
                )]),
            ),
            error => (format!("Failed to verify the integrity: {error:?}"), None),
        };
        Refusal {
            log: line.clone(),
            error: line,
            problems,
        }
    }

    /// What the caller is told.
    fn output(
        self,
        task_execution: TasksExecution,
    ) -> ImportElectionEventOutput {
        ImportElectionEventOutput {
            id: None,
            message: None,
            error: Some(self.error),
            task_execution: Some(task_execution),
            problems: self.problems,
        }
    }
}

/// A refusal: the task marked failed, and the reasons handed back.
///
/// The problems go into the task's annotations as well as the response, so the
/// task list shows the same reasons as the dialog that started it.
async fn refuse(
    task_execution: TasksExecution,
    refusal: Refusal,
) -> Json<ImportElectionEventOutput> {
    let _res = match refusal.problems.as_deref() {
        Some(problems) => {
            update_fail_with_problems(&task_execution, &refusal.log, problems)
                .await
        }
        None => update_fail(&task_execution, &refusal.log).await,
    };
    Json(refusal.output(task_execution))
}

/// What the import task learns from the caller's token, never from the
/// request body: whether they may write encrypted voter attributes (and who
/// does), and who started the import, whom its log entries name.
fn stamp_initiators(
    input: &mut import_election_event::ImportElectionEventBody,
    claims: &JwtClaims,
) {
    input.may_write_secret_attributes = authorize(
        claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::VOTER_SECRET_ATTRIBUTE_WRITE],
    )
    .is_ok();
    input.secret_write_initiator = input
        .may_write_secret_attributes
        .then(|| ElectoralLogAdminContext::from_claims(claims));
    input.importer = Some(ElectoralLogAdminContext::from_claims(claims));
}

#[instrument(skip(claims))]
#[post("/import-election-event", format = "json", data = "<body>")]
pub async fn import_election_event_f(
    body: Json<import_election_event::ImportElectionEventBody>,
    claims: JwtClaims,
) -> Result<Json<ImportElectionEventOutput>, (Status, String)> {
    let mut input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    authorize(&claims, true, Some(input.tenant_id.clone()), vec![])?;
    stamp_initiators(&mut input, &claims);

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

    let (temp_file_path, _document, document_type, package) =
        match get_document(&hasura_transaction, input.clone(), None).await {
            Ok(opened) => opened,
            Err(err) => {
                return Ok(
                    refuse(task_execution, Refusal::of_document(&err)).await
                );
            }
        };

    // A verified package was checked file by file against its signed
    // manifest, so a hand-typed checksum of the upload has nothing to add.
    match typed_checksum(input.sha256.clone(), package.as_ref()) {
        Some(hash) if !hash.is_empty() => {
            match integrity_check(&temp_file_path, hash) {
                Ok(_) => {
                    info!("Hash verified !");
                }
                Err(err) => {
                    info!("Failed to verify the integrity!");
                    return Ok(refuse(
                        task_execution,
                        Refusal::of_integrity(err),
                    )
                    .await);
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
            return Ok(refuse(task_execution, Refusal::of_check(&err)).await);
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
            return Ok(refuse(task_execution, Refusal::of_check(&err)).await);
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
            problems: None,
        }));
    }

    let celery_app = get_celery_app().await;
    let _celery_task = match celery_app
        .send_task(import_election_event::import_election_event::new(
            input.clone(),
            id.clone(),
            input.tenant_id.clone(),
            task_execution.clone(),
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
                problems: None,
            }));
        }
    };

    info!("Sent IMPORT_ELECTION_EVENT task {}", task_execution.id);

    Ok(Json(ImportElectionEventOutput {
        id: Some(id),
        message: Some("Task created: import_election_event".to_string()),
        error: None,
        task_execution: Some(task_execution),
        problems: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use windmill::services::import::rejection::reject;

    fn task() -> TasksExecution {
        TasksExecution {
            id: "task".to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: None,
            name: "import".to_string(),
            task_type: "IMPORT_ELECTION_EVENT".to_string(),
            execution_status: "IN_PROGRESS".to_string(),
            created_at: chrono::Local::now(),
            start_at: None,
            end_at: None,
            annotations: None,
            labels: None,
            logs: None,
            executed_by_user: "admin".to_string(),
        }
    }

    fn body(tenant: &str) -> import_election_event::ImportElectionEventBody {
        serde_json::from_value(serde_json::json!({
            "tenant_id": tenant,
            "document_id": "document",
            // Forged by the caller: the token decides.
            "may_write_secret_attributes": true,
            "importer": {"user_id": "someone-else", "username": "someone-else"},
        }))
        .unwrap()
    }

    #[test]
    fn the_importer_and_the_secret_writer_come_from_the_token() {
        use crate::test_claims::Claims;

        let mut input = body("tenant");
        let claims =
            Claims::new("tenant", "admin-id").username("admin").build();
        stamp_initiators(&mut input, &claims);
        let importer = input.importer.unwrap();
        assert_eq!(importer.user_id, "admin-id");
        assert_eq!(importer.username.as_deref(), Some("admin"));
        assert!(!input.may_write_secret_attributes);
        assert!(input.secret_write_initiator.is_none());

        let mut input = body("tenant");
        let writer = Claims::new("tenant", "writer-id")
            .roles([Permissions::VOTER_SECRET_ATTRIBUTE_WRITE])
            .build();
        stamp_initiators(&mut input, &writer);
        assert!(input.may_write_secret_attributes);
        assert_eq!(input.secret_write_initiator.unwrap().user_id, "writer-id");
        assert_eq!(input.importer.unwrap().user_id, "writer-id");
    }

    fn mismatch() -> Problem {
        import_problems::checksum_mismatch("expected", "actual")
    }

    #[test]
    fn a_refused_document_keeps_its_named_problems() {
        let error = reject("the file", mismatch()).context("unzipping");
        let refusal = Refusal::of_document(&error);
        assert_eq!(refusal.error, "unzipping");
        assert!(refusal.log.starts_with("Failed to get the document: "));
        assert_eq!(refusal.problems, Some(vec![mismatch()]));
    }

    #[test]
    fn an_unexplained_document_failure_names_no_problems() {
        let refusal = Refusal::of_document(&anyhow!("disk full"));
        assert_eq!(refusal.error, "disk full");
        assert_eq!(refusal.problems, None);
    }

    #[test]
    fn a_failed_check_says_so_in_plain_words() {
        let explained = Refusal::of_check(
            &reject("the file", mismatch()).context("Failed to validate"),
        );
        assert!(explained.error.starts_with("Error checking import: "));
        // The rejection's own listing survives the context wrapped around it.
        assert!(
            explained.error.contains("Failed to validate: ")
                && explained.error.contains("not the expected that was given"),
            "{}",
            explained.error
        );
        assert!(!explained.error.contains("Stack backtrace"));
        assert_eq!(explained.log, explained.error);
        assert_eq!(explained.problems, Some(vec![mismatch()]));

        let unexplained = Refusal::of_check(&anyhow!("bad schema"));
        assert!(unexplained.error.contains("bad schema"));
        assert_eq!(unexplained.problems, None);
    }

    #[test]
    fn a_checksum_mismatch_names_both_hashes() {
        let refusal = Refusal::of_integrity(HashFileVerifyError::HashMismatch(
            "expected".to_string(),
            "actual".to_string(),
        ));
        assert!(refusal
            .error
            .contains("actual does not match with the input hash: expected"));
        assert_eq!(refusal.problems, Some(vec![mismatch()]));
    }

    #[test]
    fn an_unreadable_file_is_no_mismatch() {
        let refusal = Refusal::of_integrity(HashFileVerifyError::IoError(
            "voters".to_string(),
            std::io::Error::new(std::io::ErrorKind::NotFound, "gone"),
        ));
        assert!(refusal
            .error
            .starts_with("Failed to verify the integrity: "));
        assert_eq!(refusal.problems, None);
    }

    #[test]
    fn the_answer_carries_problems_only_when_there_are_some() {
        let with =
            Refusal::of_check(&reject("the file", mismatch())).output(task());
        let json = serde_json::to_value(&with).unwrap();
        assert_eq!(json["problems"][0]["id"], "file.checksum-mismatch");
        assert_eq!(json["task_execution"]["id"], "task");
        assert!(json["id"].is_null());

        let without =
            Refusal::of_document(&anyhow!("disk full")).output(task());
        let json = serde_json::to_value(&without).unwrap();
        assert_eq!(json["error"], "disk full");
        assert!(json.get("problems").is_none());
    }
}

#[cfg(test)]
mod insert_election_event_tests {
    use super::*;

    fn admin(tenant_id: &str) -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": tenant_id,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": [
                    Permissions::ELECTION_EVENT_CREATE.to_string()
                ]
            }
        }))
        .unwrap()
    }

    fn event_in(tenant_id: &str) -> CreateElectionEventInput {
        serde_json::from_value(serde_json::json!({
            "tenant_id": tenant_id,
            "name": "event"
        }))
        .unwrap()
    }

    #[test]
    fn create_is_allowed_in_the_callers_tenant() {
        let tenant_id = Uuid::new_v4().to_string();
        assert!(authorize_insert_election_event(
            &admin(&tenant_id),
            &event_in(&tenant_id)
        )
        .is_ok());
    }

    #[test]
    fn create_is_rejected_in_another_tenant() {
        let result = authorize_insert_election_event(
            &admin(&Uuid::new_v4().to_string()),
            &event_in(&Uuid::new_v4().to_string()),
        );
        assert_eq!(
            result.map_err(|(status, _)| status),
            Err(Status::Unauthorized)
        );
    }
}
