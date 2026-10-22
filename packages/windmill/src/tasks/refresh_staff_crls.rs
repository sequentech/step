// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The job that refreshes the revocation lists of staff issuers
//! ([`crate::services::signing::crl`]). Beat sends it every
//! `--staff-crl-interval` seconds (hourly by default). Each event is
//! refreshed on its own, so one event's failure doesn't stop the others;
//! downloads run outside transactions.

use crate::postgres::signing_certificates::list_events_with_staff_issuers;
use crate::services::database::get_hasura_pool;
use crate::services::signing::crl::{refresh_event_crls, CrlFetcher, HttpCrlFetcher};
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use chrono::Utc;
use deadpool_postgres::Client;
use tracing::{instrument, warn};

/// Refreshes the lists of every event with staff issuers. Returns how many
/// events were refreshed.
pub async fn refresh_all_staff_crls(
    client: &mut Client,
    fetcher: &dyn CrlFetcher,
) -> Result<usize> {
    let events = {
        let transaction = client.transaction().await?;
        list_events_with_staff_issuers(&transaction).await?
    };
    let mut refreshed = 0;
    for (tenant_id, election_event_id) in events {
        match refresh_event_crls(client, tenant_id, election_event_id, fetcher, Utc::now()).await {
            Ok(_) => refreshed += 1,
            Err(err) => {
                warn!(%tenant_id, %election_event_id, error = ?err, "staff CRL refresh failed")
            }
        }
    }
    Ok(refreshed)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(expires = 600)]
pub async fn refresh_staff_crls() -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting hasura client: {error}"))?;
    refresh_all_staff_crls(&mut client, &HttpCrlFetcher::default()).await?;
    Ok(())
}
