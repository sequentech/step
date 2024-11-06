// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::{get_all_active_reports, update_report_last_document_time, Report};
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::reports::template_renderer::GenerateReportMode;
use crate::tasks::generate_report::generate_report;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::date::ISO8601;
// use crate::tasks::process_report::process_report_task;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use cron::Schedule;
use std::str::FromStr;
use tracing::{error, event, info, instrument, Level};
use uuid::Uuid;

/// Parse the next scheduled time for the report using the cron expression.
/// Returns the next run time if it is due within the current time window.
#[instrument]
pub fn get_next_scheduled_time(report: &Report) -> Option<DateTime<Local>> {
    let now = ISO8601::now();
    let Some(cron_config) = report.cron_config.clone() else {
        return None;
    };

    let cron_expression = cron_config.cron_expression.clone();

    let schedule = match Schedule::from_str(&cron_expression) {
        Ok(schedule) => schedule,
        Err(err) => {
            error!(
                "Failed to parse cron expression for report id {}: {}",
                report.id, err
            );
            return None; // Return early if there's a parsing error
        }
    };
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
    let next_run = schedule.after(&last_run).next();

    info!("Next run: {:?}", next_run);

    if let Some(next_run) = next_run {
        // Return the next run if it's in the past or due within the next minute
        if next_run <= now {
            return Some(next_run.with_timezone(&Local));
        } else {
            return None;
        }
    } else {
        // No next run found
        return None;
    }
}

fn parse_last_document_produced(date_str: &str) -> Option<DateTime<Utc>> {
    let format = "%Y-%m-%dT%H:%M:%S%.f";
    match NaiveDateTime::parse_from_str(date_str, format) {
        Ok(naive_dt) => Some(naive_dt.and_utc()),
        Err(e) => {
            error!(
                "Failed to parse last_document_produced '{}': {}",
                date_str, e
            );
            None
        }
    }
}

/// The Celery task for scheduling reports based on cron configuration.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn scheduled_reports() -> Result<()> {
    // Get the Celery app for scheduling tasks
    let celery_app = get_celery_app().await;

    // Get the current time
    let now = ISO8601::now();
    let one_minute_later = now + Duration::minutes(1);

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client: {e}"))?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    // Fetch all active reports from the database
    let active_reports = get_all_active_reports(&hasura_transaction).await?;
    info!("Found {} active reports", active_reports.len());

    // Filter out reports that need to run now based on their cron configuration
    let to_be_run_now = active_reports
        .iter()
        .filter(|report| {
            let Some(formatted_date) = get_next_scheduled_time(&report) else {
                return false;
            };
            formatted_date < one_minute_later
        })
        .collect::<Vec<_>>();

    info!("Found {} reports to be run now", to_be_run_now.len());
    // Schedule the task for each report that needs to run
    for report in to_be_run_now {
        let Some(datetime) = get_next_scheduled_time(report) else {
            continue;
        };

        let document_id = Uuid::new_v4().to_string();
        let task = celery_app
            .send_task(
                generate_report::new(
                    report.clone(),
                    document_id.clone(),
                    GenerateReportMode::REAL,
                    /*is_scheduled_task*/ true,
                )
                .with_eta(datetime.with_timezone(&Utc))
                .with_expires_in(120),
            )
            .await?;

        update_report_last_document_time(&hasura_transaction, &report.tenant_id, &report.id)
            .await?;

        event!(Level::INFO, "Scheduled report task with id: {}", report.id);
    }

    let _commit = hasura_transaction.commit().await?;

    Ok(())
}
