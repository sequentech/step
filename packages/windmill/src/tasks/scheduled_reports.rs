// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::{get_all_active_reports, update_report_last_document_time, Report};
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::GenerateReportMode;
use crate::services::tasks_execution;
use crate::tasks::generate_report::generate_report;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use celery::{error::TaskError, task::Signature, Celery};
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use croner::Cron;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::TasksExecution;
use std::future::Future;
use tracing::{error, event, info, instrument, Level};
use uuid::Uuid;

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

async fn schedule_report(
    celery_app: &Celery,
    hasura_transaction: &Transaction<'_>,
    report: &Report,
) -> Result<()> {
    let Some(datetime) = get_next_scheduled_time(report) else {
        return Ok(());
    };
    let dispatch = CeleryDispatch {
        celery_app,
        hasura_transaction,
    };
    dispatch_report(report, datetime.with_timezone(&Utc), &dispatch).await
}

/// The steps of scheduling a due report that write or send something.
pub trait ReportDispatch: Sync {
    /// Records the report's run as its last document time, in the report's
    /// savepoint.
    fn record_run(&self, report: &Report) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Creates the task execution record of the report's generation.
    fn create_execution(
        &self,
        report: &Report,
        username: &str,
    ) -> impl Future<Output = anyhow::Result<TasksExecution>> + Send;

    /// Sends the report's `generate_report` task.
    fn send(
        &self,
        task: Signature<generate_report>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Marks a task execution record failed.
    fn fail_execution(
        &self,
        task_execution: &TasksExecution,
        message: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// Dispatches through the report's savepoint and the Celery broker.
struct CeleryDispatch<'a, 't> {
    celery_app: &'a Celery,
    hasura_transaction: &'a Transaction<'t>,
}

impl ReportDispatch for CeleryDispatch<'_, '_> {
    async fn record_run(&self, report: &Report) -> anyhow::Result<()> {
        update_report_last_document_time(self.hasura_transaction, &report.tenant_id, &report.id)
            .await
    }

    async fn create_execution(
        &self,
        report: &Report,
        username: &str,
    ) -> anyhow::Result<TasksExecution> {
        tasks_execution::post(
            &report.tenant_id,
            Some(report.election_event_id.as_str()),
            ETasksExecution::GENERATE_REPORT,
            username,
        )
        .await
    }

    async fn send(&self, task: Signature<generate_report>) -> anyhow::Result<()> {
        self.celery_app
            .send_task(task)
            .await
            .map(|_task| ())
            .map_err(|err| anyhow!("{err:?}"))
    }

    async fn fail_execution(
        &self,
        task_execution: &TasksExecution,
        message: &str,
    ) -> anyhow::Result<()> {
        tasks_execution::update_fail(task_execution, message).await
    }
}

/// Schedules a due report: records its run, creates its task execution record
/// and sends its task, in that order. The run is recorded first so that a
/// failed send rolls it back with the report's savepoint, and a sent task
/// always has its run recorded. The execution record is written outside the
/// savepoint, so it is marked failed when its task cannot be sent.
async fn dispatch_report(
    report: &Report,
    datetime: DateTime<Utc>,
    dispatch: &impl ReportDispatch,
) -> Result<()> {
    let cron_config = report
        .cron_config
        .clone()
        .ok_or_else(|| anyhow!("Cron config not found"))?;

    dispatch
        .record_run(report)
        .await
        .context("Error updating report last document time")?;

    let task_execution = dispatch
        .create_execution(report, &cron_config.executer_username)
        .await
        .context("Error creating task execution record")?;

    let document_id = Uuid::new_v4().to_string();
    let task = generate_report::new(
        report.clone(),
        document_id,
        GenerateReportMode::REAL,
        cron_config.is_active,
        Some(task_execution.clone()),
        Some(cron_config.executer_username),
        None,
        false,
    )
    .with_eta(datetime)
    .with_expires_in(120);

    if let Err(err) = dispatch.send(task).await {
        if let Err(fail_err) = dispatch
            .fail_execution(
                &task_execution,
                "The scheduled report task could not be sent",
            )
            .await
        {
            error!(
                "Error marking task execution id={id} failed: {fail_err:?}",
                id = task_execution.id
            );
        }
        return Err(err.context("Error sending generate_report task").into());
    }

    event!(
        Level::INFO,
        "Scheduled report task with id: {id}",
        id = report.id
    );
    Ok(())
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

    let mut hasura_transaction = hasura_db_client.transaction().await?;

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
        let savepoint = hasura_transaction
            .transaction()
            .await
            .map_err(|err| anyhow!("Error creating savepoint: {err:?}"))?;
        match schedule_report(&celery_app, &savepoint, report).await {
            Ok(()) => savepoint
                .commit()
                .await
                .map_err(|err| anyhow!("Error releasing savepoint: {err:?}"))?,
            Err(err) => {
                error!("Error scheduling report id={id}: {err:?}", id = report.id);
                savepoint
                    .rollback()
                    .await
                    .map_err(|err| anyhow!("Error rolling back savepoint: {err:?}"))?;
            }
        }
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
    use crate::postgres::reports::ReportCronConfig;
    use crate::services::reports::template_renderer::EReportEncryption;
    use chrono::Local;
    use std::sync::Mutex;

    fn due_report() -> Report {
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
                last_document_produced: None,
                cron_expression: "0 8 * * *".to_string(),
                email_recipients: Vec::new(),
                executer_username: "admin".to_string(),
            }),
            created_at: Utc::now(),
            permission_label: None,
        }
    }

    /// Records each step it is asked for, and fails the steps it is told to.
    #[derive(Default)]
    struct FakeDispatch {
        fail_record: bool,
        fail_send: bool,
        steps: Mutex<Vec<String>>,
    }

    impl FakeDispatch {
        fn step(&self, step: String) {
            self.steps.lock().unwrap().push(step);
        }

        fn steps(&self) -> Vec<String> {
            self.steps.lock().unwrap().clone()
        }
    }

    impl ReportDispatch for FakeDispatch {
        async fn record_run(&self, report: &Report) -> anyhow::Result<()> {
            self.step(format!("record {}", report.id));
            if self.fail_record {
                return Err(anyhow!("the report row is locked"));
            }
            Ok(())
        }

        async fn create_execution(
            &self,
            report: &Report,
            username: &str,
        ) -> anyhow::Result<TasksExecution> {
            self.step(format!("create {username}"));
            Ok(TasksExecution {
                id: "execution".to_string(),
                tenant_id: report.tenant_id.clone(),
                election_event_id: Some(report.election_event_id.clone()),
                name: "Generate Report".to_string(),
                task_type: ETasksExecution::GENERATE_REPORT.to_string(),
                execution_status: "IN_PROGRESS".to_string(),
                created_at: Local::now(),
                start_at: None,
                end_at: None,
                annotations: None,
                labels: None,
                logs: None,
                executed_by_user: username.to_string(),
            })
        }

        async fn send(&self, _task: Signature<generate_report>) -> anyhow::Result<()> {
            self.step("send".to_string());
            if self.fail_send {
                return Err(anyhow!("the broker is down"));
            }
            Ok(())
        }

        async fn fail_execution(
            &self,
            task_execution: &TasksExecution,
            _message: &str,
        ) -> anyhow::Result<()> {
            self.step(format!("fail {}", task_execution.id));
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_due_report_has_its_run_recorded_before_its_task_is_sent() {
        let dispatch = FakeDispatch::default();

        dispatch_report(&due_report(), Utc::now(), &dispatch)
            .await
            .expect("the report is scheduled");

        assert_eq!(dispatch.steps(), ["record report", "create admin", "send"]);
    }

    #[tokio::test]
    async fn a_report_whose_run_cannot_be_recorded_is_not_sent() {
        let dispatch = FakeDispatch {
            fail_record: true,
            ..FakeDispatch::default()
        };

        let error = dispatch_report(&due_report(), Utc::now(), &dispatch)
            .await
            .expect_err("the run cannot be recorded");

        assert!(format!("{error:?}").contains("Error updating report last document time"));
        assert_eq!(dispatch.steps(), ["record report"]);
    }

    #[tokio::test]
    async fn a_report_whose_task_cannot_be_sent_has_its_execution_marked_failed() {
        let dispatch = FakeDispatch {
            fail_send: true,
            ..FakeDispatch::default()
        };

        let error = dispatch_report(&due_report(), Utc::now(), &dispatch)
            .await
            .expect_err("the task cannot be sent");

        assert!(format!("{error:?}").contains("Error sending generate_report task"));
        assert_eq!(
            dispatch.steps(),
            ["record report", "create admin", "send", "fail execution"]
        );
    }

    #[tokio::test]
    async fn a_report_without_a_cron_config_is_not_dispatched() {
        let dispatch = FakeDispatch::default();
        let report = Report {
            cron_config: None,
            ..due_report()
        };

        dispatch_report(&report, Utc::now(), &dispatch)
            .await
            .expect_err("there is no cron config");

        assert!(dispatch.steps().is_empty());
    }
}
