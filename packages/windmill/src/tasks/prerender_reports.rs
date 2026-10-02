// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use celery::error::TaskError;
/// Beat schedules this on the reports queue. Database SKIP LOCKED leases make
/// overlapping runs safe and recover jobs if a worker crashes mid-render.
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 240, max_retries = 0)]
pub async fn prerender_reports() -> Result<()> {
    crate::services::reports::prerender::warm_next()
        .await
        .map_err(crate::types::error::Error::from)?;
    Ok(())
}
