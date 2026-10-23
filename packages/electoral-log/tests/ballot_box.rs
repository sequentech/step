// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box against a real server, in the database of
//! `ELECTORAL_LOG_TEST_DATABASE_URL`. Each test uses events of its own and drops
//! their partitions at the end.

use anyhow::Result;
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, Acceptance, BallotStatus, QueueState, VitrineQueue,
};
use electoral_log::adapters::ballot_box_reads::{
    tally_ballots_query, BallotIdMatch, BucketRange, IpBallotsFilter, Participation,
};
use electoral_log::adapters::ballot_box_status::{BallotToReview, VoterBallotState};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::ports::ElectoralLogStore;
use electoral_log::vitrine::{leaf_hash, ElectionEntry};
use electoral_log::{ElectoralLogMessage, LogEntry};
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
    vitrine: Option<VitrineQueue>,
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
            vitrine: None,
        }
    }

    /// A vote of a transparent event, with a ballot ID that is a 32-byte hash.
    fn queued_in(mut self, log_id: i64, accepted_at: i64) -> Self {
        self.ballot = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        self.vitrine = Some(VitrineQueue {
            log_id,
            accepted_at,
        });
        self
    }

    fn at(mut self, accepted_at: i64) -> Self {
        if let Some(queue) = self.vitrine.as_mut() {
            queue.accepted_at = accepted_at;
        }
        self
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
            AcceptOutcome::Accepted { seq, id, .. } => Ok((seq, id)),
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
            vitrine: self.vitrine,
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
            .voters_by_channel(&event, Some(second.as_str()))
            .await?,
        [("ONLINE".to_string(), 1)]
    );

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
        .ballots_per_bucket(&event, Some(second.as_str()), &three_days, "ONLINE")
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
        store.participation(&event, None).await?,
        Participation {
            ballots: 4,
            voters: 3
        }
    );
    assert_eq!(
        store.participation(&event, Some(first.as_str())).await?,
        Participation {
            ballots: 3,
            voters: 2
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

// ACCEPT_WITH_VITRINE against a minimal copy of the Vitrine queue: the shape
// of vitrine.queue and of a log's partitions, defaults and grants in Vitrine's
// crates/vitrine-store/migrations/0001_schema.sql and logs::grant_queue. The
// votes run as a role that has only the ballot box's privileges and the grants
// the sidecar gives a queue log's application role.

const APP_ROLE: &str = "vitrine_contract_app";
const APP_PASSWORD: &str = "vitrine-contract-app";

const VITRINE_QUEUE: &str = r#"
CREATE SCHEMA IF NOT EXISTS vitrine;
CREATE TABLE IF NOT EXISTS vitrine.log_state (
    log_id     bigint PRIMARY KEY,
    queue_part smallint NOT NULL DEFAULT 0 CHECK (queue_part BETWEEN 0 AND 2)
);
CREATE TABLE IF NOT EXISTS vitrine.queue (
    log_id    bigint NOT NULL,
    part      smallint NOT NULL,
    id        bigint NOT NULL,
    txid      xid8 NOT NULL DEFAULT pg_current_xact_id(),
    entry     bytea NOT NULL,
    payloads  bytea,
    queued_at timestamptz NOT NULL DEFAULT now(),
    leaf_hash bytea GENERATED ALWAYS AS (sha256(decode('00', 'hex') || entry)) STORED
) PARTITION BY LIST (log_id);
CREATE OR REPLACE FUNCTION vitrine.queue_part(p_log bigint) RETURNS smallint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, pg_temp
AS $$ SELECT queue_part FROM vitrine.log_state WHERE log_id = p_log $$;
REVOKE ALL ON FUNCTION vitrine.queue_part(bigint) FROM PUBLIC;
DO $$ BEGIN
    CREATE ROLE vitrine_contract_app LOGIN PASSWORD 'vitrine-contract-app';
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
GRANT SELECT, INSERT, UPDATE ON ballot_box_ballot, ballot_box_voter, ballot_box_pending
    TO vitrine_contract_app;
GRANT USAGE ON SEQUENCE ballot_box_seq TO vitrine_contract_app;
"#;

static VITRINE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// A queue log's partitions and grants, as the sidecar creates them.
fn queue_log(log_id: i64) -> String {
    let q = format!("vitrine.queue_{log_id}");
    format!(
        "INSERT INTO vitrine.log_state (log_id) VALUES ({log_id});
         CREATE SEQUENCE {q}_id AS bigint;
         CREATE TABLE {q} PARTITION OF vitrine.queue FOR VALUES IN ({log_id})
             PARTITION BY LIST (part);
         ALTER TABLE {q} ALTER COLUMN log_id SET DEFAULT {log_id},
             ALTER COLUMN part SET DEFAULT vitrine.queue_part({log_id}),
             ALTER COLUMN id SET DEFAULT nextval('{q}_id'::regclass),
             ALTER COLUMN txid SET DEFAULT pg_current_xact_id(),
             ALTER COLUMN queued_at SET DEFAULT now();
         CREATE TABLE {q}_0 PARTITION OF {q} FOR VALUES IN (0);
         CREATE TABLE {q}_1 PARTITION OF {q} FOR VALUES IN (1);
         CREATE TABLE {q}_2 PARTITION OF {q} FOR VALUES IN (2);
         GRANT USAGE ON SCHEMA vitrine TO {APP_ROLE};
         GRANT INSERT ON {q} TO {APP_ROLE};
         GRANT USAGE ON SEQUENCE {q}_id TO {APP_ROLE};
         GRANT EXECUTE ON FUNCTION vitrine.queue_part(bigint) TO {APP_ROLE};"
    )
}

/// The ballot box as the application role, and a new queue log for it.
async fn vitrine_store(admin: &PostgresStore) -> Result<(PostgresStore, i64)> {
    VITRINE
        .get_or_try_init(|| async {
            admin.client().await?.batch_execute(VITRINE_QUEUE).await?;
            anyhow::Ok(())
        })
        .await?;
    let log_id = (Uuid::new_v4().as_u128() % 1_000_000_000_000) as i64 + 1;
    admin
        .client()
        .await?
        .batch_execute(&queue_log(log_id))
        .await?;
    let mut config: tokio_postgres::Config =
        std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    config.user(APP_ROLE).password(APP_PASSWORD);
    Ok((PostgresStore::new(config)?, log_id))
}

/// The entries queued in a log, with the leaf hashes the queue computed.
async fn queued(admin: &PostgresStore, log_id: i64) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let rows = admin
        .client()
        .await?
        .query(
            &format!("SELECT entry, leaf_hash FROM vitrine.queue_{log_id} ORDER BY id"),
            &[],
        )
        .await?;
    Ok(rows.iter().map(|r| (r.get(0), r.get(1))).collect())
}

async fn stored_accepted_at(admin: &PostgresStore, event: &str, ballot: &str) -> Result<f64> {
    Ok(admin
        .client()
        .await?
        .query_one(
            "SELECT extract(epoch FROM accepted_at)::float8 FROM ballot_box_ballot \
             WHERE election_event_id = $1::text::uuid AND ballot_id = $2",
            &[&event, &ballot],
        )
        .await?
        .get(0))
}

const ACCEPTED_AT: i64 = 1_793_001_660;

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn an_accepted_ballot_queues_its_entry_with_it() -> Result<()> {
    let admin = store().await?;
    let (app, log_id) = vitrine_store(&admin).await?;
    let (event, election, area, _) = ids();
    admin.create_ballot_box(&event).await?;

    let vote = Vote::new(&event, &election, &area, "voter", 1).queued_in(log_id, ACCEPTED_AT);
    let AcceptOutcome::Accepted {
        acceptance,
        vitrine: Some(vitrine),
        ..
    } = app.accept_ballot(&vote.request()).await?
    else {
        anyhow::bail!("not accepted with an entry");
    };
    assert_eq!(acceptance, Acceptance::New);
    assert_eq!(vitrine.queue, QueueState::Queued);
    let expected = ElectionEntry {
        event_id: event.clone(),
        election_id: election.clone(),
        ballot_id: hex::decode(&vote.ballot)?.try_into().unwrap(),
        accepted_at: ACCEPTED_AT as u64,
    };
    assert_eq!(vitrine.entry, expected.encode()?);
    assert_eq!(vitrine.leaf_hash, leaf_hash(&vitrine.entry));
    assert_eq!(
        queued(&admin, log_id).await?,
        vec![(vitrine.entry.clone(), vitrine.leaf_hash.to_vec())]
    );
    assert_eq!(
        stored_accepted_at(&admin, &event, &vote.ballot).await?,
        ACCEPTED_AT as f64
    );
    drop_ballot_box(&admin, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn a_refused_vote_queues_nothing() -> Result<()> {
    let admin = store().await?;
    let (app, log_id) = vitrine_store(&admin).await?;
    let (event, election, area, other_area) = ids();
    admin.create_ballot_box(&event).await?;

    let first = Vote::new(&event, &election, &area, "voter", 1).queued_in(log_id, ACCEPTED_AT);
    assert!(matches!(
        app.accept_ballot(&first.request()).await?,
        AcceptOutcome::Accepted { .. }
    ));
    let second = Vote::new(&event, &election, &area, "voter", 1).queued_in(log_id, ACCEPTED_AT);
    assert_eq!(
        app.accept_ballot(&second.request()).await?,
        AcceptOutcome::TooManyVotes
    );
    let elsewhere =
        Vote::new(&event, &election, &other_area, "voter", 0).queued_in(log_id, ACCEPTED_AT);
    assert_eq!(
        app.accept_ballot(&elsewhere.request()).await?,
        AcceptOutcome::VotedInOtherArea
    );
    let mut clash = Vote::new(&event, &election, &area, "other", 0).queued_in(log_id, ACCEPTED_AT);
    clash.ballot = first.ballot.clone();
    assert_eq!(
        app.accept_ballot(&clash.request()).await?,
        AcceptOutcome::DuplicateBallotId
    );
    assert_eq!(queued(&admin, log_id).await?.len(), 1);
    drop_ballot_box(&admin, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn a_retried_cast_returns_the_original_acceptance() -> Result<()> {
    let admin = store().await?;
    let (app, log_id) = vitrine_store(&admin).await?;
    let (event, election, area, _) = ids();
    admin.create_ballot_box(&event).await?;

    for allowed in [1, 0] {
        let voter = format!("voter-{allowed}");
        let vote =
            Vote::new(&event, &election, &area, &voter, allowed).queued_in(log_id, ACCEPTED_AT);
        let first = app.accept_ballot(&vote.request()).await?;
        let retry = vote.at(ACCEPTED_AT + 7);
        let again = app.accept_ballot(&retry.request()).await?;
        let (
            AcceptOutcome::Accepted {
                seq,
                id,
                vitrine: Some(original),
                ..
            },
            AcceptOutcome::Accepted {
                seq: seq_again,
                id: id_again,
                acceptance,
                vitrine: Some(replayed),
                ..
            },
        ) = (first, again)
        else {
            anyhow::bail!("allowed {allowed}: the retry wasn't accepted");
        };
        assert_eq!((seq, id), (seq_again, id_again), "allowed {allowed}");
        assert_eq!(acceptance, Acceptance::Replayed);
        assert_eq!(replayed.entry, original.entry);
        assert_eq!(replayed.leaf_hash, original.leaf_hash);
        assert_eq!(
            app.voter_state(&event, &election, &voter)
                .await?
                .unwrap()
                .votes,
            1
        );
    }
    assert_eq!(queued(&admin, log_id).await?.len(), 2);

    // A ballot rejected since isn't replayed.
    let vote = Vote::new(&event, &election, &area, "rejected", 1).queued_in(log_id, ACCEPTED_AT);
    app.accept_ballot(&vote.request()).await?;
    admin
        .client()
        .await?
        .execute(
            "UPDATE ballot_box_ballot SET status = 'rejected' \
             WHERE election_event_id = $1::text::uuid AND ballot_id = $2",
            &[&event, &vote.ballot],
        )
        .await?;
    assert_eq!(
        app.accept_ballot(&vote.request()).await?,
        AcceptOutcome::TooManyVotes
    );
    drop_ballot_box(&admin, &event).await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn a_queue_that_refuses_the_insert_never_refuses_the_ballot() -> Result<()> {
    let admin = store().await?;
    let (app, log_id) = vitrine_store(&admin).await?;
    let (event, election, area, _) = ids();
    admin.create_ballot_box(&event).await?;

    let first = Vote::new(&event, &election, &area, "first", 1).queued_in(log_id, ACCEPTED_AT);
    assert!(matches!(
        app.accept_ballot(&first.request()).await?,
        AcceptOutcome::Accepted { .. }
    ));
    // Freezing a log takes the insert grant back.
    admin
        .client()
        .await?
        .batch_execute(&format!(
            "REVOKE INSERT ON vitrine.queue_{log_id} FROM {APP_ROLE}"
        ))
        .await?;
    let frozen = Vote::new(&event, &election, &area, "frozen", 1).queued_in(log_id, ACCEPTED_AT);
    let AcceptOutcome::Accepted {
        vitrine: Some(vitrine),
        ..
    } = app.accept_ballot(&frozen.request()).await?
    else {
        anyhow::bail!("refused");
    };
    assert_eq!(vitrine.queue, QueueState::NotQueued);
    assert_eq!(
        stored_accepted_at(&admin, &event, &frozen.ballot).await?,
        ACCEPTED_AT as f64
    );

    // A log whose queue partition doesn't exist.
    let missing =
        Vote::new(&event, &election, &area, "missing", 1).queued_in(log_id + 1, ACCEPTED_AT);
    let AcceptOutcome::Accepted {
        vitrine: Some(vitrine),
        ..
    } = app.accept_ballot(&missing.request()).await?
    else {
        anyhow::bail!("refused");
    };
    assert_eq!(vitrine.queue, QueueState::NotQueued);
    assert_eq!(queued(&admin, log_id).await?.len(), 1);
    drop_ballot_box(&admin, &event).await
}
