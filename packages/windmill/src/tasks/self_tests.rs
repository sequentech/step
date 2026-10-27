// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use celery::error::TaskError;
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn run_self_tests(
    tenant_id: String,
    election_event_id: String,
    task_id: String,
) -> Result<()> {
    crate::services::self_tests::execute(&tenant_id, &election_event_id, &task_id).await?;
    Ok(())
}
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 15)]
pub async fn schedule_self_tests() -> Result<()> {
    use crate::services::{database::get_hasura_pool, electoral_log_audit_deadline::AuditDeadline};
    AuditDeadline::after(std::time::Duration::from_secs(40))
        .run(async {
            let mut c = get_hasura_pool().await.get().await?;
            let tx = c.transaction().await?;
            let acquired: bool = tx
                .query_one("SELECT pg_try_advisory_xact_lock(848527,3)", &[])
                .await?
                .try_get(0)?;
            if acquired {
                crate::services::self_tests::periodic().await?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await
        .map_err(anyhow::Error::from)??;
    Ok(())
}

#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 15)]
pub async fn reconcile_security_alerts() -> Result<()> {
    use crate::services::{
        audit_storage, database::get_hasura_pool, electoral_log_audit_deadline::AuditDeadline,
        security_notifications,
    };
    AuditDeadline::after(std::time::Duration::from_secs(45))
        .run(async {
            let mut c = get_hasura_pool().await.get().await?;
            let tx = c.transaction().await?;
            let acquired: bool = tx
                .query_one("SELECT pg_try_advisory_xact_lock(848527,2)", &[])
                .await?
                .try_get(0)?;
            if acquired {
                audit_storage::reconcile_alerts().await?;
                security_notifications::dispatch_pending().await?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await
        .map_err(anyhow::Error::from)??;
    Ok(())
}
