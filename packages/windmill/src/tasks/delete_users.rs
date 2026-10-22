// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::audit_log::{AdminAction, AdminChange, AuditActor, AuditObject};
use crate::services::ballot_box::get_voter_ballot_states;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::election::is_election_event_locked_down;
use crate::services::task_audit;
use crate::services::tasks_execution::{update_complete_with_annotations, update_fail};
use crate::services::users::{list_users, list_users_with_vote_info, ListUsersFilter};
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use tracing::{info, instrument};

/// Page size when resolving a whole filtered set to ids. `list_users`
/// clamps the limit to `low_sql_limit`, so the loop below must not assume it
/// gets the page size it asked for.
const RESOLVE_PAGE_SIZE: i32 = 1000;

/// Failures recorded verbatim on the task before the rest are summarised, so a
/// wholesale failure cannot blow up the task_execution row.
const MAX_REPORTED_FAILURES: usize = 50;

const VOTER_HAS_BALLOT: &str = "the voter has cast a ballot";

/// The deleted ids an audit entry lists.
const MAX_AUDITED_IDS: usize = 1000;

const VOTER: &str = "voter";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteUsersOutput {
    pub ids: Option<String>,
    pub error_msg: Option<String>,
    pub task_execution: Option<TasksExecution>,
}

/// Deletes voters in the background.
///
/// Either an explicit `user_ids` list is supplied, or `filter` selects every
/// voter matching the caller's current list filters -- that is what "select
/// all" has to mean. Resolving the ids here rather than in the browser is the
/// point of the task: the browser only ever knows the page it has loaded.
///
/// A failure on one voter does not abort the run. Every voter is attempted and
/// the failures are reported together, so one bad row cannot leave the deletion
/// half applied with no record of where it stopped.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn delete_users(
    realm: String,
    user_ids: Option<Vec<String>>,
    filter: Option<ListUsersFilter>,
    task_execution: Option<TasksExecution>,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    ensure_election_event_not_locked(&task_execution).await?;

    let ids = match user_ids {
        Some(ids) => ids,
        None => {
            let Some(filter) = filter else {
                return fail(
                    &task_execution,
                    "Neither an explicit voter list nor a filter was supplied".to_string(),
                )
                .await;
            };
            match resolve_ids(filter).await {
                Ok(ids) => ids,
                Err(err) => {
                    return fail(
                        &task_execution,
                        format!("Failed to resolve the voters to delete: {err}"),
                    )
                    .await;
                }
            }
        }
    };

    // Resolving a large filtered set can take long enough for lockdown to
    // change after the task starts. Recheck immediately before the first
    // destructive operation.
    ensure_election_event_not_locked(&task_execution).await?;

    let voters_with_ballots = match voters_with_active_ballots(&task_execution).await {
        Ok(voters) => voters,
        Err(err) => {
            return fail(
                &task_execution,
                format!("Failed to check the voters' ballots: {err}"),
            )
            .await;
        }
    };

    let total = ids.len();
    info!("Deleting {total} voters from realm {realm}");

    let client = match KeycloakAdminClient::new().await {
        Ok(client) => client,
        Err(err) => {
            return fail(
                &task_execution,
                format!("Failed to build the Keycloak admin client: {err}"),
            )
            .await;
        }
    };

    let mut failures: Vec<String> = vec![];
    let mut deleted_ids: Vec<&String> = vec![];
    for id in &ids {
        if voters_with_ballots.contains(id) {
            failures.push(format!("{id}: {VOTER_HAS_BALLOT}"));
            continue;
        }
        match client.delete_user(&realm, id).await {
            Ok(_) => deleted_ids.push(id),
            Err(err) => failures.push(format!("{id}: {err}")),
        }
    }
    let deleted = deleted_ids.len();

    if deleted > 0 {
        if let Err(err) = audit_deleted(&task_execution, audit_actor, &deleted_ids, total).await {
            return fail(
                &task_execution,
                format!("Deleted {deleted} of {total} voters, but the audit entry could not be queued: {err:#}"),
            )
            .await;
        }
    }

    if !failures.is_empty() {
        let shown: Vec<String> = failures
            .iter()
            .take(MAX_REPORTED_FAILURES)
            .cloned()
            .collect();
        let omitted = failures.len().saturating_sub(MAX_REPORTED_FAILURES);
        let tail = if omitted > 0 {
            format!("\n...and {omitted} more")
        } else {
            String::new()
        };
        return fail(
            &task_execution,
            format!(
                "Deleted {deleted} of {total} voters. {} failed:\n{}{tail}",
                failures.len(),
                shown.join("\n")
            ),
        )
        .await;
    }

    if let Some(task_execution) = &task_execution {
        update_complete_with_annotations(
            task_execution,
            json!({"deleted": deleted, "total": total}),
        )
        .await?;
    }
    Ok(())
}

/// The entry of a run's deletions, in the election event's log. The ids are
/// listed up to [`MAX_AUDITED_IDS`]; a larger run records its count.
fn deleted_voters_change(
    task: &TasksExecution,
    actor: Option<AuditActor>,
    deleted_ids: &[&String],
    total: usize,
) -> anyhow::Result<AdminChange> {
    let target = task_audit::target(&task.tenant_id, task.election_event_id.as_deref())?;
    let mut extra = json!({"count": deleted_ids.len(), "total": total, "task_id": task.id});
    if deleted_ids.len() <= MAX_AUDITED_IDS {
        extra["ids"] = json!(deleted_ids);
    }
    Ok(task_audit::change(
        actor,
        target,
        AuditObject::new(VOTER, task_audit::MANY, None),
        AdminAction::Delete,
    )
    .with_extra(extra)
    .with_description(format!("Deleted {} voter accounts", deleted_ids.len())))
}

async fn audit_deleted(
    task_execution: &Option<TasksExecution>,
    actor: Option<AuditActor>,
    deleted_ids: &[&String],
    total: usize,
) -> anyhow::Result<()> {
    let task = task_execution
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing task execution record"))?;
    task_audit::record(deleted_voters_change(task, actor, deleted_ids, total)?).await
}

async fn ensure_election_event_not_locked(task_execution: &Option<TasksExecution>) -> Result<()> {
    let Some(task) = task_execution else {
        return fail(
            task_execution,
            "Delete voters task is missing its task execution record".to_string(),
        )
        .await;
    };
    let Some(election_event_id) = task.election_event_id.as_deref() else {
        return fail(
            task_execution,
            "Delete voters task is missing its election event id".to_string(),
        )
        .await;
    };

    match is_election_event_locked_down(&task.tenant_id, election_event_id).await {
        Ok(false) => Ok(()),
        Ok(true) => fail(task_execution, "Election event is locked down".to_string()).await,
        Err(err) => {
            fail(
                task_execution,
                format!("Failed to check election event lockdown: {err}"),
            )
            .await
        }
    }
}

/// The voters of the task's election event with a valid or in-progress
/// ballot. Deleting one would leave that ballot out of the tally.
async fn voters_with_active_ballots(
    task_execution: &Option<TasksExecution>,
) -> anyhow::Result<HashSet<String>> {
    let task = task_execution
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing task execution record"))?;
    let election_event_id = task
        .election_event_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Missing election event id"))?;
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let states =
        get_voter_ballot_states(&hasura_transaction, &task.tenant_id, election_event_id).await?;
    Ok(states
        .into_iter()
        .filter(|(_, state)| state.has_active_vote())
        .map(|(voter_id, _)| voter_id)
        .collect())
}

/// Marks the task failed and returns the same message as the task error, so the
/// admin portal widget and the logs agree on why it stopped.
#[instrument(skip(task_execution))]
async fn fail(task_execution: &Option<TasksExecution>, message: String) -> Result<()> {
    if let Some(task_execution) = task_execution {
        update_fail(task_execution, &message).await?;
    }
    Err(Error::String(message))
}

/// Pages through the filtered set so a large event is not silently truncated by
/// the per-query row limit. Advances by the number of rows actually returned
/// rather than the requested page size, because the underlying queries clamp the
/// limit to `low_sql_limit`.
///
/// `has_voted` is deliberately routed through `list_users_with_vote_info`:
/// `list_users` does not implement it, and the voters list applies it in
/// application code over the vote info rather than in SQL. Resolving through
/// `list_users` alone would ignore the filter and select every voter in the
/// event, which for a delete is the difference between "the 300 who have not
/// voted" and "everybody".
#[instrument(skip(filter), err)]
async fn resolve_ids(filter: ListUsersFilter) -> Result<Vec<String>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| Error::String(format!("Error getting Hasura DB pool: {err}")))?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| Error::String(format!("Error getting Keycloak DB pool: {err}")))?;
    let keycloak_transaction = keycloak_db_client.transaction().await?;

    let filter_by_has_voted = filter.has_voted;
    let mut seen: HashSet<String> = HashSet::new();
    let mut all: Vec<String> = vec![];
    let mut offset: i32 = filter.offset.unwrap_or(0);
    loop {
        let mut page = filter.clone();
        page.limit = Some(RESOLVE_PAGE_SIZE);
        page.offset = Some(offset);

        let (ids, returned) = match filter_by_has_voted {
            Some(has_voted) => {
                let (users, _) =
                    list_users_with_vote_info(&hasura_transaction, &keycloak_transaction, page)
                        .await?;
                // Page by rows examined, not rows kept, or the offset would
                // stall as soon as a page is fully filtered out.
                let examined = users.len() as i32;
                let ids = users
                    .into_iter()
                    .filter(|user| {
                        let votes = user.votes_info.as_ref().map(|v| v.len()).unwrap_or(0);
                        if has_voted {
                            votes > 0
                        } else {
                            0 == votes
                        }
                    })
                    .filter_map(|user| user.id)
                    .collect::<Vec<String>>();
                (ids, examined)
            }
            None => {
                let (users, _) =
                    list_users(&hasura_transaction, &keycloak_transaction, page).await?;
                let returned = users.len() as i32;
                let ids = users
                    .into_iter()
                    .filter_map(|user| user.id)
                    .collect::<Vec<String>>();
                (ids, returned)
            }
        };

        // The event-scoped query joins user_attribute for the area, so a voter
        // carrying more than one area-id yields the same id twice. Deleting it
        // twice would 404 on the second attempt and fail an otherwise clean run.
        for id in ids {
            if seen.insert(id.clone()) {
                all.push(id);
            }
        }

        if 0 == returned {
            break;
        }
        offset += returned;
    }

    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit_log::AuditOrigin;
    use chrono::Local;

    const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const EVENT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";

    fn task() -> TasksExecution {
        TasksExecution {
            id: "task-id".to_string(),
            tenant_id: TENANT_ID.to_string(),
            election_event_id: Some(EVENT_ID.to_string()),
            name: "Delete Voters".to_string(),
            task_type: "DELETE_VOTERS".to_string(),
            execution_status: "IN_PROGRESS".to_string(),
            created_at: Local::now(),
            start_at: None,
            end_at: None,
            annotations: None,
            labels: None,
            logs: None,
            executed_by_user: "Ada".to_string(),
        }
    }

    fn admin() -> AuditActor {
        AuditActor {
            user_id: Some("admin-id".into()),
            username: Some("ada".into()),
            role: Some("admin-user".into()),
        }
    }

    #[test]
    fn a_run_is_one_entry_in_the_event_log_with_the_deleted_ids() {
        let ids = ["voter-1".to_string(), "voter-2".to_string()];
        let deleted: Vec<&String> = ids.iter().collect();
        let change = deleted_voters_change(&task(), Some(admin()), &deleted, 3).unwrap();

        assert_eq!(
            change.target.election_event_id.map(|id| id.to_string()),
            Some(EVENT_ID.to_string())
        );
        assert_eq!(change.object, AuditObject::new(VOTER, "*", None));
        assert_eq!(change.action, AdminAction::Delete);
        assert_eq!(change.actor, admin());
        assert_eq!(change.origin, AuditOrigin::Windmill);
        assert_eq!(
            change.extra,
            json!({"count": 2, "total": 3, "task_id": "task-id", "ids": ["voter-1", "voter-2"]})
        );
        assert_eq!(change.describe(), "Deleted 2 voter accounts");
    }

    #[test]
    fn a_large_run_records_its_count_without_the_ids() {
        let ids: Vec<String> = (0..=MAX_AUDITED_IDS)
            .map(|n| format!("voter-{n}"))
            .collect();
        let deleted: Vec<&String> = ids.iter().collect();
        let change = deleted_voters_change(&task(), None, &deleted, ids.len()).unwrap();

        assert_eq!(change.extra["count"], MAX_AUDITED_IDS + 1);
        assert!(change.extra.get("ids").is_none());
        assert_eq!(change.actor, AuditActor::default());
    }

    #[test]
    fn a_task_without_a_readable_tenant_cannot_be_audited() {
        let mut task = task();
        task.tenant_id = "tenant".to_string();
        assert!(deleted_voters_change(&task, None, &[], 0).is_err());
    }
}
