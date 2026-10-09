// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::{
    deactivate_report_schedule, get_all_active_reports, update_report_last_document_time, Report,
    ReportType,
};
use crate::postgres::signing_report_release::report_awaits_signatures;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::GenerateReportMode;
use crate::services::reports::template_time::{event_presentation, zone_or_utc};
use crate::services::signing::actions::reports::held_by_the_tally;
use crate::services::tasks_execution;
use crate::tasks::generate_report::generate_report;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use chrono_tz::Tz;
use croner::Cron;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::time_zones::primary_time_zone;
use std::collections::HashMap;
use std::str::FromStr;
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
    deactivate_report_schedule(&hasura_transaction, &report.tenant_id, &report.id).await?;
    hasura_transaction.commit().await?;

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

/// Parse the next scheduled time for the report using the cron expression,
/// read in `zone` (the event's primary zone: "every day at 08:00" is 08:00
/// there, whatever the server's clock zone).
#[instrument]
pub fn get_next_scheduled_time(report: &Report, zone: Tz) -> Option<DateTime<Utc>> {
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
    let next_run = match schedule.find_next_occurrence(&last_run.with_timezone(&zone), false) {
        Ok(next_run) => next_run,
        Err(err) => {
            error!("Error finding next occurence: {err:?}");
            return None;
        }
    };

    info!("Next run: {:?}", next_run);
    return Some(next_run.with_timezone(&Utc));
}

/// The primary zone of each event that has an active report.
async fn report_zones(
    hasura_transaction: &Transaction<'_>,
    reports: &[Report],
) -> HashMap<(String, String), Tz> {
    let mut zones = HashMap::new();
    for report in reports {
        let key = (report.tenant_id.clone(), report.election_event_id.clone());
        if zones.contains_key(&key) {
            continue;
        }
        let zone = match get_election_event_by_id(hasura_transaction, &key.0, &key.1).await {
            Ok(event) => zone_or_utc(&primary_time_zone(Some(&event_presentation(
                event.presentation.as_ref(),
            )))),
            Err(err) => {
                error!(
                    "Error reading the zone of election event {}: {err:?}; scheduling its reports in UTC",
                    key.1
                );
                Tz::UTC
            }
        };
        zones.insert(key, zone);
    }
    zones
}

fn zone_of(zones: &HashMap<(String, String), Tz>, report: &Report) -> Tz {
    zones
        .get(&(report.tenant_id.clone(), report.election_event_id.clone()))
        .copied()
        .unwrap_or(Tz::UTC)
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
    let now = Utc::now();
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
    let zones = report_zones(&hasura_transaction, &active_reports).await;

    // Filter out reports that need to run now based on their cron configuration
    let to_be_run_now = active_reports
        .iter()
        .filter(|report| {
            let Some(formatted_date) = get_next_scheduled_time(&report, zone_of(&zones, report))
            else {
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
        let Some(datetime) = get_next_scheduled_time(report, zone_of(&zones, report)) else {
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

        // A report the tally holds while its rule needs signatures is never
        // generated (nor mailed) on a schedule.
        if let Ok(report_type) = ReportType::from_str(&report.report_type) {
            if held_by_the_tally(
                &hasura_transaction,
                Uuid::parse_str(&report.tenant_id).map_err(|e| anyhow!("{e}"))?,
                Uuid::parse_str(&report.election_event_id).map_err(|e| anyhow!("{e}"))?,
                &report_type,
            )
            .await?
            {
                info!(
                    report_id = %report.id,
                    "skipping a scheduled report the tally holds for its signatures"
                );
                update_report_last_document_time(
                    &hasura_transaction,
                    &report.tenant_id,
                    &report.id,
                )
                .await?;
                continue;
            }
        }
        // A report whose signing request waits keeps that request: its
        // next run is skipped instead of replacing what is being signed.
        if report_awaits_signatures(
            &hasura_transaction,
            Uuid::parse_str(&report.tenant_id).map_err(|e| anyhow!("{e}"))?,
            Uuid::parse_str(&report.election_event_id).map_err(|e| anyhow!("{e}"))?,
            Uuid::parse_str(&report.id).map_err(|e| anyhow!("{e}"))?,
        )
        .await?
        {
            info!(report_id = %report.id, "skipping a report whose signing request waits");
            update_report_last_document_time(&hasura_transaction, &report.tenant_id, &report.id)
                .await?;
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
                    false,
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
    use chrono::TimeZone;
    use sequent_core::election_config::{EReportEncryption, ReportCronConfig};

    fn daily_at_eight(last_document_produced: &str) -> Report {
        Report {
            id: "report".to_string(),
            election_event_id: "event".to_string(),
            tenant_id: "tenant".to_string(),
            election_id: None,
            report_type: "PARTICIPATION_REPORT".to_string(),
            template_alias: None,
            encryption_policy: EReportEncryption::Unencrypted,
            cron_config: Some(ReportCronConfig {
                is_active: true,
                last_document_produced: Some(last_document_produced.to_string()),
                cron_expression: "0 8 * * *".to_string(),
                email_recipients: Vec::new(),
                executer_username: "admin".to_string(),
            }),
            created_at: Utc::now(),
            permission_label: None,
            copies: None,
            output_formats: None,
        }
    }

    #[test]
    fn the_cron_runs_at_the_wall_time_of_the_primary_zone() {
        // 23:00 UTC on 7 April: the next 08:00 depends on the zone.
        let report = daily_at_eight("2028-04-07T23:00:00.000");
        for primary in ["Asia/Manila", "Europe/Madrid", "UTC"] {
            let zone: Tz = primary.parse().unwrap();
            let next = get_next_scheduled_time(&report, zone).unwrap();
            let local = next.with_timezone(&zone);

            let expected_day = if zone.with_ymd_and_hms(2028, 4, 7, 8, 0, 0).unwrap()
                > Utc.with_ymd_and_hms(2028, 4, 7, 23, 0, 0).unwrap()
            {
                7
            } else {
                8
            };
            assert_eq!(
                next,
                zone.with_ymd_and_hms(2028, 4, expected_day, 8, 0, 0)
                    .unwrap()
                    .with_timezone(&Utc),
                "{primary}"
            );
            assert_eq!(local.format("%H:%M").to_string(), "08:00", "{primary}");
        }
    }

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
