// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box against a real server, in the database of
//! `ELECTORAL_LOG_TEST_DATABASE_URL`. Each test uses events of its own and drops
//! their partitions at the end.

use anyhow::Result;
use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus};
use electoral_log::adapters::postgres::PostgresStore;
use uuid::Uuid;

async fn store() -> Result<PostgresStore> {
    let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let store = PostgresStore::new(config)?;
    store.initialize().await?;
    Ok(store)
}

async fn drop_ballot_box(store: &PostgresStore, event: &str) -> Result<()> {
    let suffix = event.replace('-', "");
    store
        .client()
        .await?
        .batch_execute(&format!(
            "DELETE FROM ballot_box_pending WHERE election_event_id = '{event}';
             DROP TABLE ballot_box_ballot_{suffix}, ballot_box_voter_{suffix};"
        ))
        .await?;
    Ok(())
}

struct Vote {
    event: String,
    election: String,
    area: String,
    voter: String,
    ballot: String,
    allowed: i32,
}

impl Vote {
    fn new(event: &str, election: &str, area: &str, voter: &str, allowed: i32) -> Self {
        Self {
            event: event.into(),
            election: election.into(),
            area: area.into(),
            voter: voter.into(),
            ballot: Uuid::new_v4().simple().to_string(),
            allowed,
        }
    }

    fn request(&self) -> AcceptBallot<'_> {
        AcceptBallot {
            election_event_id: &self.event,
            election_id: &self.election,
            area_id: &self.area,
            voter_id: &self.voter,
            ballot_id: &self.ballot,
            format: "test",
            content: "ciphertext",
            voter_signature: None,
            pseudonym_hash: &[1; 64],
            ballot_hash: &[2; 64],
            voting_channel: "ONLINE",
            status: BallotStatus::Valid,
            voter_ip: Some("192.0.2.1"),
            voter_country: Some("ES"),
            username: Some("voter"),
            allowed_votes: self.allowed,
        }
    }
}

fn ids() -> (String, String, String, String) {
    (
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    )
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn votes_follow_the_revote_and_area_rules() -> Result<()> {
    let store = store().await?;
    let (event, election, area, other_area) = ids();
    store.create_ballot_box(&event).await?;
    store.create_ballot_box(&event).await?;
    assert!(store.has_ballot_box(&event).await?);

    let first = Vote::new(&event, &election, &area, "voter-a", 2);
    assert!(matches!(
        store.accept_ballot(&first.request()).await?,
        AcceptOutcome::Accepted { .. }
    ));
    let again = Vote::new(&event, &election, &area, "voter-a", 2);
    assert!(matches!(
        store.accept_ballot(&again.request()).await?,
        AcceptOutcome::Accepted { .. }
    ));
    let third = Vote::new(&event, &election, &area, "voter-a", 2);
    assert_eq!(
        store.accept_ballot(&third.request()).await?,
        AcceptOutcome::TooManyVotes
    );
    let elsewhere = Vote::new(&event, &election, &other_area, "voter-a", 0);
    assert_eq!(
        store.accept_ballot(&elsewhere.request()).await?,
        AcceptOutcome::VotedInOtherArea
    );

    let state = store
        .voter_state(&event, &election, "voter-a")
        .await?
        .unwrap();
    assert_eq!(state.votes, 2);
    assert_eq!(state.last_ballot_id, again.ballot);

    // A reused ballot ID changes nothing, not even the voter's count.
    let mut reused = Vote::new(&event, &election, &area, "voter-b", 0);
    reused.ballot = first.ballot.clone();
    assert_eq!(
        store.accept_ballot(&reused.request()).await?,
        AcceptOutcome::DuplicateBallotId
    );
    assert!(store
        .voter_state(&event, &election, "voter-b")
        .await?
        .is_none());

    // 0 allows any number of votes.
    for _ in 0..5 {
        let vote = Vote::new(&event, &election, &area, "voter-c", 0);
        assert!(matches!(
            store.accept_ballot(&vote.request()).await?,
            AcceptOutcome::Accepted { .. }
        ));
    }
    assert_eq!(
        store
            .voter_state(&event, &election, "voter-c")
            .await?
            .unwrap()
            .votes,
        5
    );

    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn concurrent_votes_of_one_voter_respect_the_limit() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let mut tasks = Vec::new();
    for _ in 0..24 {
        let store = store.clone();
        let vote = Vote::new(&event, &election, &area, "voter", 3);
        tasks.push(tokio::spawn(async move {
            store.accept_ballot(&vote.request()).await
        }));
    }
    let mut accepted = 0;
    for task in tasks {
        if matches!(task.await??, AcceptOutcome::Accepted { .. }) {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 3);
    assert_eq!(
        store
            .voter_state(&event, &election, "voter")
            .await?
            .unwrap()
            .votes,
        3
    );
    assert_eq!(store.pending_count(&event).await?, 3);
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn accepted_ballots_wait_for_the_sequencer_in_order() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let mut seqs = Vec::new();
    for voter in ["v1", "v2", "v3", "v4"] {
        let vote = Vote::new(&event, &election, &area, voter, 1);
        match store.accept_ballot(&vote.request()).await? {
            AcceptOutcome::Accepted { seq, .. } => seqs.push(seq),
            other => anyhow::bail!("unexpected {other:?}"),
        }
    }
    let pending = store.pending_ballots(&event, 3).await?;
    assert_eq!(
        pending.iter().map(|ballot| ballot.seq).collect::<Vec<_>>(),
        seqs[..3]
    );
    assert_eq!(pending[0].voter_id, "v1");
    assert_eq!(pending[0].election_id, election);
    assert_eq!(pending[0].ballot_hash, vec![2; 64]);
    assert!(store.events_with_pending_ballots().await?.contains(&event));
    assert_eq!(store.remove_pending(&event, &seqs[..3]).await?, 3);
    let rest = store.pending_ballots(&event, 10).await?;
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].seq, seqs[3]);
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn one_sequencer_at_a_time_holds_an_event() -> Result<()> {
    let store = store().await?;
    let event = Uuid::new_v4().to_string();
    assert!(store.take_sequencer_lease(&event, "a", 60).await?);
    assert!(store.take_sequencer_lease(&event, "a", 60).await?);
    assert!(!store.take_sequencer_lease(&event, "b", 60).await?);
    store.release_sequencer_lease(&event, "b").await?;
    assert!(!store.take_sequencer_lease(&event, "b", 60).await?);
    store.release_sequencer_lease(&event, "a").await?;
    assert!(store.take_sequencer_lease(&event, "b", 60).await?);
    store
        .client()
        .await?
        .execute(
            "DELETE FROM ballot_box_sequencer WHERE election_event_id = $1::text::uuid",
            &[&event],
        )
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn events_without_a_ballot_box_refuse_votes() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    assert!(!store.has_ballot_box(&event).await?);
    let vote = Vote::new(&event, &election, &area, "voter", 1);
    assert!(store.accept_ballot(&vote.request()).await.is_err());
    Ok(())
}
