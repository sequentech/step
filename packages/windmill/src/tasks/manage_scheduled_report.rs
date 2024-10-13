// use crate::postgres::reports::Report;
// use crate::services::celery_app::get_celery_app;
// use crate::services::pg_lock::PgLock;
// use crate::services::database::get_hasura_pool;
// use crate::services::date::ISO8601;
// use crate::services::reports::template_renderer::ReportType;
// use crate::types::error::{Error, Result};
// use chrono::Duration;
// use deadpool_postgres::Client as DbClient;
// use deadpool_postgres::Transaction;
// use serde::{Deserialize, Serialize};
// use tracing::instrument;
// use tracing::{event, info, Level};
// use uuid::Uuid;

// pub async fn proccess_report_wrapped(
//     hasura_transaction: &Transaction<'_>,
//     report: Report,
// ) -> AnyhowResult<()> {
//     let celery_app = get_celery_app().await;
//     let document_id: String = Uuid::new_v4().to_string();
//     match report.report_type {
//         ReportType::BALLOT_RECEIPT => {

//         } 
//     }
// }

// #[instrument(err)]
// #[wrap_map_err::wrap_map_err(TaskError)]
// #[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
// pub async fn manage_schedulued_report(
//     report: Report,
// ) -> Result<()> {
//     let lock: PgLock = PgLock::acquire(
//         format!(
//             "execute_manage_election_event_date-{}-{}-{}",
//             tenant_id, election_event_id, scheduled_event_id
//         ),
//         Uuid::new_v4().to_string(),
//         ISO8601::now() + Duration::seconds(120),
//     )
//     .await?;
//     let mut hasura_db_client: DbClient = get_hasura_pool()
//         .await
//         .get()
//         .await
//         .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
//     let hasura_transaction = hasura_db_client.transaction().await?;
    

//     match res {
//         Ok(data) => {
//             let commit = hasura_transaction
//                 .commit()
//                 .await
//                 .map_err(|e| anyhow!("Commit failed manage_event_election_dates: {}", e));
//             lock.release().await?;
//             commit?;
//         }
//         Err(err) => {
//             let rollback = hasura_transaction.rollback().await;
//             lock.release().await?;
//             rollback?;
//             return Err(anyhow!("{}", err).into());
//         }
//     }

//     Ok(())
// }