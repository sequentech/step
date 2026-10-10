// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Seals and publishes ballot boxes (VOTE-FREEZE).
//!
//! - **At a close**, after it commits, [`kick_ballot_box_sealer`] runs
//!   [`schedule_ballot_box_seals`], which schedules one [`seal_ballot_box`]
//!   per new pending seal at its deadline (`eta`) when that is at most
//!   [`ETA_HORIZON_MINUTES`] away, so a box is sealed within moments of
//!   its deadline.
//! - **The guarantee:** beat runs the light dispatcher [`seal_ballot_boxes`]
//!   every minute on the Beat queue. It lists the `pending` seals past their
//!   deadline, the `sealed` ones still to publish and the `failed` ones whose
//!   failure entry isn't recorded as posted, and enqueues one
//!   [`seal_ballot_box`] per box.
//!
//! [`seal_ballot_box`] runs on the short queue, which every worker
//! configuration consumes and long tallies don't hold. Every message
//! expires [`SEAL_MESSAGE_EXPIRES_SECONDS`] after it is due (beat sends a
//! new one every minute). A duplicate is harmless: the row and box locks are taken
//! without waiting, and the status recheck makes a second run a no-op.

use crate::postgres::ballot_box_seal::{list_new_pending, list_open_work_ids, BallotBoxSealStatus};
use crate::services::ballot_box_seal::publish::{publish_box, PublishOutcome};
use crate::services::ballot_box_seal::seal::{post_failure, seal_box, SealOutcome};
use crate::services::ballot_box_seal::ProductionSealEnvironment;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use chrono::Utc;
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Seconds between two dispatcher runs.
pub const SEAL_BALLOT_BOXES_INTERVAL_SECONDS: u64 = 60;
/// The most seals of each status one dispatcher run enqueues; the next run
/// takes the rest.
const SEALS_PER_RUN: i64 = 1000;
/// How long a kick may take to reach the broker.
const KICK_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a `seal_ballot_box` message stays valid after it is due.
pub const SEAL_MESSAGE_EXPIRES_SECONDS: i64 = 120;
/// How far back a close's schedule looks for the seals it created.
const NEW_SEALS_WINDOW_MINUTES: i64 = 10;
/// A close schedules a seal at its deadline only when the deadline is this
/// close; a message waiting for its `eta` is held unacked by the worker, and
/// the broker drops consumers that hold one too long. The beat covers later
/// deadlines within a minute of them.
const ETA_HORIZON_MINUTES: i64 = 5;

/// The `seal_ballot_box` message for a seal: due now (beat), or at `eta`
/// (a close's schedule); either way it expires
/// [`SEAL_MESSAGE_EXPIRES_SECONDS`] after it is due.
pub fn seal_message(
    seal_id: &Uuid,
    eta: Option<chrono::DateTime<Utc>>,
) -> celery::task::Signature<seal_ballot_box> {
    let message = seal_ballot_box::new(seal_id.to_string());
    match eta {
        Some(due) => message
            .with_eta(due)
            .with_expires(due + chrono::Duration::seconds(SEAL_MESSAGE_EXPIRES_SECONDS)),
        None => message.with_expires_in(SEAL_MESSAGE_EXPIRES_SECONDS as u32),
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 30, max_retries = 0, expires = 30)]
pub async fn seal_ballot_boxes() -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting the hasura client: {error:?}"))?;
    let transaction = client.transaction().await.map_err(anyhow::Error::from)?;
    let ids = list_open_work_ids(&transaction, Utc::now(), SEALS_PER_RUN).await?;
    transaction.commit().await.map_err(anyhow::Error::from)?;
    if ids.is_empty() {
        return Ok(());
    }
    info!(seals = ids.len(), "Dispatching ballot box seals");
    let celery_app = get_celery_app().await;
    for seal_id in ids {
        if let Err(send_error) = celery_app.send_task(seal_message(&seal_id, None)).await {
            error!(%seal_id, "Error enqueuing the ballot box seal: {send_error:?}");
        }
    }
    Ok(())
}

/// Seals one ballot box if it is pending and due, then publishes it if it
/// is sealed, or posts its failure entry if it failed.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
// No `expires` here: each send sets it (see `seal_message`), since an
// explicit `with_expires` would be ignored next to a task default.
#[celery::task(time_limit = 900, max_retries = 0)]
pub async fn seal_ballot_box(seal_id: String) -> Result<()> {
    let seal_id = Uuid::parse_str(&seal_id).map_err(anyhow::Error::from)?;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting the hasura client: {error:?}"))?;
    let environment = ProductionSealEnvironment;
    let sealed = seal_box(&mut client, &environment, &seal_id).await?;
    info!(%seal_id, ?sealed, "Ballot box seal");
    match sealed {
        SealOutcome::NotFound => return Ok(()),
        SealOutcome::Failed(_) | SealOutcome::NotPending(BallotBoxSealStatus::Failed) => {
            post_failure(&mut client, &environment, &seal_id).await?;
            return Ok(());
        }
        _ => {}
    }
    let published = publish_box(&mut client, &environment, &seal_id).await?;
    if published == PublishOutcome::Published {
        info!(%seal_id, "Ballot box seal published");
    }
    Ok(())
}

/// Schedules one [`seal_ballot_box`] per pending seal created in the last
/// minutes and not tried yet whose deadline is at most
/// [`ETA_HORIZON_MINUTES`] away, at its deadline (`eta`), expiring
/// [`SEAL_MESSAGE_EXPIRES_SECONDS`] after it. Later deadlines are the beat's.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 30, max_retries = 0, expires = 30)]
pub async fn schedule_ballot_box_seals() -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting the hasura client: {error:?}"))?;
    let transaction = client.transaction().await.map_err(anyhow::Error::from)?;
    let since = Utc::now() - chrono::Duration::minutes(NEW_SEALS_WINDOW_MINUTES);
    let horizon = Utc::now() + chrono::Duration::minutes(ETA_HORIZON_MINUTES);
    let seals: Vec<_> = list_new_pending(&transaction, since)
        .await?
        .into_iter()
        .filter(|(_, deadline)| *deadline <= horizon)
        .collect();
    transaction.commit().await.map_err(anyhow::Error::from)?;
    if seals.is_empty() {
        return Ok(());
    }
    info!(
        seals = seals.len(),
        "Scheduling ballot box seals at their deadlines"
    );
    let celery_app = get_celery_app().await;
    for (seal_id, deadline) in seals {
        let due = deadline.max(Utc::now());
        if let Err(send_error) = celery_app
            .send_task(seal_message(&seal_id, Some(due)))
            .await
        {
            error!(%seal_id, "Error scheduling the ballot box seal: {send_error:?}");
        }
    }
    Ok(())
}

/// Asks a worker to schedule the seals a close made, after it commits. Fire
/// and forget: it never fails or panics, even with the broker down; beat
/// runs the dispatcher anyway.
pub fn kick_ballot_box_sealer() -> JoinHandle<()> {
    tokio::spawn(async move {
        let send = async {
            get_celery_app()
                .await
                .send_task(schedule_ballot_box_seals::new())
                .await
                .map(|_| ())
                .map_err(|error| anyhow!("{error:?}"))
        };
        match tokio::time::timeout(KICK_TIMEOUT, AssertUnwindSafe(send).catch_unwind()).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => warn!("Error kicking the ballot box sealer: {error:#}"),
            Ok(Err(_)) => warn!("Kicking the ballot box sealer panicked"),
            Err(_) => warn!("Kicking the ballot box sealer timed out"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use celery::protocol::Message;

    #[test]
    fn every_seal_message_expires_after_it_is_due() {
        let seal_id = Uuid::new_v4();
        let before = Utc::now();
        let now: Message = seal_message(&seal_id, None).try_into().unwrap();
        let expires = now.headers.expires.unwrap();
        assert!(expires >= before + chrono::Duration::seconds(SEAL_MESSAGE_EXPIRES_SECONDS));
        assert!(now.headers.eta.is_none());

        let due = Utc::now() + chrono::Duration::minutes(5);
        let later: Message = seal_message(&seal_id, Some(due)).try_into().unwrap();
        assert_eq!(later.headers.eta, Some(due));
        assert_eq!(
            later.headers.expires,
            Some(due + chrono::Duration::seconds(SEAL_MESSAGE_EXPIRES_SECONDS))
        );
    }
}
