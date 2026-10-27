// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::attack_heuristics::{self, AuthFailure};
use crate::types::error::Result;
use celery::error::TaskError;
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(
    max_retries = 10,
    max_retry_delay = 30,
    retry_for_unexpected = true,
    acks_late = true
)]
pub async fn record_attack_auth_failure(input: AuthFailure) -> Result<()> {
    crate::services::electoral_log_audit_deadline::AuditDeadline::database()
        .run(attack_heuristics::record_auth(&input))
        .await
        .map_err(anyhow::Error::from)??;
    Ok(())
}
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 15)]
pub async fn evaluate_attack_activity() -> Result<()> {
    crate::services::electoral_log_audit_deadline::AuditDeadline::after(
        std::time::Duration::from_secs(20),
    )
    .run(async {
        let mut db = crate::services::database::get_hasura_pool()
            .await
            .get()
            .await?;
        let tx = db.transaction().await?;
        let acquired: bool = tx
            .query_one("SELECT pg_try_advisory_xact_lock(848527,5)", &[])
            .await?
            .get(0);
        if acquired {
            attack_heuristics::reconcile().await?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(anyhow::Error::from)??;
    Ok(())
}
