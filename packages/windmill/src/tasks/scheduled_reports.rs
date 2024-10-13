use crate::postgres::reports::{get_all_active_reports, Report};
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
// use crate::tasks::process_report::process_report_task;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::{DateTime, Utc, Duration, Local};
use cron::Schedule;
use deadpool_postgres::Client as DbClient;
use std::str::FromStr;
use tracing::{event, info, instrument, Level, error};

/// Parse the next scheduled time for the report using the cron expression.
/// Returns the next run time if it is due within the current time window.
#[instrument]
pub fn get_next_scheduled_time(
    report: &Report,
) -> Option<DateTime<Local>> {

    let now = ISO8601::now();
    let Some(cron_config) = report.cron_config.clone() else {
        return None;
    };
 
    let cron_expression = cron_config.cron_expression.clone();
    info!("cron_expression: {}", cron_expression);
    let schedule = match Schedule::from_str(&cron_expression) {
        Ok(schedule) => schedule,
        Err(err) => {
            error!("Failed to parse cron expression for report id {}: {}", report.id, err);
            return None; // Return early if there's a parsing error
        }
    };

    // Determine the last run time, fall back to created_at if last_document_produced is not set
    let last_run = cron_config
        .last_document_produced
        .unwrap_or_else(|| report.created_at);

    // Get the next scheduled time after the last run
    let next_run = schedule.after(&last_run).next();

    info!("Next run: {:?}", next_run);

    if let Some(next_run) = next_run {
        // Return the next run if it's in the past or due within the next minute
        if next_run <= now {
            return Some(next_run.with_timezone(&Local))
        } else {
            return None
        }
    } else {
        // No next run found
       return None
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

    // Establish a connection to Hasura database
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
        // let task = celery_app
        //     .send_task(
        //         process_report_task::new(report.id.clone())
        //             .with_eta(now)
        //             .with_expires_in(120),
        //     )
        //     .await?;
        
        event!(
            Level::INFO,
            "Scheduled report task with id: {}",
            report.id
        );
    }

    Ok(())
}
