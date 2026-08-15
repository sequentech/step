// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::tasks_execution::{update_complete_with_annotations, update_fail};
use crate::services::users::{list_users_ids, list_users_with_vote_info, ListUsersFilter};
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use tracing::{info, instrument};

/// Page size when resolving a whole filtered set to ids. `list_users_ids`
/// clamps the limit to `low_sql_limit`, so the loop below must not assume it
/// gets the page size it asked for.
const RESOLVE_PAGE_SIZE: i32 = 1000;

/// Failures recorded verbatim on the task before the rest are summarised, so a
/// wholesale failure cannot blow up the task_execution row.
const MAX_REPORTED_FAILURES: usize = 50;

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
) -> Result<()> {
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
    let mut deleted: usize = 0;
    for id in &ids {
        match client.delete_user(&realm, id).await {
            Ok(_) => deleted += 1,
            Err(err) => failures.push(format!("{id}: {err}")),
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
/// `list_users_ids` does not implement it, and the voters list applies it in
/// application code over the vote info rather than in SQL. Resolving through
/// `list_users_ids` alone would ignore the filter and select every voter in the
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
                let ids = list_users_ids(&hasura_transaction, &keycloak_transaction, page).await?;
                let returned = ids.len() as i32;
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
