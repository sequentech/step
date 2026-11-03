// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Reads the voters' stored ballots from the election event's ballot box, in the
//! event's electoral-log database on the server `ELECTORAL_LOG_PG_*` names.
//! Administrators' GraphQL access never returns ballot contents.
use super::voting::StoredBallot;
use anyhow::{bail, ensure, Context, Result};
use electoral_log::adapters::ballot_box::BallotStatus;
use electoral_log::adapters::ballot_box_reads::BallotIdMatch;
use electoral_log::adapters::ballot_box_received::ReceivedStatus;
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::adapters::privacy::BallotIdentity;
use std::time::{Duration, Instant};
use windmill::services::ballot_box_reads::ballot_box_time;
use windmill::services::electoral_log_board::get_event_store;

/// How long the read waits for the sequencer to record the accepted ballots.
const RECORDED_WAIT: Duration = Duration::from_secs(60);
const RECORDED_POLL: Duration = Duration::from_secs(1);

/// Wait until every accepted ballot of the event is in its log, so that a ballot
/// is never reported missing from the log while the sequencer is still at work.
pub async fn wait_until_recorded(
    store: &PostgresStore,
    election_event_id: &str,
    timeout: Duration,
) -> Result<()> {
    let started = Instant::now();
    loop {
        let waiting = store.pending_count(election_event_id).await?;
        if waiting == 0 {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            bail!("{waiting} accepted ballots are not in the election event's log yet");
        }
        tokio::time::sleep(RECORDED_POLL).await;
    }
}

/// Identity-based acceptance cannot inspect private ballots or silently report
/// an empty ballot box. Public proof validation is the supported alternative.
async fn require_identity_acceptance(store: &PostgresStore, event: &str) -> Result<()> {
    ensure!(store.ballot_identity(event).await? == BallotIdentity::VoterId,
        "Identity-based acceptance is unsupported for pseudonym events; use the public ballot proof demonstration");
    Ok(())
}

/// The voters' valid ballots, in the order they were cast.
pub async fn valid_ballots(
    store: &PostgresStore,
    election_event_id: &str,
    voter_ids: &[String],
) -> Result<Vec<StoredBallot>> {
    require_identity_acceptance(store, election_event_id).await?;
    let mut stored = Vec::new();
    for voter_id in voter_ids {
        for ballot in store.voter_ballots(election_event_id, voter_id).await? {
            if ballot.status != BallotStatus::Valid {
                continue;
            }
            let content = store
                .voter_ballot_contents(
                    election_event_id,
                    voter_id,
                    &ballot.election_id,
                    BallotIdMatch::Exact(&ballot.ballot_id),
                )
                .await?
                .into_iter()
                .find(|found| found.ballot.id == ballot.id)
                .map(|found| found.content)
                .with_context(|| format!("Ballot {} has no content", ballot.ballot_id))?;
            let receipt_ballot_id = store
                .received_ballot(
                    election_event_id,
                    &ballot.election_id,
                    voter_id,
                    &ballot.ballot_id,
                )
                .await?
                .filter(|received| received.status == ReceivedStatus::Cast)
                .map(|received| received.ballot_id);
            stored.push(StoredBallot {
                voter_id: voter_id.clone(),
                receipt_ballot_id,
                content,
                cast_at: ballot_box_time(ballot.accepted_at)?,
                ballot_id: ballot.ballot_id,
            });
        }
    }
    stored.sort_by_key(|ballot| ballot.cast_at);
    Ok(stored)
}

/// The voters' valid ballots in the election event's ballot box, once the
/// sequencer has recorded them all.
pub fn stored_ballots(election_event_id: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>> {
    tokio::runtime::Runtime::new()
        .context("Failed to create Tokio runtime")?
        .block_on(async {
            let store = get_event_store(election_event_id)
                .await
                .context("Cannot open the election event's ballot box (ELECTORAL_LOG_PG_*)")?;
            require_identity_acceptance(&store, election_event_id).await?;
            wait_until_recorded(&store, election_event_id, RECORDED_WAIT).await?;
            valid_ballots(&store, election_event_id, voter_ids).await
        })
}

#[cfg(test)]
mod privacy_tests {
    use super::*;
    use electoral_log::adapters::privacy::BallotIdentity;
    use windmill::services::electoral_log_board::get_event_databases;

    #[tokio::test]
    #[ignore = "requires disposable ELECTORAL_LOG_PG_* and ENV_SLUG"]
    async fn identity_based_acceptance_refuses_privacy_events_before_returning_empty_ballots() {
        let databases = get_event_databases().await.unwrap();
        let tenant = uuid::Uuid::new_v4().to_string();
        let event = uuid::Uuid::new_v4().to_string();
        let store = databases.create_event(&tenant, &event).await.unwrap();
        store.create_ballot_box(&event).await.unwrap();
        assert!(valid_ballots(&store, &event, &["synthetic-voter".into()])
            .await
            .unwrap()
            .is_empty());
        store
            .set_ballot_identity(&event, BallotIdentity::Pseudonym)
            .await
            .unwrap();
        let result = valid_ballots(&store, &event, &["synthetic-voter".into()]).await;
        databases.drop_event(&tenant, &event).await.unwrap();
        let error = result.unwrap_err();
        assert!(error.to_string().contains("public ballot proof"));
    }
}
