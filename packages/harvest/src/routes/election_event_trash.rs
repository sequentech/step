// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit;
use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use windmill::services::database::get_hasura_pool;
use windmill::services::delete_election_event::ElectionEventDeletionRefusal;
use windmill::services::election_event_trash::{
    get_election_event_trash, restore_election_event, ElectionEventTrash,
    TrashError,
};
use windmill::services::tasks_execution::{post, update_complete, update_fail};
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;
use windmill::types::tasks::ETasksExecution;

#[derive(Deserialize, Debug)]
pub struct ListTrashedElectionEventsInput {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct RestoreElectionEventInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreElectionEventOutput {
    id: String,
    is_archived: bool,
}

fn internal_error(error: impl std::fmt::Debug) -> (Status, String) {
    (Status::InternalServerError, format!("{error:?}"))
}

/// The status a refused Trash operation answers with.
fn trash_error_status(error: &anyhow::Error) -> Status {
    if error
        .downcast_ref::<ElectionEventDeletionRefusal>()
        .is_some()
    {
        return Status::Forbidden;
    }
    match error.downcast_ref::<TrashError>() {
        Some(TrashError::NotFound) => Status::NotFound,
        Some(_) => Status::Conflict,
        None => Status::InternalServerError,
    }
}

fn authorize_trash(claims: &JwtClaims) -> Result<String, (Status, String)> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    authorize(
        claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_DELETE],
    )?;
    Ok(tenant_id)
}

/// A page of the caller tenant's election events in Trash.
#[instrument(skip(claims))]
#[post("/list-trashed-election-events", format = "json", data = "<body>")]
pub async fn list_trashed_election_events_f(
    body: Json<ListTrashedElectionEventsInput>,
    claims: JwtClaims,
) -> Result<Json<ElectionEventTrash>, (Status, String)> {
    let tenant_id = authorize_trash(&claims)?;
    let input = body.into_inner();
    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(internal_error)?;
    let hasura_transaction = hasura_db_client
        .build_transaction()
        .read_only(true)
        .start()
        .await
        .map_err(internal_error)?;
    let trash = get_election_event_trash(
        &hasura_transaction,
        &tenant_id,
        input.limit,
        input.offset,
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(trash))
}

/// Restores an election event of the caller's tenant from Trash, with the
/// archive state it had.
#[instrument(skip(claims))]
#[post("/restore-election-event", format = "json", data = "<body>")]
pub async fn restore_election_event_f(
    body: Json<RestoreElectionEventInput>,
    claims: JwtClaims,
) -> Result<Json<RestoreElectionEventOutput>, (Status, String)> {
    let tenant_id = authorize_trash(&claims)?;
    let election_event_id = body.into_inner().election_event_id;
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::RESTORE_ELECTION_EVENT,
        &executer_name,
    )
    .await
    .map_err(internal_error)?;

    let restored = async {
        let mut hasura_db_client = get_hasura_pool().await.get().await?;
        let hasura_transaction = hasura_db_client.transaction().await?;
        let is_archived = restore_election_event(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &decode_permission_labels(&claims),
            Some(&audit::actor(&claims)),
        )
        .await?;
        hasura_transaction.commit().await?;
        kick_audit_log_outbox();
        anyhow::Ok(is_archived)
    }
    .await;

    match restored {
        Ok(is_archived) => {
            info!(
                tenant_id,
                election_event_id,
                user_id = claims.hasura_claims.user_id,
                is_archived,
                "Restored election event from Trash"
            );
            let _ = update_complete(&task_execution, None).await;
            Ok(Json(RestoreElectionEventOutput {
                id: election_event_id,
                is_archived,
            }))
        }
        Err(error) => {
            let message = format!(
                "Error restoring election event {election_event_id}: {error}"
            );
            let _ = update_fail(&task_execution, &message).await;
            Err((trash_error_status(&error), message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID,
    };

    #[test]
    fn trash_requires_the_election_event_delete_permission() {
        let reader = admin_claims(CALLER_TENANT_ID, &["election-event-read"]);
        assert!(authorize_trash(&reader).is_err());
        let archiver =
            admin_claims(CALLER_TENANT_ID, &["election-event-archive"]);
        assert!(authorize_trash(&archiver).is_err());

        let deleter =
            admin_claims(CALLER_TENANT_ID, &["election-event-delete"]);
        assert_eq!(authorize_trash(&deleter).unwrap(), CALLER_TENANT_ID);
    }

    #[rocket::async_test]
    async fn routes_refuse_callers_without_the_delete_permission_first() {
        for roles in [&["election-event-read"][..], &["election-event-archive"]]
        {
            let claims = admin_claims(CALLER_TENANT_ID, roles);
            let listed = list_trashed_election_events_f(
                Json(ListTrashedElectionEventsInput {
                    limit: None,
                    offset: None,
                }),
                claims.clone(),
            )
            .await;
            assert_eq!(listed.unwrap_err().0, Status::Unauthorized);

            let restored = restore_election_event_f(
                Json(RestoreElectionEventInput {
                    election_event_id: "1f0e4c2a-6b8d-4e3f-9a71-2c5d8e9f0a1b"
                        .to_string(),
                }),
                claims,
            )
            .await;
            assert_eq!(restored.unwrap_err().0, Status::Unauthorized);
        }
    }

    #[test]
    fn refusals_answer_not_found_forbidden_or_conflict() {
        assert_eq!(
            trash_error_status(&TrashError::NotFound.into()),
            Status::NotFound
        );
        for error in [TrashError::NotInTrash, TrashError::PurgeStarted] {
            assert_eq!(trash_error_status(&error.into()), Status::Conflict);
        }
        assert_eq!(
            trash_error_status(
                &ElectionEventDeletionRefusal::PermissionLabels.into()
            ),
            Status::Forbidden
        );
        assert_eq!(
            trash_error_status(&anyhow::anyhow!("database down")),
            Status::InternalServerError
        );
    }
}
