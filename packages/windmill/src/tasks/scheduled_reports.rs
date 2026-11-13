// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::{
    deactivate_report_schedule, get_all_active_reports, update_report_last_document_time, Report,
};
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::GenerateReportMode;
use crate::services::tasks_execution;
use crate::tasks::generate_report::generate_report;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use croner::Cron;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use sequent_core::types::permissions::Permissions;
use strum_macros::Display;
use tracing::{error, event, info, instrument, warn, Level};
use uuid::Uuid;

/// The Keycloak state of the user a scheduled report runs as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledReportExecutor {
    pub enabled: bool,
    pub realm_roles: Vec<String>,
}

/// Whether a scheduled report may run, or why it may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum ScheduledReportAuthorization {
    #[strum(to_string = "authorized")]
    Authorized,
    #[strum(to_string = "the schedule's user does not exist in the tenant")]
    ExecutorNotFound,
    #[strum(to_string = "the schedule's user is disabled")]
    ExecutorDisabled,
    #[strum(to_string = "the schedule's user is not allowed to read reports")]
    MissingPermission,
}

/// A scheduled report needs the same permission as generating it from the
/// Reports tab, held by its user at the time of each run.
pub fn authorize_scheduled_report(
    executor: Option<&ScheduledReportExecutor>,
) -> ScheduledReportAuthorization {
    let Some(executor) = executor else {
        return ScheduledReportAuthorization::ExecutorNotFound;
    };
    if !executor.enabled {
        return ScheduledReportAuthorization::ExecutorDisabled;
    }
    let report_read = Permissions::REPORT_READ.to_string();
    if !executor.realm_roles.iter().any(|role| *role == report_read) {
        return ScheduledReportAuthorization::MissingPermission;
    }
    ScheduledReportAuthorization::Authorized
}

/// Looks up the schedule's user in the tenant realm. Returns `None` when the
/// username is empty or no user has it.
#[instrument(err)]
async fn get_scheduled_report_executor(
    tenant_id: &str,
    username: &str,
) -> anyhow::Result<Option<ScheduledReportExecutor>> {
    if username.is_empty() {
        return Ok(None);
    }
    let realm = get_tenant_realm(tenant_id);
    let keycloak = KeycloakAdminClient::new().await?;
    let users = keycloak
        .client
        .realm_users_get(
            &realm,
            Some(true),
            None,
            None,
            None,
            Some(true),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(username.to_string()),
        )
        .await
        .map_err(|err| anyhow!("Error looking up user: {err:?}"))?;
    let Some(user) = users.into_iter().find(|user| {
        user.username
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case(username))
    }) else {
        return Ok(None);
    };
    let user_id = user.id.ok_or_else(|| anyhow!("Keycloak user has no id"))?;
    let realm_roles = keycloak
        .client
        .realm_users_with_user_id_role_mappings_realm_composite_get(&realm, &user_id, Some(true))
        .await
        .map_err(|err| anyhow!("Error reading user roles: {err:?}"))?
        .into_iter()
        .filter_map(|role| role.name)
        .collect();
    Ok(Some(ScheduledReportExecutor {
        enabled: user.enabled.unwrap_or(false),
        realm_roles,
    }))
}

/// Turns off a schedule that failed its check and records a failed task. The
/// deactivation is committed first, so the task never reports a deactivation
/// that was rolled back.
#[instrument(skip(report), err)]
async fn deactivate_unauthorized_schedule(
    report: &Report,
    executer_username: &str,
    authorization: ScheduledReportAuthorization,
) -> anyhow::Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura client: {err}"))?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let deactivated = deactivate_report_schedule(
        &hasura_transaction,
        &report.tenant_id,
        &report.id,
        executer_username,
    )
    .await?;
    hasura_transaction.commit().await?;
    if !deactivated {
        info!(
            "Scheduled report id={id} was changed since it was checked, leaving it as it is",
            id = report.id
        );
        return Ok(());
    }

    let task_execution = tasks_execution::post(
        &report.tenant_id,
        Some(report.election_event_id.as_str()),
        ETasksExecution::GENERATE_REPORT,
        executer_username,
    )
    .await?;
    tasks_execution::update_fail(
        &task_execution,
        &format!("Scheduled report deactivated: {authorization}"),
    )
    .await
}

/// Parse the next scheduled time for the report using the cron expression.
/// Returns the next run time if it is due within the current time window.
#[instrument]
pub fn get_next_scheduled_time(report: &Report) -> Option<DateTime<Local>> {
    let Some(cron_config) = report.cron_config.clone() else {
        return None;
    };
    info!("Cron config: {:?}", cron_config);
    let cron_expression = cron_config.cron_expression.clone();

    let schedule = match Cron::new(&cron_expression).parse() {
        Ok(schedule) => schedule,
        Err(err) => {
            error!(
                "Failed to parse cron expression for report id={id} and cron_expression={cron}: {err}",
                id=report.id, cron=cron_expression, err=err
            );
            return None; // Return early if there's a parsing error
        }
    };

    // TODO: This should NOT be a naive date
    let last_document_produced_date = match &cron_config.last_document_produced {
        Some(date_str) => parse_last_document_produced(date_str),
        None => Some(report.created_at),
    };

    info!(
        "last_document_produced_date: {:?}",
        last_document_produced_date
    );
    let last_run = match last_document_produced_date {
        Some(last_run) => last_run,
        None => {
            error!("No last run date found for report id {}", report.id);
            return None;
        }
    };
    // Get the next scheduled time after the last run
    let next_run = match schedule.find_next_occurrence(&last_run, false) {
        Ok(next_run) => next_run,
        Err(err) => {
            error!("Error finding next occurence: {err:?}");
            return None;
        }
    };

    info!("Next run: {:?}", next_run);
    return Some(next_run.with_timezone(&Local));
}

fn parse_last_document_produced(date_str: &str) -> Option<DateTime<Utc>> {
    let format = "%Y-%m-%dT%H:%M:%S%.f";
    match NaiveDateTime::parse_from_str(date_str, format) {
        Ok(naive_dt) => Some(naive_dt.and_utc()),
        Err(e) => {
            error!("Failed to parse last_document_produced '{date_str}': {e}");
            None
        }
    }
}

/// The Celery task for scheduling reports based on cron configuration.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn scheduled_reports(rate_seconds: u64) -> Result<()> {
    // Get the Celery app for scheduling tasks
    let celery_app = get_celery_app().await;

    // Get the current time
    let now = Local::now();
    let nsecs_later = now + Duration::seconds(rate_seconds as i64);

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client: {e}"))?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    // Fetch all active reports from the database
    let active_reports = get_all_active_reports(&hasura_transaction)
        .await
        .map_err(|err| anyhow!("Error getting all active reports: {err:?}"))?;
    info!("Found {len} active reports", len = active_reports.len());

    // Filter out reports that need to run now based on their cron configuration
    let to_be_run_now = active_reports
        .iter()
        .filter(|report| {
            let Some(formatted_date) = get_next_scheduled_time(&report) else {
                return false;
            };
            formatted_date < nsecs_later
        })
        .collect::<Vec<_>>();
    info!(
        "Found {num} reports to be run now",
        num = to_be_run_now.len()
    );

    // Schedule the task for each report that needs to run
    for report in to_be_run_now {
        let Some(datetime) = get_next_scheduled_time(report) else {
            continue;
        };

        let cron_config = report
            .cron_config
            .clone()
            .ok_or_else(|| anyhow!("Cron config not found"))?;

        let executor =
            match get_scheduled_report_executor(&report.tenant_id, &cron_config.executer_username)
                .await
            {
                Ok(executor) => executor,
                Err(err) => {
                    error!(
                        "Skipping scheduled report id={id}: its user could not be checked: {err:?}",
                        id = report.id
                    );
                    continue;
                }
            };
        let authorization = authorize_scheduled_report(executor.as_ref());
        if authorization != ScheduledReportAuthorization::Authorized {
            warn!(
                "Deactivating scheduled report id={id}: {authorization}",
                id = report.id
            );
            if let Err(err) = deactivate_unauthorized_schedule(
                report,
                &cron_config.executer_username,
                authorization,
            )
            .await
            {
                error!(
                    "Error deactivating scheduled report id={id}: {err:?}",
                    id = report.id
                );
            }
            continue;
        }

        let document_id = Uuid::new_v4().to_string();

        // Create a task execution record for this report generation
        let task_execution = tasks_execution::post(
            &report.tenant_id,
            Some(report.election_event_id.as_str()),
            ETasksExecution::GENERATE_REPORT,
            &cron_config.executer_username,
        )
        .await
        .map_err(|err| anyhow!("Error creating task execution record: {err:?}"))?;

        let _task = celery_app
            .send_task(
                generate_report::new(
                    report.clone(),
                    document_id.clone(),
                    GenerateReportMode::REAL,
                    cron_config.is_active,
                    Some(task_execution),
                    Some(cron_config.executer_username),
                    None,
                )
                .with_eta(datetime.with_timezone(&Utc))
                .with_expires_in(120),
            )
            .await
            .map_err(|err| anyhow!("Error sending generate_report task: {err:?}"))?;

        update_report_last_document_time(&hasura_transaction, &report.tenant_id, &report.id)
            .await
            .map_err(|err| anyhow!("Error updating report last document time: {err:?}"))?;

        event!(
            Level::INFO,
            "Scheduled report task with id: {id}",
            id = report.id
        );
    }

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|err| anyhow!("Error committing hasura transaction: {err:?}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executor(enabled: bool, realm_roles: &[Permissions]) -> ScheduledReportExecutor {
        ScheduledReportExecutor {
            enabled,
            realm_roles: realm_roles.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn scheduled_report_runs_for_enabled_executor_with_report_read() {
        let executor = executor(true, &[Permissions::ADMIN_USER, Permissions::REPORT_READ]);
        assert_eq!(
            authorize_scheduled_report(Some(&executor)),
            ScheduledReportAuthorization::Authorized
        );
    }

    #[test]
    fn scheduled_report_skipped_when_executor_missing() {
        assert_eq!(
            authorize_scheduled_report(None),
            ScheduledReportAuthorization::ExecutorNotFound
        );
    }

    #[test]
    fn scheduled_report_skipped_when_executor_disabled() {
        let executor = executor(false, &[Permissions::REPORT_READ]);
        assert_eq!(
            authorize_scheduled_report(Some(&executor)),
            ScheduledReportAuthorization::ExecutorDisabled
        );
    }

    #[test]
    fn scheduled_report_skipped_when_executor_lacks_report_read() {
        let executor = executor(true, &[Permissions::ADMIN_USER, Permissions::REPORT_WRITE]);
        assert_eq!(
            authorize_scheduled_report(Some(&executor)),
            ScheduledReportAuthorization::MissingPermission
        );
    }
}
