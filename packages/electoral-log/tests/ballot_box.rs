// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box against a real server, in the database of
//! `ELECTORAL_LOG_TEST_DATABASE_URL`. Each test uses events of its own and drops
//! their partitions at the end.

use anyhow::Result;
use chrono::{DateTime, Utc};
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, AcceptReceived, BallotStatus,
};
use electoral_log::adapters::ballot_box_reads::{
    tally_ballots_query, BallotIdMatch, BucketRange, FirstValidVote, FirstValidVotes,
    IpBallotsFilter, Participation, SEQUENCE_SETTLED_SECS,
};
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedSignature, ReceivedStatus,
};
use electoral_log::adapters::ballot_box_seal::{
    box_ballots, insert_seal, is_ballot_box_sealed, is_sealed_refusal, try_lock_ballot_box,
    BallotBoxId, BoxBallot, NewSeal, SealOutcome,
};
use electoral_log::adapters::ballot_box_status::{BallotToReview, VoterBallotState};
use electoral_log::adapters::postgres::{append_in, PostgresStore};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{ElectoralLogMessage, LogEntry, StoredRecord};
use std::time::Duration;
use uuid::Uuid;

/// Applying the schema takes locks that deadlock with the appends of tests running
/// at the same time, so each test binary applies it once.
static SCHEMA: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn initialized(store: &PostgresStore) -> Result<()> {
    SCHEMA.get_or_try_init(|| store.initialize()).await?;
    Ok(())
}

async fn store() -> Result<PostgresStore> {
    let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let store = PostgresStore::new(config)?;
    initialized(&store).await?;
    Ok(store)
}

async fn drop_ballot_box(store: &PostgresStore, event: &str) -> Result<()> {
    store.drop_ballot_box(event).await?;
    assert!(!store.has_ballot_box(event).await?);
    Ok(())
}

struct Vote {
    event: String,
    election: String,
    area: String,
    voter: String,
    ballot: String,
    allowed: i32,
    channel: &'static str,
    ip: Option<&'static str>,
    country: Option<&'static str>,
    status: BallotStatus,
    content: &'static str,
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
            channel: "ONLINE",
            ip: Some("192.0.2.1"),
            country: Some("ES"),
            status: BallotStatus::Valid,
            content: "ciphertext",
        }
    }

    fn with_content(mut self, content: &'static str) -> Self {
        self.content = content;
        self
    }

    fn sent_from(
        mut self,
        channel: &'static str,
        ip: Option<&'static str>,
        country: Option<&'static str>,
    ) -> Self {
        self.channel = channel;
        self.ip = ip;
        self.country = country;
        self
    }

    fn with_status(mut self, status: BallotStatus) -> Self {
        self.status = status;
        self
    }

    async fn accept(&self, store: &PostgresStore) -> Result<i64> {
        Ok(self.accept_with_id(store).await?.0)
    }

    async fn accept_with_id(&self, store: &PostgresStore) -> Result<(i64, String)> {
        match store.accept_ballot(&self.request()).await? {
            AcceptOutcome::Accepted { seq, id } => Ok((seq, id)),
            other => anyhow::bail!("unexpected {other:?}"),
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
            content: self.content,
            voter_signature: None,
            pseudonym_hash: &[1; 64],
            ballot_hash: &[2; 64],
            voting_channel: self.channel,
            status: self.status,
            voter_ip: self.ip,
            voter_country: self.country,
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
async fn pending_ballots_become_valid_or_rejected_and_rejections_give_votes_back() -> Result<()> {
    let store = store().await?;
    let (event, election, area, other_area) = ids();
    store.create_ballot_box(&event).await?;

    let pending =
        Vote::new(&event, &election, &area, "voter-a", 1).with_status(BallotStatus::Pending);
    let (_, pending_id) = pending.accept_with_id(&store).await?;
    let found = store.pending_ballot(&event, &pending_id).await?.unwrap();
    assert_eq!(found.voter_id, "voter-a");
    assert_eq!(found.status, BallotStatus::Pending);
    assert_eq!(
        store.voter_ballot_state(&event, "voter-a").await?,
        VoterBallotState {
            has_pending: true,
            has_valid: false
        }
    );
    // A pending ballot counts toward the limit.
    let again = Vote::new(&event, &election, &area, "voter-a", 1);
    assert_eq!(
        store.accept_ballot(&again.request()).await?,
        AcceptOutcome::TooManyVotes
    );

    // Only the expected status changes, and only once.
    assert!(
        !store
            .set_ballot_status(
                &event,
                "voter-a",
                &pending_id,
                BallotStatus::Valid,
                BallotStatus::Rejected
            )
            .await?
    );
    assert!(
        store
            .set_ballot_status(
                &event,
                "voter-a",
                &pending_id,
                BallotStatus::Pending,
                BallotStatus::Valid
            )
            .await?
    );
    assert!(
        !store
            .set_ballot_status(
                &event,
                "voter-a",
                &pending_id,
                BallotStatus::Pending,
                BallotStatus::Rejected
            )
            .await?
    );
    assert!(store.pending_ballot(&event, &pending_id).await?.is_none());
    assert_eq!(
        store.voter_ballot_state(&event, "voter-a").await?,
        VoterBallotState {
            has_pending: false,
            has_valid: true
        }
    );
    assert!(store
        .set_ballot_status(
            &event,
            "voter-a",
            &pending_id,
            BallotStatus::Rejected,
            BallotStatus::Valid
        )
        .await
        .is_err());

    // A rejected ballot no longer counts: the voter votes again, even in another area.
    let rejected =
        Vote::new(&event, &election, &area, "voter-b", 1).with_status(BallotStatus::Pending);
    let (_, rejected_id) = rejected.accept_with_id(&store).await?;
    assert!(
        store
            .set_ballot_status(
                &event,
                "voter-b",
                &rejected_id,
                BallotStatus::Pending,
                BallotStatus::Rejected
            )
            .await?
    );
    assert_eq!(
        store
            .voter_state(&event, &election, "voter-b")
            .await?
            .unwrap()
            .votes,
        0
    );
    assert_eq!(
        store.voter_ballot_state(&event, "voter-b").await?,
        VoterBallotState::default()
    );
    let elsewhere = Vote::new(&event, &election, &other_area, "voter-b", 1);
    elsewhere.accept(&store).await?;
    let state = store
        .voter_state(&event, &election, "voter-b")
        .await?
        .unwrap();
    assert_eq!((state.votes, state.area_id), (1, other_area.clone()));

    // Rejecting a voter's ballots covers pending and valid ones, in every election.
    let other_election = Uuid::new_v4().to_string();
    Vote::new(&event, &election, &area, "voter-c", 0)
        .accept(&store)
        .await?;
    Vote::new(&event, &election, &area, "voter-c", 0)
        .with_status(BallotStatus::Pending)
        .accept(&store)
        .await?;
    Vote::new(&event, &other_election, &area, "voter-c", 0)
        .accept(&store)
        .await?;
    assert_eq!(store.reject_voter_ballots(&event, "voter-c").await?, 3);
    assert_eq!(store.reject_voter_ballots(&event, "voter-c").await?, 0);
    for election in [&election, &other_election] {
        assert_eq!(
            store
                .voter_state(&event, election, "voter-c")
                .await?
                .unwrap()
                .votes,
            0
        );
    }

    let mut states = store.voter_ballot_states(&event).await?;
    states.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        states,
        vec![
            (
                "voter-a".to_string(),
                VoterBallotState {
                    has_pending: false,
                    has_valid: true
                }
            ),
            (
                "voter-b".to_string(),
                VoterBallotState {
                    has_pending: false,
                    has_valid: true
                }
            ),
        ]
    );
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn pending_ballots_are_listed_for_review_in_pages() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let mut ids = Vec::new();
    for voter in ["v1", "v2", "v3"] {
        let vote = Vote::new(&event, &election, &area, voter, 1).with_status(BallotStatus::Pending);
        ids.push(vote.accept_with_id(&store).await?.1);
    }
    Vote::new(&event, &election, &area, "v4", 1)
        .accept(&store)
        .await?;
    ids.sort();

    let mut listed = Vec::new();
    let mut after: Option<BallotToReview> = None;
    loop {
        let page = store.ballots_to_review(after.as_ref(), 0.0, 2).await?;
        let Some(last) = page.last().cloned() else {
            break;
        };
        listed.extend(
            page.into_iter()
                .filter(|ballot| ballot.election_event_id == event)
                .map(|ballot| ballot.id),
        );
        after = Some(last);
    }
    assert_eq!(listed, ids);
    // Ballots younger than the given age wait for the next review.
    let young = store.ballots_to_review(None, 3600.0, 1000).await?;
    assert!(young.iter().all(|ballot| ballot.election_event_id != event));
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

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn reads_cover_voters_administrators_and_reports() -> Result<()> {
    let store = store().await?;
    let (event, first, second, area) = ids();
    store.create_ballot_box(&event).await?;
    let a1 = Vote::new(&event, &first, &area, "voter-a", 0);
    let a2 = Vote::new(&event, &first, &area, "voter-a", 0).sent_from(
        "KIOSK",
        Some("192.0.2.1"),
        Some("ES"),
    );
    let b = Vote::new(&event, &first, &area, "voter-b", 0).sent_from(
        "TELEPHONE",
        Some("198.51.100.7"),
        Some("FR"),
    );
    let c = Vote::new(&event, &second, &area, "voter-c", 0).sent_from("ONLINE", None, None);
    let d = Vote::new(&event, &first, &area, "voter-d", 0).with_status(BallotStatus::Pending);
    let e = Vote::new(&event, &first, &area, "voter-e", 0).with_status(BallotStatus::Rejected);
    for vote in [&a1, &a2, &b, &c, &d, &e] {
        vote.accept(&store).await?;
    }

    let ballots = store.voter_ballots(&event, "voter-a").await?;
    assert_eq!(
        ballots
            .iter()
            .map(|ballot| ballot.ballot_id.as_str())
            .collect::<Vec<_>>(),
        [a1.ballot.as_str(), a2.ballot.as_str()]
    );
    assert_eq!(ballots[1].voting_channel, "KIOSK");
    assert_eq!(ballots[1].status, BallotStatus::Valid);
    assert_eq!(ballots[1].election_id, first);
    assert!(ballots[0].accepted_at > 0 && ballots[0].accepted_at <= ballots[1].accepted_at);
    assert!(store.voter_ballots(&event, "nobody").await?.is_empty());

    let found = store
        .voter_ballot_contents(&event, "voter-a", &first, BallotIdMatch::Exact(&a1.ballot))
        .await?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].content, "ciphertext");
    assert_eq!(found[0].ballot.area_id, area);
    assert_eq!(found[0].ballot.ballot_id, a1.ballot);
    let prefixed = store
        .voter_ballot_contents(
            &event,
            "voter-a",
            &first,
            BallotIdMatch::Prefix(&a2.ballot[..4]),
        )
        .await?;
    assert!(prefixed
        .iter()
        .any(|found| found.ballot.ballot_id == a2.ballot));
    for (voter, election, ballot_id) in [
        ("voter-a", &first, BallotIdMatch::Exact(&b.ballot)),
        ("voter-a", &first, BallotIdMatch::Exact(&a1.ballot[..4])),
        ("voter-a", &second, BallotIdMatch::Exact(&a1.ballot)),
        ("voter-b", &first, BallotIdMatch::Prefix(&a1.ballot)),
    ] {
        assert!(store
            .voter_ballot_contents(&event, voter, election, ballot_id)
            .await?
            .is_empty());
    }

    // The latest valid ballot of each voter: pending and rejected ones do not count.
    assert_eq!(
        store.voters_by_channel(&event, None).await?,
        [
            ("KIOSK".to_string(), 1),
            ("ONLINE".to_string(), 1),
            ("TELEPHONE".to_string(), 1)
        ]
    );
    assert_eq!(
        store
            .voters_by_channel(&event, Some(std::slice::from_ref(&second)))
            .await?,
        [("ONLINE".to_string(), 1)]
    );
    assert_eq!(
        store
            .voters_by_channel(&event, Some(&[first.clone(), second.clone()]))
            .await?,
        store.voters_by_channel(&event, None).await?
    );
    assert!(store.voters_by_channel(&event, Some(&[])).await?.is_empty());

    let today: String = store
        .client()
        .await?
        .query_one(
            "SELECT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD')",
            &[],
        )
        .await?
        .get(0);
    let start = format!("{today} 00:00:00");
    let end = format!("{today} 23:59:59.999999");
    let day = BucketRange {
        resolution: "day",
        time_zone: "UTC",
        start: &start,
        end: &end,
        last_buckets: None,
    };
    let buckets = store
        .ballots_per_bucket(&event, None, &day, "ONLINE")
        .await?;
    assert!(buckets
        .iter()
        .all(|bucket| bucket.day == today && bucket.bucket == format!("{today}T00:00:00")));
    assert_eq!(
        buckets
            .iter()
            .map(|bucket| (bucket.channel.as_str(), bucket.count))
            .collect::<Vec<_>>(),
        [("KIOSK", 1), ("ONLINE", 2), ("TELEPHONE", 1)]
    );
    let three_days = BucketRange {
        last_buckets: Some(3),
        ..day.clone()
    };
    let buckets = store
        .ballots_per_bucket(
            &event,
            Some(std::slice::from_ref(&second)),
            &three_days,
            "ONLINE",
        )
        .await?;
    assert_eq!(buckets.len(), 3);
    assert_eq!(
        buckets
            .iter()
            .map(|bucket| bucket.count)
            .collect::<Vec<_>>(),
        [0, 0, 1]
    );

    let by_ip = store
        .ballots_by_ip(
            &event,
            &IpBallotsFilter {
                limit: 10,
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(by_ip.len(), 2);
    assert_eq!(
        (
            by_ip[0].ip.as_str(),
            by_ip[0].country.as_str(),
            by_ip[0].count
        ),
        ("192.0.2.1", "ES", 2)
    );
    assert_eq!(by_ip[0].voter_ids, ["voter-a", "voter-a"]);
    assert_eq!(by_ip[1].voter_ids, ["voter-b"]);
    let filtered = store
        .ballots_by_ip(
            &event,
            &IpBallotsFilter {
                ip_pattern: Some("%198.51%"),
                limit: 10,
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].election_id, first);
    let second_page = store
        .ballots_by_ip(
            &event,
            &IpBallotsFilter {
                limit: 1,
                offset: 1,
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(second_page, by_ip[1..]);
    let second_election = store
        .ballots_by_ip(
            &event,
            &IpBallotsFilter {
                elections: Some(std::slice::from_ref(&second)),
                limit: 10,
                ..Default::default()
            },
        )
        .await?;
    assert!(second_election.is_empty());
    let first_election = store
        .ballots_by_ip(
            &event,
            &IpBallotsFilter {
                elections: Some(std::slice::from_ref(&first)),
                limit: 10,
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(first_election, by_ip);

    let voters: Vec<String> = ["voter-a", "voter-d", "voter-e", "nobody"]
        .map(String::from)
        .to_vec();
    let mut votes = store.votes_of_voters(&event, &voters, None).await?;
    votes.sort_by(|x, y| x.voter_id.cmp(&y.voter_id));
    assert_eq!(
        votes
            .iter()
            .map(|votes| (
                votes.voter_id.as_str(),
                votes.election_id.as_str(),
                votes.votes
            ))
            .collect::<Vec<_>>(),
        [
            ("voter-a", first.as_str(), 2),
            ("voter-d", first.as_str(), 1)
        ]
    );
    assert!(store
        .votes_of_voters(&event, &voters, Some(second.as_str()))
        .await?
        .is_empty());

    assert_eq!(
        store.participation(&event, None, None).await?,
        Participation {
            ballots: 4,
            voters: 3
        }
    );
    assert_eq!(
        store
            .participation(&event, Some(first.as_str()), None)
            .await?,
        Participation {
            ballots: 3,
            voters: 2
        }
    );
    assert_eq!(
        store
            .participation(&event, Some(first.as_str()), Some(area.as_str()))
            .await?,
        Participation {
            ballots: 3,
            voters: 2
        }
    );
    let (_, _, _, no_ballots_area) = ids();
    assert_eq!(
        store
            .participation(&event, Some(first.as_str()), Some(no_ballots_area.as_str()))
            .await?,
        Participation {
            ballots: 0,
            voters: 0
        }
    );
    drop_ballot_box(&store, &event).await
}

fn cast_vote_record(event: &str, seq: i64) -> LogEntry {
    LogEntry {
        delivery_id: format!("ballot-box:{event}:{seq}"),
        message: ElectoralLogMessage {
            id: 0,
            created: 1,
            sender_pk: "sender".into(),
            statement_timestamp: 1,
            statement_kind: "CastVote".into(),
            message: seq.to_be_bytes().to_vec(),
            version: "2".into(),
            user_id: None,
            username: None,
            election_id: None,
            area_id: None,
            ballot_id: None,
        },
    }
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn the_tally_reads_each_voters_latest_valid_recorded_ballot() -> Result<()> {
    let store = store().await?;
    let (event, election, area, other_area) = ids();
    store.create_ballot_box(&event).await?;
    let board = format!("tallytest{}", Uuid::new_v4().simple());
    store.create_board(&board).await?;

    let mut sequenced = Vec::new();
    for vote in [
        Vote::new(&event, &election, &area, "v1", 0).with_content("first"),
        Vote::new(&event, &election, &area, "v1", 0)
            .with_content("second")
            .sent_from("KIOSK", None, None),
        Vote::new(&event, &election, &area, "v2", 0).with_content("valid"),
        Vote::new(&event, &election, &area, "v2", 0)
            .with_content("rejected")
            .with_status(BallotStatus::Rejected),
        Vote::new(&event, &election, &area, "v3", 0).with_status(BallotStatus::Pending),
        Vote::new(&event, &election, &other_area, "v4", 0),
    ] {
        sequenced.push(vote.accept(&store).await?);
    }
    let waiting = Vote::new(&event, &election, &area, "v5", 0)
        .accept(&store)
        .await?;
    store
        .append(
            &board,
            &mut sequenced
                .iter()
                .map(|seq| anyhow::Ok(cast_vote_record(&event, *seq))),
        )
        .await?;
    store.remove_pending(&event, &sequenced).await?;

    assert_eq!(store.unsequenced_count(&event, &election, &area).await?, 1);
    assert_eq!(
        store
            .unsequenced_count(&event, &election, &other_area)
            .await?,
        0
    );
    assert_eq!(
        store.pending_status_count(&event, &election, &area).await?,
        1
    );
    assert_eq!(
        store
            .unrecorded_count(&board, &event, &election, &area)
            .await?,
        0
    );

    let query = tally_ballots_query(&event, &election, &area)?;
    let rows: Vec<(String, String, String)> = store
        .client()
        .await?
        .query(&query, &[])
        .await?
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect();
    assert_eq!(
        rows,
        [
            ("v1".to_string(), "second".to_string(), "KIOSK".to_string()),
            ("v2".to_string(), "valid".to_string(), "ONLINE".to_string()),
        ]
    );
    assert!(tally_ballots_query(&event, &election, "x' OR '1'='1").is_err());

    // A sequenced ballot whose record is gone is reported.
    store
        .client()
        .await?
        .execute(
            "DELETE FROM electoral_log_messages WHERE board_name = $1 AND delivery_id = $2",
            &[&board, &format!("ballot-box:{event}:{}", sequenced[1])],
        )
        .await?;
    assert_eq!(
        store
            .unrecorded_count(&board, &event, &election, &area)
            .await?,
        1
    );

    store.remove_pending(&event, &[waiting]).await?;
    store.delete_board(&board).await?;
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn ballot_boxes_come_and_go_without_holding_back_other_events_votes() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    // A vote of the event in progress: its open transaction holds a lock on the
    // partitioned tables until it ends.
    let in_progress = store.client().await?;
    in_progress
        .batch_execute(&format!(
            "BEGIN;
             INSERT INTO ballot_box_voter (election_event_id, election_id, voter_id, area_id,
                 votes, last_ballot_id, updated_at)
             VALUES ('{event}', '{election}', 'in-progress', '{area}', 1, 'ballot', now());"
        ))
        .await?;

    let other = Uuid::new_v4().to_string();
    tokio::time::timeout(Duration::from_secs(5), store.create_ballot_box(&other)).await??;
    assert!(store.has_ballot_box(&other).await?);
    store.create_ballot_box(&other).await?;

    // Dropping waits for the transaction in progress, and votes go on meanwhile.
    let dropping = tokio::spawn({
        let store = store.clone();
        let other = other.clone();
        async move { store.drop_ballot_box(&other).await }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!dropping.is_finished());
    let vote = Vote::new(&event, &election, &area, "voter", 0);
    let accepted =
        tokio::time::timeout(Duration::from_secs(5), store.accept_ballot(&vote.request()))
            .await??;
    assert!(matches!(accepted, AcceptOutcome::Accepted { .. }));
    in_progress.batch_execute("ROLLBACK").await?;
    tokio::time::timeout(Duration::from_secs(10), dropping).await???;
    assert!(!store.has_ballot_box(&other).await?);
    store.drop_ballot_box(&other).await?;

    let ballots = store.voter_ballots(&event, "voter").await?;
    assert_eq!(ballots.len(), 1);
    drop_ballot_box(&store, &event).await
}

fn ballot_box<'a>(event: &'a str, election: &'a str, area: &'a str) -> BallotBoxId<'a> {
    BallotBoxId {
        election_event_id: event,
        election_id: election,
        area_id: area,
    }
}

fn seal_record(ballot_box: BallotBoxId<'_>, kind: &str) -> LogEntry {
    LogEntry {
        delivery_id: format!(
            "{kind}:{}:{}:{}",
            ballot_box.election_event_id, ballot_box.election_id, ballot_box.area_id
        ),
        message: ElectoralLogMessage {
            id: 0,
            created: 1,
            sender_pk: "sender".into(),
            statement_timestamp: 1,
            statement_kind: kind.into(),
            message: kind.as_bytes().to_vec(),
            version: "1".into(),
            user_id: None,
            username: None,
            election_id: Some(ballot_box.election_id.into()),
            area_id: Some(ballot_box.area_id.into()),
            ballot_id: None,
        },
    }
}

fn sealed(ballots: &[BoxBallot]) -> SealOutcome {
    SealOutcome::Sealed {
        seal_hash: vec![7; 64],
        manifest: vec![1, 2, 3],
        ballots_in_box: i64::try_from(ballots.len()).unwrap(),
        ballots_counted: i64::try_from(
            ballots
                .iter()
                .filter(|ballot| ballot.status == BallotStatus::Valid)
                .count(),
        )
        .unwrap(),
    }
}

/// Seal a box as the sealer does, in one transaction: lock it, read it, append the
/// seal's record and store the seal. `failed` stores a failed seal instead.
async fn seal_box(
    store: &PostgresStore,
    board: &str,
    ballot_box: BallotBoxId<'_>,
    failed: bool,
) -> Result<(StoredRecord, Vec<BoxBallot>)> {
    let mut client = store.client().await?;
    let tx = client.transaction().await?;
    assert!(try_lock_ballot_box(&tx, ballot_box).await?);
    let mut ballots = Vec::new();
    box_ballots(&tx, ballot_box, |ballot| {
        ballots.push(ballot);
        Ok(())
    })
    .await?;
    let (kind, outcome) = if failed {
        (
            "BallotBoxSealFailed",
            SealOutcome::Failed {
                reason: "unreadable ballot".into(),
            },
        )
    } else {
        ("BallotBoxSealed", sealed(&ballots))
    };
    let record = seal_record(ballot_box, kind);
    let mut stored = append_in(&tx, board, std::slice::from_ref(&record)).await?;
    assert_eq!(stored.len(), 1);
    assert!(
        insert_seal(
            &tx,
            &NewSeal {
                ballot_box,
                outcome: &outcome,
                signed_message: &record.message.message,
                record: &stored[0],
            },
        )
        .await?
    );
    tx.commit().await?;
    Ok((stored.remove(0), ballots))
}

fn sealed_error_message(error: &tokio_postgres::Error) -> Option<&str> {
    error.as_db_error().map(|error| error.message())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn a_sealed_box_refuses_every_write_and_other_boxes_take_them() -> Result<()> {
    let store = store().await?;
    let (event, election, area, other_area) = ids();
    store.create_ballot_box(&event).await?;
    let board = format!("sealtest{}", Uuid::new_v4().simple());
    store.create_board(&board).await?;
    let (_, sealed_id) = Vote::new(&event, &election, &area, "voter-a", 0)
        .with_content("sealed")
        .accept_with_id(&store)
        .await?;
    let (_, open_id) = Vote::new(&event, &election, &other_area, "voter-b", 0)
        .accept_with_id(&store)
        .await?;
    let sealed_box = ballot_box(&event, &election, &area);
    let (record, read) = seal_box(&store, &board, sealed_box, false).await?;
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].voter_id, "voter-a");
    assert_eq!(read[0].content, "sealed");

    // New ballots: only the sealed box refuses them, and the refusal stores nothing.
    let refused = Vote::new(&event, &election, &area, "voter-c", 0);
    assert_eq!(
        store.accept_ballot(&refused.request()).await?,
        AcceptOutcome::BallotBoxSealed
    );
    assert!(store
        .voter_state(&event, &election, "voter-c")
        .await?
        .is_none());
    let open = Vote::new(&event, &election, &other_area, "voter-d", 0);
    assert!(matches!(
        store.accept_ballot(&open.request()).await?,
        AcceptOutcome::Accepted { .. }
    ));

    // Status changes.
    let error = store
        .set_ballot_status(
            &event,
            "voter-a",
            &sealed_id,
            BallotStatus::Valid,
            BallotStatus::Rejected,
        )
        .await
        .unwrap_err();
    assert!(is_sealed_refusal(&error), "{error:#}");
    let error = store
        .reject_voter_ballots(&event, "voter-a")
        .await
        .unwrap_err();
    assert!(is_sealed_refusal(&error), "{error:#}");
    assert_eq!(
        store.voter_ballots(&event, "voter-a").await?[0].status,
        BallotStatus::Valid
    );
    assert!(
        store
            .set_ballot_status(
                &event,
                "voter-b",
                &open_id,
                BallotStatus::Valid,
                BallotStatus::Rejected
            )
            .await?
    );

    // Any other write of the box's ballots, including one that moves a ballot in or
    // out of it, and truncating the event's ballots.
    let client = store.client().await?;
    let partition = format!("ballot_box_ballot_{}", event.replace('-', ""));
    for statement in [
        format!("DELETE FROM ballot_box_ballot WHERE election_event_id = '{event}' AND id = '{sealed_id}'"),
        format!("UPDATE ballot_box_ballot SET area_id = '{area}' WHERE election_event_id = '{event}' AND id = '{open_id}'"),
        format!("UPDATE ballot_box_ballot SET area_id = '{other_area}' WHERE election_event_id = '{event}' AND id = '{sealed_id}'"),
        format!("TRUNCATE {partition}"),
    ] {
        let error = client.batch_execute(&statement).await.unwrap_err();
        assert!(is_ballot_box_sealed(&error), "{statement}: {error:?}");
    }
    let parent_guard: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_trigger WHERE tgrelid = 'ballot_box_ballot'::regclass \
             AND tgname IN ('ballot_box_seal_guard', 'ballot_box_seal_guard_truncate')",
            &[],
        )
        .await?
        .get(0);
    assert_eq!(parent_guard, 2);

    // Above READ COMMITTED a write could miss a seal committed while it waited.
    let mut connection = store.client().await?;
    let tx = connection
        .build_transaction()
        .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
        .start()
        .await?;
    let error = tx
        .execute(
            "UPDATE ballot_box_ballot SET status = status \
             WHERE election_event_id = $1::text::uuid AND id = $2::text::uuid",
            &[&event, &open_id],
        )
        .await
        .unwrap_err();
    assert_eq!(
        sealed_error_message(&error),
        Some("ballot_box_seal_requires_read_committed")
    );
    tx.rollback().await?;

    // Seals are permanent.
    for statement in [
        format!(
            "UPDATE ballot_box_seal SET failure_reason = 'x' WHERE election_event_id = '{event}'"
        ),
        format!("DELETE FROM ballot_box_seal WHERE election_event_id = '{event}'"),
        "TRUNCATE ballot_box_seal".to_string(),
    ] {
        let error = client.batch_execute(&statement).await.unwrap_err();
        assert_eq!(
            sealed_error_message(&error),
            Some("ballot_box_seal_permanent"),
            "{statement}"
        );
    }

    // Reads of the seals.
    let seal = store.ballot_box_seal(sealed_box).await?.unwrap();
    assert_eq!(seal.election_id, election);
    assert_eq!(seal.area_id, area);
    assert_eq!(seal.outcome, sealed(&read));
    assert_eq!(seal.signed_message, b"BallotBoxSealed".to_vec());
    assert_eq!(seal.log_delivery_id, record.delivery_id);
    assert_eq!(seal.log_uid, record.log_uid);
    assert_eq!(seal.log_leaf_index, record.leaf_index);
    assert!(store.has_ballot_box_seals(&event).await?);
    assert!(!store.has_ballot_box_seals(&election).await?);
    assert_eq!(store.ballot_box_seals(&event).await?, vec![seal.clone()]);
    assert_eq!(
        store.voter_sealed_boxes(&event, "voter-a").await?,
        vec![(election.clone(), area.clone())]
    );
    assert!(store
        .voter_sealed_boxes(&event, "voter-b")
        .await?
        .is_empty());
    let mut areas = vec![area.clone(), other_area.clone()];
    areas.sort();
    assert_eq!(store.ballot_box_areas(&event, &election).await?, areas);

    // The tally reads the seal, its record and the box's ballots in one snapshot.
    let mut ballots = Vec::new();
    let read_box = store
        .read_sealed_box(&board, sealed_box, |ballot| {
            ballots.push(ballot);
            Ok(())
        })
        .await?;
    assert_eq!(read_box.seal, Some(seal));
    assert_eq!(read_box.seal_records.len(), 1);
    assert_eq!(read_box.seal_records[0].id, record.id);
    assert_eq!(ballots, read);
    let mut unsealed = 0;
    let open_box = store
        .read_sealed_box(&board, ballot_box(&event, &election, &other_area), |_| {
            unsealed += 1;
            Ok(())
        })
        .await?;
    assert!(open_box.seal.is_none() && open_box.seal_records.is_empty());
    assert_eq!(unsealed, 0);

    // A failed seal locks its box too.
    seal_box(
        &store,
        &board,
        ballot_box(&event, &election, &other_area),
        true,
    )
    .await?;
    let late = Vote::new(&event, &election, &other_area, "voter-e", 0);
    assert_eq!(
        store.accept_ballot(&late.request()).await?,
        AcceptOutcome::BallotBoxSealed
    );
    let failed = store
        .ballot_box_seal(ballot_box(&event, &election, &other_area))
        .await?
        .unwrap();
    assert_eq!(
        failed.outcome,
        SealOutcome::Failed {
            reason: "unreadable ballot".into()
        }
    );

    // A sealed ballot box is kept until its event's database is dropped.
    assert!(store.drop_ballot_box(&event).await.is_err());
    assert!(store.has_ballot_box(&event).await?);
    store.delete_board(&board).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn writes_wait_for_a_seal_in_progress_and_a_seal_waits_for_no_write() -> Result<()> {
    let store = store().await?;
    let (event, election, area, other_area) = ids();
    store.create_ballot_box(&event).await?;
    let board = format!("sealtest{}", Uuid::new_v4().simple());
    store.create_board(&board).await?;
    let (_, ballot) = Vote::new(&event, &election, &area, "voter-a", 0)
        .accept_with_id(&store)
        .await?;
    let sealed_box = ballot_box(&event, &election, &area);
    let accept_later = |voter: &'static str| {
        let store = store.clone();
        let vote = Vote::new(&event, &election, &area, voter, 0);
        tokio::spawn(async move { store.accept_ballot(&vote.request()).await })
    };

    // A write in progress keeps the sealer out, without making it wait.
    let writer = store.client().await?;
    writer
        .batch_execute(&format!(
            "BEGIN; UPDATE ballot_box_ballot SET status = status \
             WHERE election_event_id = '{event}' AND id = '{ballot}'"
        ))
        .await?;
    let mut sealer = store.client().await?;
    let tx = sealer.transaction().await?;
    assert!(!try_lock_ballot_box(&tx, sealed_box).await?);
    tx.rollback().await?;
    writer.batch_execute("COMMIT").await?;

    // While the sealer holds the box, its writes wait and other boxes' do not.
    let tx = sealer.transaction().await?;
    assert!(try_lock_ballot_box(&tx, sealed_box).await?);
    let waiting = accept_later("voter-b");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!waiting.is_finished());
    let other = Vote::new(&event, &election, &other_area, "voter-c", 0);
    assert!(matches!(
        tokio::time::timeout(
            Duration::from_secs(5),
            store.accept_ballot(&other.request())
        )
        .await??,
        AcceptOutcome::Accepted { .. }
    ));
    // A seal given up lets the write through.
    tx.rollback().await?;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), waiting).await???,
        AcceptOutcome::Accepted { .. }
    ));

    // A write that waited for a seal that commits is refused.
    let tx = sealer.transaction().await?;
    assert!(try_lock_ballot_box(&tx, sealed_box).await?);
    let waiting = accept_later("voter-d");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!waiting.is_finished());
    let record = seal_record(sealed_box, "BallotBoxSealed");
    let stored = append_in(&tx, &board, std::slice::from_ref(&record)).await?;
    let outcome = SealOutcome::Failed {
        reason: "reason".into(),
    };
    assert!(
        insert_seal(
            &tx,
            &NewSeal {
                ballot_box: sealed_box,
                outcome: &outcome,
                signed_message: &record.message.message,
                record: &stored[0],
            },
        )
        .await?
    );
    tx.commit().await?;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), waiting).await???,
        AcceptOutcome::BallotBoxSealed
    );
    assert!(store
        .voter_state(&event, &election, "voter-d")
        .await?
        .is_none());

    // A box is sealed once: a second seal stores nothing.
    let tx = sealer.transaction().await?;
    assert!(try_lock_ballot_box(&tx, sealed_box).await?);
    let again = LogEntry {
        delivery_id: format!("{}-again", record.delivery_id),
        ..record.clone()
    };
    let stored = append_in(&tx, &board, std::slice::from_ref(&again)).await?;
    assert!(
        !insert_seal(
            &tx,
            &NewSeal {
                ballot_box: sealed_box,
                outcome: &outcome,
                signed_message: &again.message.message,
                record: &stored[0],
            },
        )
        .await?
    );
    tx.rollback().await?;
    assert_eq!(
        store
            .ballot_box_seal(sealed_box)
            .await?
            .unwrap()
            .log_delivery_id,
        record.delivery_id
    );
    store.delete_board(&board).await
}

fn receipt(ballot_id: &str) -> ReceivedSignature {
    ReceivedSignature {
        ballot_id: ballot_id.into(),
        key_id: "key".into(),
        received_signature: format!("signed {ballot_id}"),
    }
}

/// A short Ballot ID of its own.
fn short_ballot_id() -> String {
    let hex = Uuid::new_v4().simple().to_string().to_uppercase();
    format!("{}-{}", &hex[..4], &hex[4..8])
}

fn received_at() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_841_396_472_123).unwrap()
}

struct Received {
    event: String,
    election: String,
    area: String,
    voter: &'static str,
    hash: String,
}

impl Received {
    fn new(event: &str, election: &str, area: &str, voter: &'static str) -> Self {
        Self {
            event: event.into(),
            election: election.into(),
            area: area.into(),
            voter,
            hash: Uuid::new_v4().simple().to_string(),
        }
    }

    fn request(&self) -> ReceiveBallot<'_> {
        ReceiveBallot {
            election_event_id: &self.event,
            election_id: &self.election,
            area_id: &self.area,
            voter_id: self.voter,
            ballot_hash: &self.hash,
            content: "received ciphertext",
            voter_signing_pk: "voter key",
            voter_ballot_signature: "voter signature",
        }
    }

    async fn receive(&self, store: &PostgresStore, ballot_id: &str) -> Result<String> {
        match store
            .receive_ballot(&self.request(), received_at(), |_| Ok(receipt(ballot_id)))
            .await?
        {
            ReceiveOutcome::Received(received) => Ok(received.ballot_id),
            other => anyhow::bail!("unexpected {other:?}"),
        }
    }

    /// The vote that casts it, allowing `allowed` votes.
    fn vote(&self, allowed: i32) -> Vote {
        let mut vote = Vote::new(&self.event, &self.election, &self.area, self.voter, allowed)
            .with_content("received ciphertext");
        vote.ballot = self.hash.clone();
        vote
    }
}

fn cast(ballot_id: &str) -> AcceptReceived<'_> {
    AcceptReceived {
        ballot_id,
        cast_at: received_at() + chrono::Duration::seconds(30),
        cast_signature: "cast signature",
        cast_receipt_signature: "receipt signature",
    }
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn received_ballots_are_stored_once_under_a_free_ballot_id() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let first = Received::new(&event, &election, &area, "voter-a");
    let taken = short_ballot_id();
    let ReceiveOutcome::Received(received) = store
        .receive_ballot(&first.request(), received_at(), |_| Ok(receipt(&taken)))
        .await?
    else {
        anyhow::bail!("not received");
    };
    assert_eq!(received.ballot_id, taken);
    assert_eq!(received.status, ReceivedStatus::Received);
    assert_eq!(received.received_at, received_at());
    assert_eq!(received.received_signature, format!("signed {taken}"));
    assert_eq!(received.cast_at, None);

    // The same ballot again: the receipt stored then, without signing another.
    assert_eq!(
        store
            .receive_ballot(&first.request(), received_at(), |_| {
                anyhow::bail!("signed again")
            })
            .await?,
        ReceiveOutcome::AlreadyReceived(received.clone())
    );

    // A taken Ballot ID is signed again a millisecond later.
    let second = Received::new(&event, &election, &area, "voter-b");
    let free = short_ballot_id();
    let mut signed = Vec::new();
    let ReceiveOutcome::Received(other) = store
        .receive_ballot(&second.request(), received_at(), |time| {
            signed.push(time);
            Ok(receipt(if signed.len() == 1 { &taken } else { &free }))
        })
        .await?
    else {
        anyhow::bail!("not received");
    };
    assert_eq!(other.ballot_id, free);
    assert_eq!(
        signed,
        [
            received_at(),
            received_at() + chrono::Duration::milliseconds(1)
        ]
    );
    assert_eq!(other.received_at, signed[1]);

    // Nothing is stored when every attempt's Ballot ID is taken.
    let third = Received::new(&event, &election, &area, "voter-c");
    let mut attempts = 0;
    assert_eq!(
        store
            .receive_ballot(&third.request(), received_at(), |_| {
                attempts += 1;
                Ok(receipt(&taken))
            })
            .await?,
        ReceiveOutcome::NoFreeBallotId
    );
    assert_eq!(attempts, 8);
    assert!(store
        .received_ballot(&event, &election, "voter-c", &third.hash)
        .await?
        .is_none());

    // Reads by ballot hash and by Ballot ID, of the voter's own ballots only.
    assert_eq!(
        store
            .received_ballot(&event, &election, "voter-a", &first.hash)
            .await?,
        Some(received.clone())
    );
    assert_eq!(
        store
            .received_ballot_by_id(&event, &election, "voter-a", &taken)
            .await?,
        Some(received)
    );
    assert!(store
        .received_ballot_by_id(&event, &election, "voter-b", &taken)
        .await?
        .is_none());
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn received_ballots_are_cast_once_and_refusals_keep_them_received() -> Result<()> {
    let store = store().await?;
    let (event, election, area, other_area) = ids();
    store.create_ballot_box(&event).await?;

    let ballot = Received::new(&event, &election, &area, "voter-a");
    let ballot_id = ballot.receive(&store, &short_ballot_id()).await?;
    let vote = ballot.vote(1);
    // Only the voter's received ballot with that Ballot ID and hash is cast.
    assert_eq!(
        store
            .accept_received_ballot(&vote.request(), &cast("NONE-NONE"))
            .await?,
        AcceptOutcome::NotReceivable
    );
    let mut other_hash = ballot.vote(1);
    other_hash.ballot = Uuid::new_v4().simple().to_string();
    assert_eq!(
        store
            .accept_received_ballot(&other_hash.request(), &cast(&ballot_id))
            .await?,
        AcceptOutcome::NotReceivable
    );
    let id = match store
        .accept_received_ballot(&vote.request(), &cast(&ballot_id))
        .await?
    {
        AcceptOutcome::Accepted { id, .. } => id,
        other => anyhow::bail!("unexpected {other:?}"),
    };
    let stored = store
        .received_ballot_by_id(&event, &election, "voter-a", &ballot_id)
        .await?
        .unwrap();
    assert_eq!(stored.status, ReceivedStatus::Cast);
    assert_eq!(stored.cast_at, Some(cast(&ballot_id).cast_at));
    assert_eq!(stored.cast_signature.as_deref(), Some("cast signature"));
    assert_eq!(
        stored.cast_receipt_signature.as_deref(),
        Some("receipt signature")
    );
    let ballots = store.voter_ballots(&event, "voter-a").await?;
    assert_eq!(ballots.len(), 1);
    assert_eq!(ballots[0].id, id);
    assert_eq!(ballots[0].ballot_id, ballot.hash);
    assert_eq!(
        ballots[0].accepted_at,
        cast(&ballot_id).cast_at.timestamp_micros()
    );

    // A second cast stores nothing.
    assert_eq!(
        store
            .accept_received_ballot(&vote.request(), &cast(&ballot_id))
            .await?,
        AcceptOutcome::DuplicateBallotId
    );
    assert_eq!(store.voter_ballots(&event, "voter-a").await?.len(), 1);

    // The locator finds a cast ballot by its receipt's Ballot ID.
    for matching in [
        BallotIdMatch::Exact(&ballot_id),
        BallotIdMatch::Prefix(&ballot_id[..4]),
    ] {
        let found = store
            .received_cast_ballots(&event, "voter-a", &election, matching)
            .await?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].received_ballot_id, ballot_id);
        assert_eq!(found[0].ballot.ballot.id, id);
        assert_eq!(found[0].ballot.content, "received ciphertext");
    }

    // The voter's rules refuse a cast and leave the ballot received.
    let too_many = Received::new(&event, &election, &area, "voter-b");
    Vote::new(&event, &election, &area, "voter-b", 1)
        .accept(&store)
        .await?;
    let elsewhere = Received::new(&event, &election, &area, "voter-c");
    Vote::new(&event, &election, &other_area, "voter-c", 0)
        .accept(&store)
        .await?;
    for (received, expected) in [
        (&too_many, AcceptOutcome::TooManyVotes),
        (&elsewhere, AcceptOutcome::VotedInOtherArea),
    ] {
        let ballot_id = received.receive(&store, &short_ballot_id()).await?;
        assert_eq!(
            store
                .accept_received_ballot(&received.vote(1).request(), &cast(&ballot_id))
                .await?,
            expected
        );
        let kept = store
            .received_ballot_by_id(&event, &election, received.voter, &ballot_id)
            .await?
            .unwrap();
        assert_eq!(kept.status, ReceivedStatus::Received);
        assert_eq!(kept.cast_at, None);
        assert!(store
            .received_cast_ballots(
                &event,
                received.voter,
                &election,
                BallotIdMatch::Exact(&ballot_id)
            )
            .await?
            .is_empty());
    }
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn concurrent_casts_of_a_received_ballot_store_one() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let ballot = Received::new(&event, &election, &area, "voter-a");
    let ballot_id = ballot.receive(&store, &short_ballot_id()).await?;

    // Both casts wait for the received ballot, then run one after the other.
    let holder = store.client().await?;
    holder
        .batch_execute(&format!(
            "BEGIN; SELECT 1 FROM ballot_box_received \
             WHERE election_event_id = '{event}' AND ballot_id = '{ballot_id}' FOR UPDATE"
        ))
        .await?;
    let casts: Vec<_> = (0..2)
        .map(|_| {
            let store = store.clone();
            let vote = ballot.vote(0);
            let ballot_id = ballot_id.clone();
            tokio::spawn(async move {
                store
                    .accept_received_ballot(&vote.request(), &cast(&ballot_id))
                    .await
            })
        })
        .collect();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(casts.iter().all(|cast| !cast.is_finished()));
    holder.batch_execute("COMMIT").await?;
    let mut outcomes = Vec::new();
    for cast in casts {
        outcomes.push(tokio::time::timeout(Duration::from_secs(5), cast).await???);
    }
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AcceptOutcome::Accepted { .. }))
            .count(),
        1,
        "{outcomes:?}"
    );
    assert!(
        outcomes.contains(&AcceptOutcome::DuplicateBallotId),
        "{outcomes:?}"
    );
    assert_eq!(store.voter_ballots(&event, "voter-a").await?.len(), 1);
    assert_eq!(
        store
            .voter_state(&event, &election, "voter-a")
            .await?
            .unwrap()
            .votes,
        1
    );
    drop_ballot_box(&store, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn a_received_ballot_of_a_sealed_box_stays_received() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let board = format!("sealtest{}", Uuid::new_v4().simple());
    store.create_board(&board).await?;
    let ballot = Received::new(&event, &election, &area, "voter-a");
    let ballot_id = ballot.receive(&store, &short_ballot_id()).await?;
    seal_box(&store, &board, ballot_box(&event, &election, &area), false).await?;
    assert_eq!(
        store
            .accept_received_ballot(&ballot.vote(0).request(), &cast(&ballot_id))
            .await?,
        AcceptOutcome::BallotBoxSealed
    );
    let kept = store
        .received_ballot_by_id(&event, &election, "voter-a", &ballot_id)
        .await?
        .unwrap();
    assert_eq!(kept.status, ReceivedStatus::Received);
    assert!(store.voter_ballots(&event, "voter-a").await?.is_empty());
    store.delete_board(&board).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn first_valid_votes_are_read_up_to_the_sequencer() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let mut sequenced = Vec::new();
    for vote in [
        Vote::new(&event, &election, &area, "v1", 0),
        Vote::new(&event, &election, &area, "v2", 0),
        Vote::new(&event, &election, &area, "v1", 0),
        Vote::new(&event, &election, &area, "v3", 0).with_status(BallotStatus::Rejected),
    ] {
        sequenced.push(vote.accept(&store).await?);
    }
    let waiting = Vote::new(&event, &election, &area, "v4", 0)
        .accept(&store)
        .await?;
    store.remove_pending(&event, &sequenced).await?;
    settle(&store, &event).await?;
    let accepted = store.voter_ballots(&event, "v1").await?[0].accepted_at;

    let read = store.first_valid_votes(&event, 0).await?;
    assert_eq!(read.next_seq, waiting - 1);
    let mut votes = read.votes;
    votes.sort_by(|a, b| a.voter_id.cmp(&b.voter_id));
    assert_eq!(votes.len(), 2);
    assert_eq!(
        votes[0],
        FirstValidVote {
            election_id: election.clone(),
            voter_id: "v1".into(),
            accepted_at: accepted,
        }
    );
    assert_eq!(votes[1].voter_id, "v2");

    // The next read starts where this one stopped, once the sequencer moved on.
    let again = store.first_valid_votes(&event, read.next_seq).await?;
    assert!(again.votes.is_empty());
    assert_eq!(again.next_seq, read.next_seq);
    store.remove_pending(&event, &[waiting]).await?;
    let next = store.first_valid_votes(&event, read.next_seq).await?;
    assert_eq!(next.next_seq, waiting);
    assert_eq!(
        next.votes
            .iter()
            .map(|vote| vote.voter_id.as_str())
            .collect::<Vec<_>>(),
        ["v4"]
    );
    drop_ballot_box(&store, &event).await
}

/// Moves the event's ballots back in time, as if they had been accepted
/// [`SEQUENCE_SETTLED_SECS`] ago and more.
async fn settle(store: &PostgresStore, event: &str) -> Result<()> {
    store
        .client()
        .await?
        .execute(
            "UPDATE ballot_box_ballot SET accepted_at = accepted_at - make_interval(secs => $2) \
             WHERE election_event_id = $1::text::uuid",
            &[&event, &(SEQUENCE_SETTLED_SECS + 1.0)],
        )
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn first_valid_votes_are_read_again_while_a_vote_may_be_in_progress() -> Result<()> {
    let store = store().await?;
    let (event, election, area, _) = ids();
    store.create_ballot_box(&event).await?;
    let first = Vote::new(&event, &election, &area, "v1", 0)
        .accept(&store)
        .await?;
    store.remove_pending(&event, &[first]).await?;
    settle(&store, &event).await?;

    // A vote in progress: it took its sequence number, but neither its ballot nor
    // its place in the sequencer's queue is visible until it commits.
    let in_progress = store.client().await?;
    in_progress.batch_execute("BEGIN").await?;
    let held: i64 = in_progress
        .query_one(
            "INSERT INTO ballot_box_ballot (election_event_id, ballot_id, election_id, \
                 area_id, voter_id, format, content, pseudonym_hash, ballot_hash, \
                 voting_channel, status) \
             VALUES ($1::text::uuid, $2, $3::text::uuid, $4::text::uuid, 'v2', 'test', \
                 'ciphertext', $5, $5, 'ONLINE', 'valid') \
             RETURNING seq",
            &[
                &event,
                &Uuid::new_v4().simple().to_string(),
                &election,
                &area,
                &vec![3u8; 64],
            ],
        )
        .await?
        .try_get(0)?;
    in_progress
        .execute(
            "INSERT INTO ballot_box_pending (election_event_id, seq) \
             VALUES ($1::text::uuid, $2)",
            &[&event, &held],
        )
        .await?;
    // A later vote commits and is sequenced first.
    let later = Vote::new(&event, &election, &area, "v3", 0)
        .accept(&store)
        .await?;
    assert!(later > held);
    store.remove_pending(&event, &[later]).await?;

    let voters = |read: &FirstValidVotes| {
        let mut voters: Vec<String> = read
            .votes
            .iter()
            .map(|vote| vote.voter_id.clone())
            .collect();
        voters.sort();
        voters
    };
    // The later vote is read, but the next read starts below the vote in progress.
    let read = store.first_valid_votes(&event, 0).await?;
    assert_eq!(voters(&read), ["v1", "v3"]);
    assert_eq!(read.next_seq, first);

    in_progress.batch_execute("COMMIT").await?;
    store.remove_pending(&event, &[held]).await?;
    let again = store.first_valid_votes(&event, read.next_seq).await?;
    assert_eq!(voters(&again), ["v2", "v3"]);
    assert_eq!(again.next_seq, first);

    // Once no vote that started before them can still be running, the cursor
    // passes them.
    settle(&store, &event).await?;
    let settled = store.first_valid_votes(&event, again.next_seq).await?;
    assert_eq!(voters(&settled), ["v2", "v3"]);
    assert_eq!(settled.next_seq, later);
    let after = store.first_valid_votes(&event, settled.next_seq).await?;
    assert!(after.votes.is_empty());
    assert_eq!(after.next_seq, later);
    drop_ballot_box(&store, &event).await
}
