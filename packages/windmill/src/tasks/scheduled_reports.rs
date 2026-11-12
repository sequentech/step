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
use celery::{error::TaskError, task::Signature, Celery};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use chrono_tz::Tz;
use croner::Cron;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use sequent_core::time_zones::primary_time_zone;
use sequent_core::types::permissions::Permissions;
use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;
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

    let mut hasura_transaction = hasura_db_client.transaction().await?;

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
    let scheduling = CeleryReportScheduling {
        sender: &celery_app,
        zones: &zones,
    };
    schedule_due_reports(&mut hasura_transaction, &to_be_run_now, &scheduling).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|err| anyhow!("Error committing hasura transaction: {err:?}"))?;

    Ok(())
}

/// Schedules one due report inside `hasura_transaction`, as each tick of
/// `scheduled_reports` does for every due report.
pub trait ReportScheduling: Sync {
    fn schedule(
        &self,
        hasura_transaction: &Transaction<'_>,
        report: &Report,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Sends a due report's `generate_report` task, reading its cron in the
/// primary zone of its event.
struct CeleryReportScheduling<'a, S: GenerateReportSender> {
    sender: &'a S,
    zones: &'a HashMap<(String, String), Tz>,
}

/// Where the `generate_report` tasks of the due reports are sent.
pub trait GenerateReportSender: Sync {
    fn send(
        &self,
        task: Signature<generate_report>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

impl GenerateReportSender for Arc<Celery> {
    async fn send(&self, task: Signature<generate_report>) -> anyhow::Result<()> {
        self.send_task(task)
            .await
            .map(|_task| ())
            .map_err(|err| anyhow!("Error sending generate_report task: {err:?}"))
    }
}

impl<S: GenerateReportSender> ReportScheduling for CeleryReportScheduling<'_, S> {
    async fn schedule(&self, hasura_transaction: &Transaction<'_>, report: &Report) -> Result<()> {
        let Some(datetime) = get_next_scheduled_time(report, zone_of(self.zones, report)) else {
            return Ok(());
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
                    return Ok(());
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
            return Ok(());
        }

        // A report the tally holds while its rule needs signatures is never
        // generated (nor mailed) on a schedule.
        if let Ok(report_type) = ReportType::from_str(&report.report_type) {
            if held_by_the_tally(
                hasura_transaction,
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
                update_report_last_document_time(hasura_transaction, &report.tenant_id, &report.id)
                    .await?;
                return Ok(());
            }
        }
        // A report whose signing request waits keeps that request: its
        // next run is skipped instead of replacing what is being signed.
        if report_awaits_signatures(
            hasura_transaction,
            Uuid::parse_str(&report.tenant_id).map_err(|e| anyhow!("{e}"))?,
            Uuid::parse_str(&report.election_event_id).map_err(|e| anyhow!("{e}"))?,
            Uuid::parse_str(&report.id).map_err(|e| anyhow!("{e}"))?,
        )
        .await?
        {
            info!(report_id = %report.id, "skipping a report whose signing request waits");
            update_report_last_document_time(hasura_transaction, &report.tenant_id, &report.id)
                .await?;
            return Ok(());
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

        self.sender
            .send(
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
            .await?;

        update_report_last_document_time(hasura_transaction, &report.tenant_id, &report.id)
            .await
            .map_err(|err| anyhow!("Error updating report last document time: {err:?}"))?;

        event!(
            Level::INFO,
            "Scheduled report task with id: {id}",
            id = report.id
        );

        Ok(())
    }
}

/// Schedules every due report in a savepoint of its own. A report that cannot
/// be scheduled is logged and rolled back to its savepoint, so that the other
/// due reports are still scheduled and keep their last document time. Returns
/// how many failed.
pub async fn schedule_due_reports(
    hasura_transaction: &mut Transaction<'_>,
    reports: &[&Report],
    scheduling: &impl ReportScheduling,
) -> Result<usize> {
    let mut failed = 0;
    for report in reports {
        let savepoint = hasura_transaction
            .transaction()
            .await
            .context("Error creating savepoint")?;
        match scheduling.schedule(&savepoint, report).await {
            Ok(()) => savepoint
                .commit()
                .await
                .context("Error releasing savepoint")?,
            Err(err) => {
                failed += 1;
                error!("Error scheduling report id={id}: {err:?}", id = report.id);
                savepoint
                    .rollback()
                    .await
                    .context("Error rolling back savepoint")?;
            }
        }
    }
    if failed > 0 {
        warn!("{failed} scheduled reports failed this tick");
    }
    Ok(failed)
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
    fn a_report_without_a_cron_config_has_no_next_run() {
        let report = Report {
            cron_config: None,
            ..daily_at_eight("2028-04-07T23:00:00.000")
        };
        assert_eq!(get_next_scheduled_time(&report, Tz::UTC), None);
    }

    #[test]
    fn a_report_with_an_invalid_cron_expression_has_no_next_run() {
        let mut report = daily_at_eight("2028-04-07T23:00:00.000");
        report
            .cron_config
            .as_mut()
            .expect("a cron config")
            .cron_expression = "not a cron".to_string();
        assert_eq!(get_next_scheduled_time(&report, Tz::UTC), None);
    }

    #[test]
    fn a_report_with_an_unreadable_last_document_time_has_no_next_run() {
        let report = daily_at_eight("yesterday");
        assert_eq!(get_next_scheduled_time(&report, Tz::UTC), None);
    }

    #[test]
    fn a_report_never_produced_runs_after_its_creation() {
        let mut report = daily_at_eight("2028-04-07T23:00:00.000");
        report
            .cron_config
            .as_mut()
            .expect("a cron config")
            .last_document_produced = None;
        report.created_at = Utc.with_ymd_and_hms(2028, 4, 7, 23, 0, 0).unwrap();
        assert_eq!(
            get_next_scheduled_time(&report, Tz::UTC),
            Some(Utc.with_ymd_and_hms(2028, 4, 8, 8, 0, 0).unwrap())
        );
    }

    #[test]
    fn the_last_document_time_is_read_in_utc() {
        let expected = Utc.with_ymd_and_hms(2028, 4, 7, 23, 30, 15).unwrap();
        assert_eq!(
            parse_last_document_produced("2028-04-07T23:30:15.000"),
            Some(expected)
        );
        assert_eq!(parse_last_document_produced("2028-04-07 23:30"), None);
    }

    #[test]
    fn an_event_without_a_known_zone_is_scheduled_in_utc() {
        let report = daily_at_eight("2028-04-07T23:00:00.000");
        let mut zones = HashMap::new();
        assert_eq!(zone_of(&zones, &report), Tz::UTC);
        let manila: Tz = "Asia/Manila".parse().expect("a zone");
        zones.insert(
            (report.tenant_id.clone(), report.election_event_id.clone()),
            manila,
        );
        assert_eq!(zone_of(&zones, &report), manila);
    }

    #[tokio::test]
    async fn a_schedule_without_a_username_has_no_executor() {
        let executor = get_scheduled_report_executor("tenant", "")
            .await
            .expect("an empty username needs no lookup");
        assert_eq!(executor, None);
    }

    #[test]
    fn the_authorization_of_a_schedule_reads_as_a_reason() {
        assert_eq!(
            ScheduledReportAuthorization::Authorized.to_string(),
            "authorized"
        );
        assert_eq!(
            ScheduledReportAuthorization::ExecutorNotFound.to_string(),
            "the schedule's user does not exist in the tenant"
        );
        assert_eq!(
            ScheduledReportAuthorization::ExecutorDisabled.to_string(),
            "the schedule's user is disabled"
        );
        assert_eq!(
            ScheduledReportAuthorization::MissingPermission.to_string(),
            "the schedule's user is not allowed to read reports"
        );
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

    /// Records each report it schedules in the `scheduled` table. A report
    /// whose id starts with "failing" then runs a statement that fails, which
    /// aborts the transaction until its savepoint is rolled back.
    struct RecordingScheduling;

    impl ReportScheduling for RecordingScheduling {
        async fn schedule(
            &self,
            hasura_transaction: &Transaction<'_>,
            report: &Report,
        ) -> Result<()> {
            hasura_transaction
                .execute(
                    "INSERT INTO scheduled (report_id) VALUES ($1)",
                    &[&report.id],
                )
                .await
                .expect("the transaction is usable for every report");
            if report.id.starts_with("failing") {
                let error = hasura_transaction
                    .batch_execute("SELECT 1 / 0")
                    .await
                    .expect_err("a division by zero");
                return Err(anyhow::Error::from(error).into());
            }
            if report.id.starts_with("aborting") {
                hasura_transaction
                    .batch_execute("SELECT 1 / 0")
                    .await
                    .expect_err("a division by zero");
            }
            Ok(())
        }
    }

    async fn test_client() -> deadpool_postgres::Object {
        let pool = crate::services::database::PgConfig::from_env()
            .expect("HASURA_DB__* names the test PostgreSQL server")
            .hasura_db
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .expect("a test database pool");
        pool.get().await.expect("a test database connection")
    }

    /// Never asked to send: the reports it is given are skipped before that.
    struct UnusedSender;

    impl GenerateReportSender for UnusedSender {
        async fn send(&self, _task: Signature<generate_report>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_report_without_a_cron_config_is_not_sent() {
        let mut client = test_client().await;
        let transaction = client.transaction().await.expect("a transaction");
        let report = Report {
            cron_config: None,
            ..daily_at_eight("2028-04-07T23:00:00.000")
        };
        let zones = HashMap::new();
        let scheduling = CeleryReportScheduling {
            sender: &UnusedSender,
            zones: &zones,
        };

        scheduling
            .schedule(&transaction, &report)
            .await
            .expect("a report without a next run is skipped");
        transaction.rollback().await.expect("a rollback");
    }

    #[tokio::test]
    async fn a_transaction_that_cannot_open_a_savepoint_fails_the_tick() {
        let mut client = test_client().await;
        let mut transaction = client.transaction().await.expect("a transaction");
        transaction
            .batch_execute("SELECT 1 / 0")
            .await
            .expect_err("a division by zero aborts the transaction");
        let report = daily_at_eight("2028-04-07T23:00:00.000");

        let error = schedule_due_reports(&mut transaction, &[&report], &RecordingScheduling)
            .await
            .expect_err("no savepoint can be created");

        assert!(format!("{error:?}").contains("Error creating savepoint"));
        transaction.rollback().await.expect("a rollback");
    }

    #[tokio::test]
    async fn a_report_that_aborts_the_transaction_fails_the_tick() {
        let mut client = test_client().await;
        let mut transaction = client.transaction().await.expect("a transaction");
        transaction
            .batch_execute(
                "CREATE TEMPORARY TABLE scheduled (n serial, report_id text) ON COMMIT DROP",
            )
            .await
            .expect("a table for the scheduled reports");
        let report = Report {
            id: "aborting".to_string(),
            ..daily_at_eight("2028-04-07T23:00:00.000")
        };

        let error = schedule_due_reports(&mut transaction, &[&report], &RecordingScheduling)
            .await
            .expect_err("the savepoint cannot be released");

        assert!(format!("{error:?}").contains("Error releasing savepoint"));
        transaction.rollback().await.expect("a rollback");
    }

    #[tokio::test]
    async fn a_report_that_cannot_be_scheduled_is_rolled_back_alone() {
        let mut client = test_client().await;
        let mut transaction = client.transaction().await.expect("a transaction");
        transaction
            .batch_execute(
                "CREATE TEMPORARY TABLE scheduled (n serial, report_id text) ON COMMIT DROP",
            )
            .await
            .expect("a table for the scheduled reports");
        let reports = ["first", "failing", "third", "failing-too", "last"].map(|id| Report {
            id: id.to_string(),
            ..daily_at_eight("2028-04-07T23:00:00.000")
        });
        let due: Vec<&Report> = reports.iter().collect();

        let failed = schedule_due_reports(&mut transaction, &due, &RecordingScheduling)
            .await
            .expect("the failures do not stop the tick");

        assert_eq!(failed, 2);
        let scheduled: Vec<String> = transaction
            .query("SELECT report_id FROM scheduled ORDER BY n", &[])
            .await
            .expect("the scheduled reports")
            .iter()
            .map(|row| row.get(0))
            .collect();
        assert_eq!(scheduled, ["first", "third", "last"]);
        transaction.rollback().await.expect("a rollback");
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
