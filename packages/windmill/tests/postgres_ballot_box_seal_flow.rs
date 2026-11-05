// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! VOTE-FREEZE end to end on a private migrated Hasura database and the
//! event's own electoral-log database: every close path creates the seals
//! with its provenance, closed voting never reopens, sealed elections and
//! events can't be deleted, a cast after the seal deadline or into a sealed
//! box gets the closed-voting error, and the sealer and publisher seal,
//! sign, record and publish each box once.
//!
//! Needs `HASURA_DB__*` and `ELECTORAL_LOG_PG_*` (the stack's
//! `.devcontainer/.env`, or the PostgreSQL service of the CI jobs) and sets
//! its own `MASTER_SECRET`. Votes go through the ballot box and reach the
//! log through the sequencer's steps. The census and the public bucket are
//! the test's own ([`TestEnvironment`]); the signing key is the event's real
//! protocol manager key from the database vault.

#![recursion_limit = "256"]

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use deadpool_postgres::{Client, Pool, Transaction};
use electoral_log::adapters::ballot_box::{AcceptOutcome, BallotStatus};
use electoral_log::adapters::ballot_box_seal::is_sealed_refusal;
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::messages::statement::StatementType;
use electoral_log::ports::ElectoralLogStore;
use electoral_log::seal::{verify_record, SealDisposition, SealRecord};
use electoral_log::{Filter, LogEntry, LogQuery, SqlCompOperators, TextColumn};
use event_database::{EventDatabase, Vote};
use sequent_core::ballot::BallotBoxSealRecordPolicy;
use sequent_core::ballot::{
    EGracePeriodPolicy, ElectionEventStatus, ElectionPresentation, ElectionStatus,
    SignedHashableBallot, VotingPeriodEnd, VotingStatus, VotingStatusChannel, TYPES_VERSION,
};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Once};
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use voting_crypto::RistrettoCtx;
use windmill::postgres::ballot_box_seal::{
    list_for_elections, BallotBoxSeal, BallotBoxSealStatus, ClosedBy, ClosedBySigner,
};
use windmill::postgres::election_event::{delete_election_event, SEALED_EVENT_DELETE_REFUSAL};
use windmill::postgres::trusted_write;
use windmill::services::ballot_box_seal::publish::{
    publish_box, record_document_id, PublishOutcome,
};
use windmill::services::ballot_box_seal::seal::{
    computed_ballot_id, seal_box, unrecorded_reason, SealOutcome, ALREADY_ON_BOARD_REASON,
};
use windmill::services::ballot_box_seal::sink::{closing_ballot_boxes, PendingSealSink};
use windmill::services::ballot_box_seal::{
    is_ballot_box_sealed, on_close, Census, CloseProvenance, ProductionSealEnvironment,
    SealEnvironment,
};
use windmill::services::election_event_status::{
    scheduled_change_applies, update_event_voting_status, update_scheduled_event_voting_status,
    update_scheduled_event_voting_status_for, SkippedElection, TransitionRefusal,
};
use windmill::services::electoral_log_board::{
    create_protocol_manager_keys, get_board_signing_key, get_event_databases, get_event_store,
};
use windmill::services::insert_cast_vote::{check_seal_deadline, CastVoteError};
use windmill::services::signing::actions::voting::{
    ClosingSignature, PostCloser, SealRecord as CloseRecord, StatusCloser,
};
use windmill::services::signing::actions::SealRecordSink;
use windmill::services::voting_status::update_election_status;

const ADMIN: &str = "admin";
const QUARTER_HOUR_SECS: u64 = 15 * 60;

/// The secrets of an event are encrypted with the deployment's master secret.
fn master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

/// The census, the public bucket and the key: the key is the event's
/// real one, the rest is the test's. The tests run the sequencer
/// themselves, so a request for it is only counted and not waited for.
struct TestEnvironment {
    census: Census,
    uploads: Mutex<Vec<(String, Vec<u8>)>>,
    /// The record policy of each upload, in order.
    upload_policies: Mutex<Vec<BallotBoxSealRecordPolicy>>,
    /// How many uploads fail before one succeeds.
    failing_uploads: AtomicUsize,
    sequencer_requests: AtomicUsize,
}

impl TestEnvironment {
    fn new(census: &[(&str, u64)]) -> Self {
        TestEnvironment {
            census: census
                .iter()
                .map(|(voter, weight)| (voter.to_string(), *weight))
                .collect(),
            uploads: Mutex::new(vec![]),
            upload_policies: Mutex::new(vec![]),
            failing_uploads: AtomicUsize::new(0),
            sequencer_requests: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl SealEnvironment for TestEnvironment {
    async fn event_store(&self, election_event_id: &str) -> Result<PostgresStore> {
        get_event_store(election_event_id).await
    }

    async fn request_sequencer(&self, _: &str) -> Result<()> {
        self.sequencer_requests.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn sequencer_wait(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }

    async fn signing_key(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> Result<SigningKey> {
        ProductionSealEnvironment
            .signing_key(hasura_transaction, tenant_id, election_event_id, board)
            .await
    }

    async fn census(
        &self,
        _: &sequent_core::types::hasura::core::ElectionEvent,
        _: &str,
        _: &str,
    ) -> Result<Census> {
        Ok(self.census.clone())
    }

    async fn upload_record(
        &self,
        _: &Transaction<'_>,
        _: &str,
        _: &str,
        name: &str,
        json: &[u8],
        document_id: Uuid,
        policy: BallotBoxSealRecordPolicy,
    ) -> Result<Uuid> {
        if self
            .failing_uploads
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_ok()
        {
            anyhow::bail!("The bucket is down");
        }
        self.uploads
            .lock()
            .unwrap()
            .push((name.to_string(), json.to_vec()));
        self.upload_policies.lock().unwrap().push(policy);
        Ok(document_id)
    }
}

/// An event with its electoral-log database and key, its elections and two
/// areas.
struct World {
    pool: Pool,
    database: EventDatabase,
    key: SigningKey,
    tenant: Uuid,
    event: Uuid,
    elections: Vec<Uuid>,
    areas: [Uuid; 2],
    user: String,
}

impl World {
    fn election(&self) -> Uuid {
        self.elections[0]
    }

    fn board(&self) -> &str {
        &self.database.board
    }

    async fn client(&self) -> Client {
        self.pool.get().await.unwrap()
    }

    async fn seals(&self, election: Uuid) -> Vec<BallotBoxSeal> {
        let mut client = self.client().await;
        let tx = client.transaction().await.unwrap();
        list_for_elections(&tx, &self.tenant, &self.event, &[election])
            .await
            .unwrap()
    }

    async fn status(&self, election: Uuid) -> ElectionStatus {
        let client = self.client().await;
        let value: Value = client
            .query_one(
                "SELECT status FROM sequent_backend.election WHERE id = $1",
                &[&election],
            )
            .await
            .unwrap()
            .get(0);
        serde_json::from_value(value).unwrap()
    }

    /// Drops the event's electoral-log database.
    async fn finish(self) {
        self.database.drop().await;
    }
}

struct Options {
    policy: &'static str,
    elections: usize,
    kiosk: bool,
    early_voting: bool,
    /// Whether the elections' ONLINE voting is open (else never started).
    online_open: bool,
    /// Whether the elections enable ONLINE.
    online_enabled: bool,
    grace_secs: Option<u64>,
    /// The event's Seal Record Publication policy; None leaves it unset.
    record_policy: Option<&'static str>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            policy: "seal-at-close",
            elections: 1,
            kiosk: false,
            early_voting: false,
            online_open: true,
            online_enabled: true,
            grace_secs: None,
            record_policy: None,
        }
    }
}

async fn world(options: Options) -> World {
    let pool = schema::pool().await;
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let elections: Vec<Uuid> = (0..options.elections).map(|_| Uuid::new_v4()).collect();
    let areas = [Uuid::new_v4(), Uuid::new_v4()];
    let database = EventDatabase::create(tenant, event).await;
    let board = database.board.clone();
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    let mut event_status = ElectionEventStatus::default();
    event_status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
    tx.execute(
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, status, presentation,
              bulletin_board_reference, voting_channels)
         VALUES ($1, $2, 'RSA256', $3, $4, $5, $6)",
        &[
            &event,
            &tenant,
            &serde_json::to_value(&event_status).unwrap(),
            &match options.record_policy {
                Some(record_policy) => json!({
                    "ballot_box_seal_policy": options.policy,
                    "ballot_box_seal_record_policy": record_policy,
                }),
                None => json!({ "ballot_box_seal_policy": options.policy }),
            },
            &json!({"id": 1, "database_name": board, "is_archived": false}),
            &json!({"online": true, "kiosk": options.kiosk, "early_voting": options.early_voting}),
        ],
    )
    .await
    .unwrap();
    let presentation = ElectionPresentation {
        grace_period_policy: Some(match options.grace_secs {
            Some(_) => EGracePeriodPolicy::GRACE_PERIOD_WITHOUT_ALERT,
            None => EGracePeriodPolicy::NO_GRACE_PERIOD,
        }),
        grace_period_secs: options.grace_secs,
        voting_period_end: Some(VotingPeriodEnd::ALLOWED),
        ..Default::default()
    };
    for election in &elections {
        let mut status = ElectionStatus::default();
        if options.online_open {
            status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        }
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, status, voting_channels, presentation,
                  num_allowed_revotes)
             VALUES ($1, $2, $3, $4, $5, $6, 0)",
            &[
                election,
                &tenant,
                &event,
                &serde_json::to_value(&status).unwrap(),
                &json!({"online": options.online_enabled, "kiosk": options.kiosk, "early_voting": options.early_voting}),
                &serde_json::to_value(&presentation).unwrap(),
            ],
        )
        .await
        .unwrap();
    }
    for area in areas {
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, 'Area')",
            &[&area, &tenant, &event],
        )
        .await
        .unwrap();
    }
    master_secret();
    create_protocol_manager_keys(&tx, &tenant.to_string(), &event.to_string(), &board)
        .await
        .unwrap();
    let key = get_board_signing_key(&tx, &tenant.to_string(), Some(&event.to_string()), &board)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    World {
        pool,
        database,
        key,
        tenant,
        event,
        elections,
        areas,
        user: Uuid::new_v4().to_string(),
    }
}

/// A stored ballot's content and its Ballot ID.
fn ballot(seed: &str) -> (String, String) {
    let content = serde_json::to_string(&SignedHashableBallot {
        version: TYPES_VERSION,
        issue_date: seed.to_string(),
        contests: vec![],
        config: seed.to_string(),
        ballot_style_hash: seed.to_string(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    })
    .unwrap();
    let ballot_id = computed_ballot_id(&content, event_database::FORMAT).unwrap();
    (content, ballot_id)
}

/// Casts a ballot of `voter` into the box (election, area), valid, rejected
/// (discarded) or pending (a Datafix vote in progress), and puts it on the
/// log as the sequencer does; returns its Ballot ID.
async fn cast(
    w: &World,
    election: Uuid,
    area: Uuid,
    voter: &str,
    status: BallotStatus,
    seed: &str,
) -> String {
    let (content, ballot_id) = ballot(seed);
    store_ballot(
        w, election, area, voter, status, &content, &ballot_id, "ONLINE",
    )
    .await;
    ballot_id
}

/// Casts a ballot as [`cast`] does, with its content, Ballot ID and channel.
#[allow(clippy::too_many_arguments)]
async fn store_ballot(
    w: &World,
    election: Uuid,
    area: Uuid,
    voter: &str,
    status: BallotStatus,
    content: &str,
    ballot_id: &str,
    channel: &str,
) {
    let vote = Vote {
        election,
        area,
        voter,
        content,
        ballot_id,
        channel,
    };
    match status {
        BallotStatus::Valid => {
            w.database.cast(&vote).await;
        }
        BallotStatus::Rejected => w.database.cast_rejected(&vote).await,
        BallotStatus::Pending => assert!(matches!(
            w.database.try_cast(&vote, BallotStatus::Pending).await,
            AcceptOutcome::Accepted { .. }
        )),
    }
    w.database.sequence(&w.key).await;
}

/// How a late vote into the box (election, area) is answered.
async fn late_cast(w: &World, election: Uuid, area: Uuid, seed: &str) -> AcceptOutcome {
    let (content, ballot_id) = ballot(seed);
    w.database
        .try_cast(
            &Vote {
                election,
                area,
                voter: "voter-late",
                content: &content,
                ballot_id: &ballot_id,
                channel: "ONLINE",
            },
            BallotStatus::Valid,
        )
        .await
}

/// Stop Voting at an election, as the route (with a user) or the scheduler
/// (without) does it.
async fn close_election(w: &World, election: Uuid, user: Option<&str>) {
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    update_election_status(
        w.tenant.to_string(),
        user,
        user.map(|_| ADMIN),
        &tx,
        &w.event.to_string(),
        &election.to_string(),
        &VotingStatus::CLOSED,
        &Some(vec![VotingStatusChannel::ONLINE]),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

fn assert_closed_by(seals: &[BallotBoxSeal], expected: &ClosedBy) {
    assert!(!seals.is_empty());
    for seal in seals {
        assert_eq!(seal.status, BallotBoxSealStatus::Pending);
        assert_eq!(&seal.closed_by, expected);
    }
}

// ---------------------------------------------------------------------------
// Close paths and provenance
// ---------------------------------------------------------------------------

#[test]
fn a_manual_post_close_creates_one_pending_seal_per_box() {
    event_database::run(async {
        let w = world(Options {
            grace_secs: Some(QUARTER_HOUR_SECS),
            ..Default::default()
        })
        .await;
        // One box has ballots; the other has none and no ballot style, so only
        // the first is a box of the election here.
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let seals = w.seals(w.election()).await;
        assert_eq!(seals.len(), 1);
        assert_eq!(seals[0].area_id, w.areas[0]);
        assert_closed_by(
            &seals,
            &ClosedBy::User {
                username: Some(ADMIN.into()),
            },
        );
        let stopped = w
            .status(w.election())
            .await
            .voting_period_dates
            .last_stopped_at
            .unwrap();
        // The column keeps microseconds.
        assert_eq!(
            seals[0].closed_at.timestamp_micros(),
            stopped.timestamp_micros()
        );
        assert_eq!(
            seals[0].grace_deadline.timestamp_micros(),
            (stopped + Duration::seconds(QUARTER_HOUR_SECS as i64)).timestamp_micros()
        );
        w.finish().await;
    })
}

#[test]
fn a_scheduled_post_close_and_a_signed_scheduled_close_are_scheduled() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            ..Default::default()
        })
        .await;
        for election in &w.elections {
            cast(
                &w,
                *election,
                w.areas[0],
                &format!("voter-{election}"),
                BallotStatus::Valid,
                &election.to_string(),
            )
            .await;
        }
        // manage_election_dates: no user.
        close_election(&w, w.elections[0], None).await;
        assert_closed_by(&w.seals(w.elections[0]).await, &ClosedBy::Scheduled);
        // enforce_signed_closes: StatusCloser, no user.
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        StatusCloser
            .close(
                &tx,
                w.tenant,
                w.event,
                w.elections[1],
                &[VotingStatusChannel::ONLINE],
            )
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_closed_by(&w.seals(w.elections[1]).await, &ClosedBy::Scheduled);
        w.finish().await;
    })
}

#[test]
fn an_event_close_seals_every_election_with_its_provenance() {
    event_database::run(async {
        for scheduled in [false, true] {
            let w = world(Options {
                elections: 2,
                ..Default::default()
            })
            .await;
            for election in &w.elections {
                cast(
                    &w,
                    *election,
                    w.areas[1],
                    &format!("voter-{election}"),
                    BallotStatus::Valid,
                    &election.to_string(),
                )
                .await;
            }
            let mut client = w.client().await;
            let tx = client.transaction().await.unwrap();
            let channels = Some(vec![VotingStatusChannel::ONLINE]);
            if scheduled {
                update_scheduled_event_voting_status(
                    &tx,
                    &w.tenant.to_string(),
                    None,
                    None,
                    &w.event.to_string(),
                    &VotingStatus::CLOSED,
                    &channels,
                    &HashSet::new(),
                )
                .await
                .unwrap();
            } else {
                update_event_voting_status(
                    &tx,
                    &w.tenant.to_string(),
                    Some(&w.user),
                    Some(ADMIN),
                    &w.event.to_string(),
                    &VotingStatus::CLOSED,
                    &channels,
                )
                .await
                .unwrap();
            }
            tx.commit().await.unwrap();
            let expected = if scheduled {
                ClosedBy::Scheduled
            } else {
                ClosedBy::User {
                    username: Some(ADMIN.into()),
                }
            };
            for election in &w.elections {
                assert_closed_by(&w.seals(*election).await, &expected);
            }
            w.finish().await;
        }
    })
}

#[test]
fn a_signed_close_records_its_request_and_signers() {
    event_database::run(async {
        let w = world(Options::default()).await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        let request = Uuid::new_v4();
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        // The signed effect closes as its requester, then the sink runs in the
        // same transaction.
        update_election_status(
            w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &tx,
            &w.event.to_string(),
            &w.election().to_string(),
            &VotingStatus::CLOSED,
            &Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await
        .unwrap();
        let record = CloseRecord {
            closed_at: Utc::now(),
            election_id: Some(w.election()),
            channels: vec!["ONLINE".into()],
            from: vec!["ONLINE=OPEN".into()],
            code: "CLOSE-1".into(),
            payload_sha256: "00".into(),
            signatures: vec![ClosingSignature {
                user_id: "u1".into(),
                username: "chair".into(),
                display_name: "Chair Person".into(),
                signed_at: Utc::now(),
                certificate_fingerprint: "ab12".into(),
                certificate_subject: None,
                certificate_cn: None,
            }],
            seals: vec![],
            authorized_by: None,
            unsigned: false,
        };
        let sink = PendingSealSink {
            tenant_id: w.tenant,
            election_event_id: w.event,
            request_id: request,
        };
        let summaries = sink.feed(&tx, &record).await.unwrap();
        tx.commit().await.unwrap();
        assert!(summaries.is_empty(), "nothing sealed yet");
        let seals = w.seals(w.election()).await;
        assert_closed_by(
            &seals,
            &ClosedBy::Signed {
                signers: Some(vec![ClosedBySigner {
                    name: "Chair Person".into(),
                    certificate_sha256: "ab12".into(),
                }]),
                signing_code: Some("CLOSE-1".into()),
            },
        );
        assert_eq!(seals[0].close_request_id, Some(request));
        // The signing panel reads the Post's boxes as they stand: each country
        // pending, by its name, since sealing comes after the commit.
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        let boxes = closing_ballot_boxes(&tx, w.tenant, w.event, w.election(), request)
            .await
            .unwrap();
        assert_eq!(boxes.len(), seals.len());
        for ballot_box in &boxes {
            assert_eq!(ballot_box.status, BallotBoxSealStatus::Pending);
            assert_ne!(ballot_box.area_name, ballot_box.area_id.to_string());
            assert_eq!(ballot_box.seal_hash, None);
        }
        // Another request's panel doesn't claim these boxes.
        let other = closing_ballot_boxes(&tx, w.tenant, w.event, w.election(), Uuid::new_v4())
            .await
            .unwrap();
        assert!(other.is_empty());
        w.finish().await;
    })
}

#[test]
fn policy_off_creates_no_seal_and_lets_voting_reopen() {
    event_database::run(async {
        let w = world(Options {
            policy: "do-not-seal",
            ..Default::default()
        })
        .await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        assert!(w.seals(w.election()).await.is_empty());
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        update_election_status(
            w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &tx,
            &w.event.to_string(),
            &w.election().to_string(),
            &VotingStatus::OPEN,
            &Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await
        .unwrap();
        w.finish().await;
    })
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

async fn change_election(
    w: &World,
    election: Uuid,
    status: VotingStatus,
    channel: VotingStatusChannel,
) -> anyhow::Result<()> {
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    update_election_status(
        w.tenant.to_string(),
        Some(&w.user),
        Some(ADMIN),
        &tx,
        &w.event.to_string(),
        &election.to_string(),
        &status,
        &Some(vec![channel]),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[test]
fn closed_voting_never_reopens() {
    event_database::run(async {
        let w = world(Options {
            kiosk: true,
            ..Default::default()
        })
        .await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        // ONLINE closed, KIOSK enabled and never started: it holds the seal.
        assert!(w.seals(w.election()).await.is_empty());
        // With Seal at close, the never-started KIOSK may be stopped directly,
        // and that finishes voting: the box is due.
        change_election(
            &w,
            w.election(),
            VotingStatus::CLOSED,
            VotingStatusChannel::KIOSK,
        )
        .await
        .unwrap();
        let seals = w.seals(w.election()).await;
        assert_eq!(seals.len(), 1);
        let online_stop = w
            .status(w.election())
            .await
            .voting_period_dates
            .last_stopped_at
            .unwrap();
        // The close time is ONLINE's, which ran.
        assert_eq!(
            seals[0].closed_at.timestamp_micros(),
            online_stop.timestamp_micros()
        );
        let reopen = change_election(
            &w,
            w.election(),
            VotingStatus::OPEN,
            VotingStatusChannel::ONLINE,
        )
        .await
        .unwrap_err();
        assert!(
            reopen.to_string().starts_with("Voting can't start again"),
            "{reopen}"
        );
        // Nor does the KIOSK channel, closed without ever starting.
        let kiosk = change_election(
            &w,
            w.election(),
            VotingStatus::OPEN,
            VotingStatusChannel::KIOSK,
        )
        .await
        .unwrap_err();
        assert!(
            kiosk.to_string().starts_with("Voting can't start again"),
            "{kiosk}"
        );
        w.finish().await;
    })
}

#[test]
fn an_event_opening_leaves_closed_posts_closed() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            ..Default::default()
        })
        .await;
        let [closed, paused] = [w.elections[0], w.elections[1]];
        cast(&w, closed, w.areas[0], "voter-a", BallotStatus::Valid, "a").await;
        close_election(&w, closed, Some(&w.user)).await;
        change_election(
            &w,
            paused,
            VotingStatus::PAUSED,
            VotingStatusChannel::ONLINE,
        )
        .await
        .unwrap();
        let mut skipped = vec![];
        for status in [VotingStatus::PAUSED, VotingStatus::OPEN] {
            let mut client = w.client().await;
            let tx = client.transaction().await.unwrap();
            skipped = update_event_voting_status(
                &tx,
                &w.tenant.to_string(),
                Some(&w.user),
                Some(ADMIN),
                &w.event.to_string(),
                &status,
                &Some(vec![VotingStatusChannel::ONLINE]),
            )
            .await
            .unwrap()
            .1;
            tx.commit().await.unwrap();
        }
        // The caller is told which Post stayed closed, and why.
        assert_eq!(
            skipped,
            vec![SkippedElection {
                election_id: closed.to_string(),
                election_name: closed.to_string(),
                reason: "ballot-box-seal-policy".into(),
            }]
        );
        assert_eq!(
            w.status(closed)
                .await
                .status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::CLOSED
        );
        assert_eq!(
            w.status(paused)
                .await
                .status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::OPEN
        );
        w.finish().await;
    })
}

#[test]
fn an_election_or_event_with_seals_cannot_be_deleted() {
    event_database::run(async {
        let w = world(Options::default()).await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;

        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        let error = delete_election_event(&tx, &w.tenant.to_string(), &w.event.to_string())
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), SEALED_EVENT_DELETE_REFUSAL);
        assert!(error
            .downcast_ref::<windmill::postgres::election_event::SealedEventDeleteRefusal>()
            .is_some());
        drop(tx);

        let error = client
            .execute(
                "DELETE FROM sequent_backend.election WHERE id = $1",
                &[&w.election()],
            )
            .await
            .unwrap_err();
        let db = error.as_db_error().unwrap();
        assert_eq!(db.code().code(), "42501");
        assert_eq!(db.message(), "ballot_box_sealed");
        assert_eq!(
            db.detail(),
            Some("This election has sealed ballot boxes and cannot be deleted.")
        );
        w.finish().await;
    })
}

// ---------------------------------------------------------------------------
// The cast check
// ---------------------------------------------------------------------------

async fn cast_check(w: &World, area: uuid::Uuid) -> Result<(), CastVoteError> {
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    let event = windmill::postgres::election_event::get_election_event_by_id(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
    )
    .await
    .unwrap();
    let election = windmill::postgres::election::get_election_by_id(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &w.election().to_string(),
    )
    .await
    .unwrap()
    .unwrap();
    let status: ElectionStatus = serde_json::from_value(election.status.clone().unwrap()).unwrap();
    let channels = serde_json::from_value(election.voting_channels.clone().unwrap()).unwrap();
    check_seal_deadline(
        &tx,
        &event,
        &w.election().to_string(),
        &area,
        Utc::now().into(),
        VotingStatusChannel::ONLINE,
        &status,
        &channels,
        &election.get_presentation().unwrap_or_default(),
    )
    .await
}

#[test]
fn a_cast_after_the_seal_deadline_is_refused() {
    event_database::run(async {
        let w = world(Options {
            grace_secs: Some(QUARTER_HOUR_SECS),
            ..Default::default()
        })
        .await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        // Open: no deadline yet.
        cast_check(&w, w.areas[0]).await.unwrap();
        close_election(&w, w.election(), Some(&w.user)).await;
        // Within the grace period of the box's seal row.
        cast_check(&w, w.areas[0]).await.unwrap();
        // The row's deadline has passed (moved past the permanence trigger,
        // for this transaction only).
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        tx.batch_execute("SET LOCAL session_replication_role = replica")
            .await
            .unwrap();
        tx.execute(
        "UPDATE sequent_backend.ballot_box_seal SET grace_deadline = now() - interval '1 second'
         WHERE election_id = $1",
        &[&w.election()],
    )
    .await
    .unwrap();
        tx.commit().await.unwrap();
        assert!(matches!(
            cast_check(&w, w.areas[0]).await,
            Err(CastVoteError::CheckStatusFailed(_))
        ));
        // A box without a row falls back to the election's own deadline, which
        // is still in the grace period.
        cast_check(&w, w.areas[1]).await.unwrap();
        w.finish().await;
    })
}

#[test]
fn a_cast_into_a_sealed_box_gets_the_closed_voting_error() {
    event_database::run(async {
        let w = world(Options::default()).await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = w.seals(w.election()).await.remove(0);
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        assert_eq!(
            late_cast(&w, w.election(), w.areas[0], "late").await,
            AcceptOutcome::BallotBoxSealed
        );
        // Nor does any ballot of the box change.
        let refusal = w
            .database
            .store
            .reject_voter_ballots(&w.event.to_string(), "voter-a")
            .await
            .unwrap_err();
        assert!(is_ballot_box_sealed(&refusal), "{refusal:#}");
        w.finish().await;
    })
}

// ---------------------------------------------------------------------------
// Sealer and publisher
// ---------------------------------------------------------------------------

async fn log_entries(w: &World, kind: StatementType, election: Uuid, area: Uuid) -> usize {
    let query = LogQuery {
        filters: vec![
            Filter::Text(
                TextColumn::StatementKind,
                SqlCompOperators::Equal,
                kind.to_string(),
            ),
            Filter::Text(
                TextColumn::ElectionId,
                SqlCompOperators::Equal,
                election.to_string(),
            ),
            Filter::Text(
                TextColumn::AreaId,
                SqlCompOperators::Equal,
                area.to_string(),
            ),
        ],
        ..LogQuery::default()
    };
    w.database
        .store
        .query(w.board(), &query)
        .await
        .unwrap()
        .len()
}

/// The seal record of the box (election, area): where the seal row says it
/// is on the log, as the log stores it.
async fn seal_log_record(w: &World, election: Uuid) -> electoral_log::StoredRecord {
    let row = seal_row(w, election).await;
    let record = row.log_record.expect("the seal row names its record");
    let delivered = w
        .database
        .store
        .delivered(w.board(), &record.delivery_id)
        .await
        .unwrap()
        .expect("the seal record is on the board");
    assert_eq!(delivered.record.log_uid, record.log_uid);
    assert_eq!(delivered.record.leaf_index, record.leaf_index);
    delivered.record
}

#[test]
fn a_box_is_sealed_and_published_once() {
    event_database::run(async {
        let w = world(Options {
            record_policy: Some("public"),
            ..Default::default()
        })
        .await;
        let (election, area) = (w.election(), w.areas[0]);
        let replaced = cast(&w, election, area, "voter-alice", BallotStatus::Valid, "a1").await;
        let counted = cast(&w, election, area, "voter-alice", BallotStatus::Valid, "a2").await;
        let weighted = cast(&w, election, area, "voter-bob", BallotStatus::Valid, "b1").await;
        let not_eligible = cast(&w, election, area, "voter-carol", BallotStatus::Valid, "c1").await;
        let discarded = cast(
            &w,
            election,
            area,
            "voter-dave",
            BallotStatus::Rejected,
            "d1",
        )
        .await;
        close_election(&w, election, Some(&w.user)).await;
        let environment =
            TestEnvironment::new(&[("voter-alice", 1), ("voter-bob", 3), ("voter-zoe", 1)]);
        let seal = w.seals(election).await.remove(0);

        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        // A rerun is a no-op.
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::NotPending(BallotBoxSealStatus::Sealed)
        );
        let sealed = w.seals(election).await.remove(0);
        assert_eq!(sealed.status, BallotBoxSealStatus::Sealed);
        assert_eq!(sealed.ballots_in_box, Some(5));
        assert_eq!(sealed.ballots_counted, Some(2));

        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::Published
        );
        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::NotSealed(BallotBoxSealStatus::Published)
        );
        let published = w.seals(election).await.remove(0);
        assert_eq!(published.status, BallotBoxSealStatus::Published);
        assert_eq!(
            published.public_path.as_deref(),
            Some(
                format!(
                    "tenant-{}/event-{}/ballot-box-seals/{election}/{area}.json",
                    w.tenant, w.event
                )
                .as_str()
            )
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
            1
        );
        let record = seal_log_record(&w, election).await;
        assert_eq!(
            record.delivery_id,
            format!("ballot-box-seal:{}:{election}:{area}", w.event)
        );

        assert_eq!(
            published.public_document_id,
            Some(record_document_id(&seal.id))
        );
        assert_eq!(
            *environment.upload_policies.lock().unwrap(),
            vec![BallotBoxSealRecordPolicy::PUBLIC]
        );

        // The public record: verifies, holds only the allowed fields, and no
        // voter id.
        let uploads = environment.uploads.lock().unwrap().clone();
        assert_eq!(uploads.len(), 1);
        let (name, json) = &uploads[0];
        assert_eq!(name, &format!("ballot-box-seals/{election}/{area}.json"));
        let text = String::from_utf8(json.clone()).unwrap();
        assert!(!text.contains("voter-"), "a voter id leaked: {text}");
        let value: Value = serde_json::from_str(&text).unwrap();
        let keys: BTreeSet<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            BTreeSet::from([
                "format",
                "tenant_id",
                "election_event",
                "election",
                "area",
                "closed_at",
                "grace_deadline",
                "sealed_at",
                "eligible_voters",
                "ballots",
                "close_request",
                "seal_hash",
                "log_entry",
                "system_public_key",
                "message",
                "message_b64",
                "entries",
                "manifest",
            ])
        );
        for entry in value["entries"].as_array().unwrap() {
            let keys: BTreeSet<&str> = entry
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                keys,
                BTreeSet::from([
                    "ballot_hash",
                    "ballot_id",
                    "disposition",
                    "weight",
                    "channel"
                ])
            );
        }
        let record: SealRecord = serde_json::from_value(value).unwrap();
        let check = verify_record(&record, None).unwrap();
        // It names its entry on the log.
        let stored = seal_log_record(&w, election).await;
        assert_eq!(record.log_entry.id, stored.id);
        assert_eq!(record.log_entry.delivery_id, Some(stored.delivery_id));
        assert_eq!(record.log_entry.leaf_index, Some(stored.leaf_index));
        // Signed by the event's key as both sender and system.
        assert_eq!(
            check.message.sender.pk.to_b64_string().unwrap(),
            record.system_public_key
        );
        assert_eq!(check.manifest.eligible_voters, 3);
        let disposition = |ballot_id: &str| {
            let entry = check
                .manifest
                .entries
                .iter()
                .find(|entry| entry.ballot_id == ballot_id)
                .unwrap();
            (entry.disposition, entry.weight)
        };
        assert_eq!(disposition(&replaced), (SealDisposition::Replaced, 0));
        assert_eq!(disposition(&counted), (SealDisposition::Counted, 1));
        assert_eq!(disposition(&weighted), (SealDisposition::Counted, 3));
        assert_eq!(
            disposition(&not_eligible),
            (SealDisposition::NotEligible, 0)
        );
        assert_eq!(disposition(&discarded), (SealDisposition::Discarded, 0));
        w.finish().await;
    })
}

/// With the Seal Record Publication policy unset (restricted), the record is
/// a private event document: the seal keeps its fixed document id and has no
/// public path. A failed upload leaves the seal sealed; the retry publishes
/// it once, with the same document id.
#[test]
fn a_restricted_record_is_a_private_document() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        cast(&w, election, area, "voter-alice", BallotStatus::Valid, "a1").await;
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-alice", 1)]);
        environment.failing_uploads.store(1, Ordering::SeqCst);
        let seal = w.seals(election).await.remove(0);
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );

        // The upload fails: nothing is published, and the log entry stays one.
        assert!(publish_box(&mut client, &environment, &seal.id)
            .await
            .is_err());
        let sealed = w.seals(election).await.remove(0);
        assert_eq!(sealed.status, BallotBoxSealStatus::Sealed);
        assert_eq!(sealed.public_document_id, None);

        // The retry publishes it, privately.
        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::Published
        );
        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::NotSealed(BallotBoxSealStatus::Published)
        );
        let published = w.seals(election).await.remove(0);
        assert_eq!(published.status, BallotBoxSealStatus::Published);
        assert_eq!(published.public_path, None);
        assert_eq!(
            published.public_document_id,
            Some(record_document_id(&seal.id))
        );
        assert!(published.log_record.is_some());
        assert_eq!(
            *environment.upload_policies.lock().unwrap(),
            vec![BallotBoxSealRecordPolicy::RESTRICTED]
        );
        let uploads = environment.uploads.lock().unwrap().clone();
        assert_eq!(uploads.len(), 1);
        assert_eq!(
            uploads[0].0,
            format!("ballot-box-seals/{election}/{area}.json")
        );
        let record: SealRecord = serde_json::from_slice(&uploads[0].1).unwrap();
        verify_record(&record, None).unwrap();
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
            1
        );
        w.finish().await;
    })
}

#[test]
fn a_ballot_id_mismatch_fails_the_seal_and_logs_it_once() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        let (content, _) = ballot("tampered");
        let (_, other_id) = ballot("other");
        store_ballot(
            &w,
            election,
            area,
            "voter-a",
            BallotStatus::Valid,
            &content,
            &other_id,
            "ONLINE",
        )
        .await;
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = w.seals(election).await.remove(0);
        let mut client = w.client().await;
        let outcome = seal_box(&mut client, &environment, &seal.id).await.unwrap();
        assert!(
            matches!(&outcome, SealOutcome::Failed(reason) if reason.starts_with("a ballot does not match its Ballot ID")),
            "{outcome:?}"
        );
        let failed = w.seals(election).await.remove(0);
        assert_eq!(failed.status, BallotBoxSealStatus::Failed);
        // The failure is stored with its record on the log.
        let record = failed.log_record.expect("the failure's record");
        assert_eq!(
            record.delivery_id,
            format!("ballot-box-seal-failed:{}:{election}:{area}", w.event)
        );
        // The box stays locked.
        assert_eq!(
            late_cast(&w, election, area, "late").await,
            AcceptOutcome::BallotBoxSealed
        );
        // The failure entry is on the log once, however often the sealer runs.
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::NotPending(BallotBoxSealStatus::Failed)
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealFailed, election, area).await,
            1
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
            0
        );
        w.finish().await;
    })
}

/// Casts a valid ballot of `voter` into the box, on `channel`.
async fn cast_on(w: &World, election: Uuid, area: Uuid, voter: &str, channel: &str) {
    let (content, ballot_id) = ballot(voter);
    store_ballot(
        w,
        election,
        area,
        voter,
        BallotStatus::Valid,
        &content,
        &ballot_id,
        channel,
    )
    .await;
}

/// W6 B2: the ballots are evidence. A ballot of a channel that isn't
/// closed (here KIOSK, which the Post doesn't enable and whose status never
/// moved) leaves the seal pending and names the channel; once that channel
/// is closed, the box is sealed.
#[test]
fn a_ballot_of_a_channel_that_is_not_closed_holds_the_seal() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        cast_on(&w, election, area, "voter-online", "ONLINE").await;
        cast_on(&w, election, area, "voter-kiosk", "KIOSK").await;
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-online", 1), ("voter-kiosk", 1)]);
        let seal = seal_row(&w, election).await;
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::WaitingForChannel("KIOSK".to_string())
        );
        let row = seal_row(&w, election).await;
        assert_eq!(row.status, BallotBoxSealStatus::Pending);
        assert_eq!(
            row.waiting_reason.as_deref(),
            Some("channel_has_ballots:KIOSK")
        );
        assert!(row.last_attempt_at.is_some());
        // KIOSK closes: the box is sealed.
        let mut status = w.status(election).await;
        status.set_status_by_channel(VotingStatusChannel::KIOSK, VotingStatus::CLOSED);
        bypass(
            &w,
            "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
            &[&election, &serde_json::to_value(&status).unwrap()],
        )
        .await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        w.finish().await;
    })
}

/// W6 B (R10 B1): a channel that took ballots and was then unchecked in the
/// Post's enabled channels while open holds the seal; the close of the
/// enabled channels makes no seal. A Post-level Stop of that channel, still
/// not enabled (closing lets no ballot in), makes the seals, with its close
/// as the close time.
#[test]
fn a_channel_disabled_while_open_holds_the_seal_until_a_post_stop_of_it() {
    event_database::run(async {
        let w = world(Options {
            kiosk: true,
            ..Default::default()
        })
        .await;
        let election = w.election();
        change_election(&w, election, VotingStatus::OPEN, VotingStatusChannel::KIOSK)
            .await
            .unwrap();
        cast_on(&w, election, w.areas[0], "voter-kiosk", "KIOSK").await;
        // The Post stops enabling KIOSK while it is open.
        let unchecked = json!({"online": true, "kiosk": false, "early_voting": false});
        bypass(
            &w,
            "UPDATE sequent_backend.election SET voting_channels = $2 WHERE id = $1",
            &[&election, &unchecked],
        )
        .await;
        close_election(&w, election, Some(&w.user)).await;
        assert!(w.seals(election).await.is_empty(), "KIOSK holds the seal");
        // The Post's Stop of KIOSK, still not enabled.
        change_election(
            &w,
            election,
            VotingStatus::CLOSED,
            VotingStatusChannel::KIOSK,
        )
        .await
        .unwrap();
        let configured: Value = w
            .client()
            .await
            .query_one(
                "SELECT voting_channels FROM sequent_backend.election WHERE id = $1",
                &[&election],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(configured, unchecked, "KIOSK stays not enabled");
        let seals = w.seals(election).await;
        assert_eq!(seals.len(), 1);
        let kiosk_closed = w
            .status(election)
            .await
            .dates_by_channel(VotingStatusChannel::KIOSK)
            .last_stopped_at
            .unwrap();
        // The column keeps microseconds.
        assert_eq!(
            seals[0].closed_at.timestamp_micros(),
            kiosk_closed.timestamp_micros()
        );
        let environment = TestEnvironment::new(&[("voter-kiosk", 1)]);
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seals[0].id)
                .await
                .unwrap(),
            SealOutcome::Sealed
        );
        w.finish().await;
    })
}

/// R10 B1: the event enables ONLINE and KIOSK, a Post enables ONLINE only.
/// With Seal at close, an event-wide manual Start opens KIOSK only where a
/// Post enables it, so the Post's scheduled close of its channels seals its
/// box: KIOSK never opened there.
#[test]
fn an_event_start_opens_only_the_channels_a_post_enables() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            kiosk: true,
            online_open: false,
            ..Default::default()
        })
        .await;
        let [online_only, both] = [w.elections[0], w.elections[1]];
        bypass(
            &w,
            "UPDATE sequent_backend.election SET voting_channels = $2 WHERE id = $1",
            &[
                &online_only,
                &json!({"online": true, "kiosk": false, "early_voting": false}),
            ],
        )
        .await;
        // Nothing has started yet, at the event either.
        bypass(
            &w,
            "UPDATE sequent_backend.election_event SET status = $2 WHERE id = $1",
            &[
                &w.event,
                &serde_json::to_value(ElectionEventStatus::default()).unwrap(),
            ],
        )
        .await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        let (_, skipped) = update_event_voting_status(
            &tx,
            &w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &w.event.to_string(),
            &VotingStatus::OPEN,
            &Some(vec![
                VotingStatusChannel::ONLINE,
                VotingStatusChannel::KIOSK,
            ]),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        // Not a skipped Post: it opened the channel it offers.
        assert!(skipped.is_empty(), "{skipped:?}");
        let status = w.status(online_only).await;
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::OPEN
        );
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::NOT_STARTED
        );
        assert!(status
            .dates_by_channel(VotingStatusChannel::KIOSK)
            .first_started_at
            .is_none());
        assert_eq!(
            w.status(both)
                .await
                .status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::OPEN
        );
        cast_on(&w, online_only, w.areas[0], "voter-online", "ONLINE").await;
        // The Post's scheduled close of the event's channels.
        let tx = client.transaction().await.unwrap();
        update_scheduled_event_voting_status_for(
            &tx,
            &w.tenant.to_string(),
            &w.event.to_string(),
            &VotingStatus::CLOSED,
            &Some(vec![
                VotingStatusChannel::ONLINE,
                VotingStatusChannel::KIOSK,
            ]),
            &HashSet::new(),
            Some(&HashSet::from([online_only.to_string()])),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let seals = w.seals(online_only).await;
        assert_eq!(seals.len(), 1, "KIOSK never opened at the Post");
        let environment = TestEnvironment::new(&[("voter-online", 1)]);
        assert_eq!(
            seal_box(&mut client, &environment, &seals[0].id)
                .await
                .unwrap(),
            SealOutcome::Sealed
        );
        w.finish().await;
    })
}

/// R10 B1: on a policy-off event, an event-wide manual Start still sets the
/// channel on every Post, as before VOTE-FREEZE.
#[test]
fn policy_off_an_event_start_still_opens_every_post() {
    event_database::run(async {
        let w = world(Options {
            policy: "do-not-seal",
            kiosk: true,
            online_open: false,
            ..Default::default()
        })
        .await;
        let election = w.election();
        bypass(
            &w,
            "UPDATE sequent_backend.election SET voting_channels = $2 WHERE id = $1",
            &[
                &election,
                &json!({"online": true, "kiosk": false, "early_voting": false}),
            ],
        )
        .await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        update_event_voting_status(
            &tx,
            &w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &w.event.to_string(),
            &VotingStatus::OPEN,
            &Some(vec![VotingStatusChannel::KIOSK]),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            w.status(election)
                .await
                .status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::OPEN
        );
        w.finish().await;
    })
}

/// R11 S2: the initialization refusal of an event-wide Start counts only
/// the Posts the Start opens: an uninitialized Online-only Post doesn't
/// block a Kiosk Start, while an Online Start is still refused for it.
#[test]
fn an_event_start_is_not_refused_for_a_post_it_leaves_alone() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            kiosk: true,
            online_open: false,
            ..Default::default()
        })
        .await;
        let [online_only, both] = [w.elections[0], w.elections[1]];
        bypass(
            &w,
            "UPDATE sequent_backend.election
         SET voting_channels = $2,
             presentation = presentation || '{\"initialization_report_policy\": \"required\"}'
         WHERE id = $1",
            &[
                &online_only,
                &json!({"online": true, "kiosk": false, "early_voting": false}),
            ],
        )
        .await;
        bypass(
            &w,
            "UPDATE sequent_backend.election_event SET status = $2 WHERE id = $1",
            &[
                &w.event,
                &serde_json::to_value(ElectionEventStatus::default()).unwrap(),
            ],
        )
        .await;
        let start = |channel| {
            let w = &w;
            async move {
                let mut client = w.client().await;
                let tx = client.transaction().await.unwrap();
                let result = update_event_voting_status(
                    &tx,
                    &w.tenant.to_string(),
                    Some(&w.user),
                    Some(ADMIN),
                    &w.event.to_string(),
                    &VotingStatus::OPEN,
                    &Some(vec![channel]),
                )
                .await;
                if result.is_ok() {
                    tx.commit().await.unwrap();
                }
                result.map(|_| ())
            }
        };
        // The Post would open Online: its initialization refuses the Start.
        assert!(start(VotingStatusChannel::ONLINE).await.is_err());
        // It doesn't offer Kiosk, so the Kiosk Start leaves it alone.
        start(VotingStatusChannel::KIOSK).await.unwrap();
        assert_eq!(
            w.status(online_only)
                .await
                .status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::NOT_STARTED
        );
        assert_eq!(
            w.status(both)
                .await
                .status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::OPEN
        );
        w.finish().await;
    })
}

/// R11 N2: with Seal at close, an event-wide Online Start leaves a Post
/// that doesn't enable Online alone, Early Voting included (as a scheduled
/// Start does); a Post that enables both has its Early Voting closed, as
/// before.
#[test]
fn an_event_online_start_leaves_early_voting_open_at_a_post_without_online() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            early_voting: true,
            online_open: false,
            ..Default::default()
        })
        .await;
        let [early_only, both] = [w.elections[0], w.elections[1]];
        bypass(
            &w,
            "UPDATE sequent_backend.election SET voting_channels = $2 WHERE id = $1",
            &[
                &early_only,
                &json!({"online": false, "kiosk": false, "early_voting": true}),
            ],
        )
        .await;
        let mut early_open = ElectionStatus::default();
        early_open.set_status_by_channel(VotingStatusChannel::EARLY_VOTING, VotingStatus::OPEN);
        bypass(
            &w,
            "UPDATE sequent_backend.election SET status = $2 WHERE election_event_id = $1",
            &[&w.event, &serde_json::to_value(&early_open).unwrap()],
        )
        .await;
        let mut event_status = ElectionEventStatus::default();
        event_status.set_status_by_channel(VotingStatusChannel::EARLY_VOTING, VotingStatus::OPEN);
        bypass(
            &w,
            "UPDATE sequent_backend.election_event SET status = $2 WHERE id = $1",
            &[&w.event, &serde_json::to_value(&event_status).unwrap()],
        )
        .await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        update_event_voting_status(
            &tx,
            &w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &w.event.to_string(),
            &VotingStatus::OPEN,
            &Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let early = w.status(early_only).await;
        assert_eq!(
            early.status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::NOT_STARTED
        );
        assert_eq!(
            early.status_by_channel(VotingStatusChannel::EARLY_VOTING),
            VotingStatus::OPEN
        );
        let both = w.status(both).await;
        assert_eq!(
            both.status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::OPEN
        );
        assert_eq!(
            both.status_by_channel(VotingStatusChannel::EARLY_VOTING),
            VotingStatus::CLOSED
        );
        w.finish().await;
    })
}

/// R11 N4: the close hook fails on an election status that doesn't parse,
/// instead of reading it as nothing started (which makes no seals).
#[test]
fn the_close_hook_fails_on_a_status_that_does_not_parse() {
    event_database::run(async {
        let w = world(Options::default()).await;
        bypass(
            &w,
            "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
            &[&w.election(), &json!({"voting_status": 42})],
        )
        .await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        let event = windmill::postgres::election_event::get_election_event_by_id(
            &tx,
            &w.tenant.to_string(),
            &w.event.to_string(),
        )
        .await
        .unwrap();
        let error = on_close(
            &tx,
            &event,
            &[w.election().to_string()],
            CloseProvenance::Scheduled,
        )
        .await
        .unwrap_err();
        assert!(format!("{error:#}").contains("voting status"), "{error:#}");
        drop(tx);
        assert!(w.seals(w.election()).await.is_empty());
        w.finish().await;
    })
}

/// W6 B: with Seal at close, early voting that never opened still closes
/// directly once ONLINE has started, so it stops holding the seal.
#[test]
fn never_started_early_voting_closes_directly_after_online_started() {
    event_database::run(async {
        let w = world(Options {
            early_voting: true,
            ..Default::default()
        })
        .await;
        let election = w.election();
        cast_on(&w, election, w.areas[0], "voter-online", "ONLINE").await;
        close_election(&w, election, Some(&w.user)).await;
        assert!(
            w.seals(election).await.is_empty(),
            "EARLY_VOTING holds the seal"
        );
        change_election(
            &w,
            election,
            VotingStatus::CLOSED,
            VotingStatusChannel::EARLY_VOTING,
        )
        .await
        .unwrap();
        assert_eq!(w.seals(election).await.len(), 1);
        w.finish().await;
    })
}

/// W6 B2: a ballot whose channel is none of the four is an incident: the
/// seal fails, as a Ballot ID mismatch does.
#[test]
fn a_ballot_of_an_unknown_channel_fails_the_seal() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        cast_on(&w, election, area, "voter-paper", "PAPER").await;
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-paper", 1)]);
        let seal = seal_row(&w, election).await;
        let mut client = w.client().await;
        let outcome = seal_box(&mut client, &environment, &seal.id).await.unwrap();
        assert_eq!(
            outcome,
            SealOutcome::Failed("a ballot has the unknown voting channel \"PAPER\"".to_string())
        );
        let failed = seal_row(&w, election).await;
        assert_eq!(failed.status, BallotBoxSealStatus::Failed);
        w.finish().await;
    })
}

/// Removes the box's seal row past the permanence trigger (as restoring an
/// older backup of the Hasura database would) and makes a new pending one.
async fn restore_pending_row(w: &World, seal: &BallotBoxSeal) -> Uuid {
    bypass(
        w,
        "DELETE FROM sequent_backend.ballot_box_seal WHERE id = $1",
        &[&seal.id],
    )
    .await;
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    windmill::postgres::ballot_box_seal::insert_pending(
        &tx,
        &[windmill::postgres::ballot_box_seal::NewPendingSeal {
            tenant_id: w.tenant,
            election_event_id: w.event,
            election_id: seal.election_id,
            area_id: seal.area_id,
            closed_at: seal.closed_at,
            grace_deadline: seal.grace_deadline,
            close_request_id: None,
            closed_by: ClosedBy::Scheduled,
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    w.seals(seal.election_id).await.remove(0).id
}

/// A seal row that lost the seal (the Hasura database restored to before
/// it) takes the seal the event's database keeps: nothing is sealed twice.
#[test]
fn a_pending_row_takes_the_stored_seal_instead_of_a_second_seal() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        cast(&w, election, area, "voter-a", BallotStatus::Valid, "a").await;
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = w.seals(election).await.remove(0);
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        let sealed = get_for_box(&w, election, area).await;
        let restored = restore_pending_row(&w, &sealed).await;
        assert_eq!(
            seal_box(&mut client, &environment, &restored)
                .await
                .unwrap(),
            SealOutcome::Sealed
        );
        let taken = get_for_box(&w, election, area).await;
        assert_eq!(taken.status, BallotBoxSealStatus::Sealed);
        assert_eq!(taken.seal_hash, sealed.seal_hash);
        assert_eq!(taken.manifest, sealed.manifest);
        assert_eq!(taken.signed_message, sealed.signed_message);
        assert_eq!(taken.log_record, sealed.log_record);
        assert_eq!(
            publish_box(&mut client, &environment, &restored)
                .await
                .unwrap(),
            PublishOutcome::Published
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
            1
        );
        w.finish().await;
    })
}

/// A seal record of the box already on the board while the box has no seal
/// (its seal removed past the guards): the seal fails instead of a second
/// seal record.
#[test]
fn a_seal_already_on_the_board_fails_instead_of_a_second_seal() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        cast(&w, election, area, "voter-a", BallotStatus::Valid, "a").await;
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = w.seals(election).await.remove(0);
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        let sealed = seal_row(&w, election).await;
        let event_client = w.database.store.client().await.unwrap();
        event_client
            .batch_execute("SET session_replication_role = replica")
            .await
            .unwrap();
        event_client
            .execute(
                "DELETE FROM ballot_box_seal WHERE election_id = $1::text::uuid",
                &[&election.to_string()],
            )
            .await
            .unwrap();
        event_client
            .batch_execute("SET session_replication_role = DEFAULT")
            .await
            .unwrap();
        let restored = restore_pending_row(&w, &sealed).await;
        assert_eq!(
            seal_box(&mut client, &environment, &restored)
                .await
                .unwrap(),
            SealOutcome::Failed(ALREADY_ON_BOARD_REASON.to_string())
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
            1
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealFailed, election, area).await,
            1
        );
        w.finish().await;
    })
}

/// The full seal row of the box (election, area).
async fn get_for_box(w: &World, election: Uuid, area: Uuid) -> BallotBoxSeal {
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    windmill::postgres::ballot_box_seal::get_for_box(&tx, &w.tenant, &w.event, &election, &area)
        .await
        .unwrap()
        .unwrap()
}

/// The sealer asks the sequencer for the box's accepted ballots that aren't
/// on the log yet and waits: a seal is ordered after every ballot it holds.
#[test]
fn ballots_not_on_the_log_yet_wait_for_the_sequencer() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        let (content, ballot_id) = ballot("a");
        let seq = match w
            .database
            .try_cast(
                &Vote {
                    election,
                    area,
                    voter: "voter-a",
                    content: &content,
                    ballot_id: &ballot_id,
                    channel: "ONLINE",
                },
                BallotStatus::Valid,
            )
            .await
        {
            AcceptOutcome::Accepted { seq, .. } => seq,
            other => panic!("{other:?}"),
        };
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = seal_row(&w, election).await;
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::WaitingForSequencer(1)
        );
        assert_eq!(environment.sequencer_requests.load(Ordering::SeqCst), 1);
        let row = seal_row(&w, election).await;
        assert_eq!(row.status, BallotBoxSealStatus::Pending);
        assert_eq!(row.waiting_reason.as_deref(), Some("sequencer:1"));
        // The sequencer appends it; the seal follows its record.
        w.database.sequence(&w.key).await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        let cast_vote = w
            .database
            .store
            .delivered(w.board(), &format!("ballot-box:{}:{seq}", w.event))
            .await
            .unwrap()
            .expect("the ballot's record");
        let sealed = seal_log_record(&w, election).await;
        assert!(cast_vote.record.leaf_index < sealed.leaf_index);
        w.finish().await;
    })
}

/// A ballot the sequencer took off its queue without its record on the
/// board is an incident: the seal fails.
#[test]
fn a_ballot_without_its_cast_vote_record_fails_the_seal() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        let (content, ballot_id) = ballot("a");
        let seq = match w
            .database
            .try_cast(
                &Vote {
                    election,
                    area,
                    voter: "voter-a",
                    content: &content,
                    ballot_id: &ballot_id,
                    channel: "ONLINE",
                },
                BallotStatus::Valid,
            )
            .await
        {
            AcceptOutcome::Accepted { seq, .. } => seq,
            other => panic!("{other:?}"),
        };
        w.database
            .store
            .remove_pending(&w.event.to_string(), &[seq])
            .await
            .unwrap();
        close_election(&w, election, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = seal_row(&w, election).await;
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Failed(unrecorded_reason(1))
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealFailed, election, area).await,
            1
        );
        w.finish().await;
    })
}

/// A refused delete leaves the boards in place: the seal check runs before
/// the task deletes the event's boards (B3) or anything else.
#[test]
fn a_refused_event_delete_touches_no_board() {
    event_database::run(async {
        let w = world(Options::default()).await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        let error = windmill::tasks::delete_election_event::delete_election_event_in(
            &tx,
            &w.tenant.to_string(),
            &w.event.to_string(),
            "realm",
        )
        .await
        .unwrap_err();
        // Exactly the refusal: no board deletion ran first (it would have
        // failed or prefixed its own error).
        assert_eq!(error.to_string(), SEALED_EVENT_DELETE_REFUSAL);
        drop(tx);
        // The event's electoral-log database and board are still there.
        assert!(get_event_databases()
            .await
            .unwrap()
            .has_event(&w.event.to_string())
            .await
            .unwrap());
        assert!(w.database.store.has_board(w.board()).await.unwrap());
        w.finish().await;
    })
}

/// A scheduled event-wide start doesn't open a channel of a Post that has
/// seals (one that never started counts as finished), and says why.
#[test]
fn a_scheduled_event_opening_leaves_sealed_posts_closed() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            kiosk: true,
            ..Default::default()
        })
        .await;
        let [sealed, open] = [w.elections[0], w.elections[1]];
        cast(&w, sealed, w.areas[0], "voter-a", BallotStatus::Valid, "a").await;
        // ONLINE closed, KIOSK never started; seals made earlier (e.g. under the
        // earlier rule, or before KIOSK was enabled).
        close_election(&w, sealed, Some(&w.user)).await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        windmill::postgres::ballot_box_seal::insert_pending(
            &tx,
            &[windmill::postgres::ballot_box_seal::NewPendingSeal {
                tenant_id: w.tenant,
                election_event_id: w.event,
                election_id: sealed,
                area_id: w.areas[0],
                closed_at: Utc::now(),
                grace_deadline: Utc::now(),
                close_request_id: None,
                closed_by: ClosedBy::Scheduled,
            }],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(w.seals(sealed).await.len(), 1);
        let tx = client.transaction().await.unwrap();
        let skipped = update_scheduled_event_voting_status_for(
            &tx,
            &w.tenant.to_string(),
            &w.event.to_string(),
            &VotingStatus::OPEN,
            &Some(vec![VotingStatusChannel::KIOSK]),
            &HashSet::new(),
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            skipped.get(&sealed.to_string()).map(|(_, refusal)| refusal),
            Some(&TransitionRefusal::BallotBoxSealPolicy)
        );
        assert!(!skipped.contains_key(&open.to_string()));
        assert_eq!(
            w.status(sealed)
                .await
                .status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::NOT_STARTED
        );
        assert_eq!(
            w.status(open)
                .await
                .status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::OPEN
        );
        w.finish().await;
    })
}

/// The event's database names the sealed boxes that hold a voter's ballots,
/// and refuses a discard of the voter's ballots as a whole while one of them
/// is in a sealed box: the ballots stay as they were sealed.
#[test]
fn a_discard_of_ballots_in_a_sealed_box_is_refused_whole() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            ..Default::default()
        })
        .await;
        let [sealed, open] = [w.elections[0], w.elections[1]];
        cast(&w, sealed, w.areas[0], "voter-a", BallotStatus::Valid, "a1").await;
        cast(&w, open, w.areas[1], "voter-a", BallotStatus::Valid, "a2").await;
        close_election(&w, sealed, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = w.seals(sealed).await.remove(0);
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        let event = w.event.to_string();
        assert_eq!(
            w.database
                .store
                .voter_sealed_boxes(&event, "voter-a")
                .await
                .unwrap(),
            vec![(sealed.to_string(), w.areas[0].to_string())]
        );
        let refusal = w
            .database
            .store
            .reject_voter_ballots(&event, "voter-a")
            .await
            .unwrap_err();
        assert!(is_sealed_refusal(&refusal) && is_ballot_box_sealed(&refusal));
        let statuses: Vec<String> = w
            .database
            .store
            .client()
            .await
            .unwrap()
            .query(
                "SELECT status FROM ballot_box_ballot
             WHERE election_event_id = $1::text::uuid ORDER BY seq",
                &[&event],
            )
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get(0))
            .collect();
        assert_eq!(statuses, vec!["valid", "valid"]);
        w.finish().await;
    })
}

// ---------------------------------------------------------------------------
// R6 hardening
// ---------------------------------------------------------------------------

/// Runs `sql` past the triggers (as a superuser restoring data would), in
/// a transaction of its own.
async fn bypass(w: &World, sql: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) {
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL session_replication_role = replica")
        .await
        .unwrap();
    tx.execute(sql, params).await.unwrap();
    tx.commit().await.unwrap();
}

/// The SQLSTATE and message of a refused statement.
async fn refused_sql(
    w: &World,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> (String, String) {
    let error = w.client().await.execute(sql, params).await.unwrap_err();
    let db = error.as_db_error().expect("a database error");
    (db.code().code().to_owned(), db.message().to_owned())
}

async fn seal_row(w: &World, election: Uuid) -> BallotBoxSeal {
    w.seals(election).await.remove(0)
}

/// D1: on a policy-off event a never-started channel still can't be
/// stopped (the platform's table is unchanged).
#[test]
fn policy_off_still_refuses_stopping_a_never_started_channel() {
    event_database::run(async {
        let w = world(Options {
            policy: "do-not-seal",
            kiosk: true,
            ..Default::default()
        })
        .await;
        let error = change_election(
            &w,
            w.election(),
            VotingStatus::CLOSED,
            VotingStatusChannel::KIOSK,
        )
        .await
        .unwrap_err();
        assert!(
            error.to_string().starts_with("Unexpected next status"),
            "{error}"
        );
        w.finish().await;
    })
}

/// D1 (R7 B1): a scheduled close closes a never-started channel only when
/// the schedule names it and the Post actually ran; a Post that never
/// opened is left alone.
#[test]
fn a_scheduled_close_closes_only_named_never_started_channels_of_posts_that_ran() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            kiosk: true,
            ..Default::default()
        })
        .await;
        let [named, unnamed] = [w.elections[0], w.elections[1]];
        for election in [named, unnamed] {
            cast(
                &w,
                election,
                w.areas[0],
                &format!("voter-{election}"),
                BallotStatus::Valid,
                &election.to_string(),
            )
            .await;
        }
        let scheduled_close = |channels: Vec<VotingStatusChannel>, only: Uuid| {
            let w = &w;
            async move {
                let mut client = w.client().await;
                let tx = client.transaction().await.unwrap();
                update_scheduled_event_voting_status_for(
                    &tx,
                    &w.tenant.to_string(),
                    &w.event.to_string(),
                    &VotingStatus::CLOSED,
                    &Some(channels),
                    &HashSet::new(),
                    Some(&HashSet::from([only.to_string()])),
                )
                .await
                .unwrap();
                tx.commit().await.unwrap();
            }
        };
        // The schedule names ONLINE and KIOSK: the Post ran, so KIOSK closes.
        scheduled_close(
            vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK],
            named,
        )
        .await;
        let status = w.status(named).await;
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::CLOSED
        );
        assert_eq!(w.seals(named).await.len(), 1);
        // The schedule names ONLINE only: KIOSK stays never started and holds.
        scheduled_close(vec![VotingStatusChannel::ONLINE], unnamed).await;
        let status = w.status(unnamed).await;
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::CLOSED
        );
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::NOT_STARTED
        );
        assert!(w.seals(unnamed).await.is_empty());
        w.finish().await;

        // A Post that never opened: a scheduled END leaves it untouched.
        let w = world(Options {
            kiosk: true,
            online_open: false,
            ..Default::default()
        })
        .await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        update_scheduled_event_voting_status(
            &tx,
            &w.tenant.to_string(),
            None,
            None,
            &w.event.to_string(),
            &VotingStatus::CLOSED,
            &Some(vec![
                VotingStatusChannel::ONLINE,
                VotingStatusChannel::KIOSK,
            ]),
            &HashSet::new(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let status = w.status(w.election()).await;
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::NOT_STARTED
        );
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::NOT_STARTED
        );
        assert!(w.seals(w.election()).await.is_empty());
        w.finish().await;
    })
}

/// R6b-B1 / R7 B1: early voting runs and its scheduled end closes it; ONLINE
/// stays never started, nothing is due, and the Post is sealed only once
/// ONLINE has opened and closed.
#[test]
fn early_voting_end_leaves_online_to_open_and_seal_later() {
    event_database::run(async {
        let w = world(Options {
            early_voting: true,
            online_open: false,
            ..Default::default()
        })
        .await;
        change_election(
            &w,
            w.election(),
            VotingStatus::OPEN,
            VotingStatusChannel::EARLY_VOTING,
        )
        .await
        .unwrap();
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-early",
            BallotStatus::Valid,
            "e",
        )
        .await;
        // The scheduler's filter for the early-voting end: it names only
        // EARLY_VOTING; ONLINE, never started, isn't among its channels.
        let status = w.status(w.election()).await;
        assert!(scheduled_change_applies(
            &status,
            &serde_json::from_value(json!({"online": true, "early_voting": true})).unwrap(),
            VotingStatusChannel::EARLY_VOTING,
            &VotingStatus::CLOSED,
            sequent_core::ballot::BallotBoxSealPolicy::SEAL_AT_CLOSE
        ));
        // The scheduled end of early voting (no user, its own channel).
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        update_election_status(
            w.tenant.to_string(),
            None,
            None,
            &tx,
            &w.event.to_string(),
            &w.election().to_string(),
            &VotingStatus::CLOSED,
            &Some(vec![VotingStatusChannel::EARLY_VOTING]),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let status = w.status(w.election()).await;
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::EARLY_VOTING),
            VotingStatus::CLOSED
        );
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::ONLINE),
            VotingStatus::NOT_STARTED
        );
        assert!(
            w.seals(w.election()).await.is_empty(),
            "ONLINE holds the seal"
        );
        // Election day: ONLINE opens and closes; then the box is due.
        change_election(
            &w,
            w.election(),
            VotingStatus::OPEN,
            VotingStatusChannel::ONLINE,
        )
        .await
        .unwrap();
        change_election(
            &w,
            w.election(),
            VotingStatus::CLOSED,
            VotingStatusChannel::ONLINE,
        )
        .await
        .unwrap();
        assert_eq!(w.seals(w.election()).await.len(), 1);
        w.finish().await;
    })
}

/// R7 B2: after voting opened, writes that keep the effective policies
/// (a missing key, a JSON null or the default spelled out) pass; a real
/// change is refused.
#[test]
fn presentation_writes_keep_working_after_voting_opened() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let client = w.client().await;
        // A server read-modify-write of the typed presentation (as the lockdown
        // and enrollment tasks do): every unset field written as null.
        let presentation: Value = client
            .query_one(
                "SELECT presentation FROM sequent_backend.election_event WHERE id = $1",
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        let mut typed: sequent_core::ballot::ElectionEventPresentation =
            serde_json::from_value(presentation).unwrap();
        typed.logo_url = Some("logo.png".into());
        let rewritten = serde_json::to_value(&typed).unwrap();
        assert!(rewritten
            .get("weighted_voting_policy")
            .is_some_and(Value::is_null));
        client
            .execute(
                "UPDATE sequent_backend.election_event SET presentation = $2 WHERE id = $1",
                &[&w.event, &rewritten],
            )
            .await
            .unwrap();
        // The form's defaults spelled out.
        client
            .execute(
                "UPDATE sequent_backend.election_event
             SET presentation = presentation || '{\"contest_encryption_policy\": \"single-contest\",
                 \"delegated_voting_policy\": \"disabled\",
                 \"weighted_voting_policy\": \"disabled-weighted-voting\"}'
             WHERE id = $1",
                &[&w.event],
            )
            .await
            .unwrap();
        // A real change.
        let (code, message) = refused_sql(
            &w,
            "UPDATE sequent_backend.election_event
         SET presentation = presentation || '{\"delegated_voting_policy\": \"enabled\"}'
         WHERE id = $1",
            &[&w.event],
        )
        .await;
        assert_eq!(code, "42501");
        assert!(message.contains("delegated_voting_policy"), "{message}");
        w.finish().await;
    })
}

/// B6b-S3: on a seal-at-close event a CLOSED channel stays CLOSED, also for
/// a trusted write (a stale read-modify-write).
#[test]
fn a_closed_channel_stays_closed_in_the_database() {
    event_database::run(async {
        let w = world(Options::default()).await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        trusted_write(&tx).await.unwrap();
        let mut status = ElectionStatus::default();
        status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        let error = tx
            .execute(
                "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
                &[&w.election(), &serde_json::to_value(&status).unwrap()],
            )
            .await
            .unwrap_err();
        let db = error.as_db_error().unwrap();
        assert_eq!(db.code().code(), "42501");
        assert_eq!(db.message(), "ballot_box_seal_closed_is_final");
        w.finish().await;
    })
}

/// B6b-S1 and D3: once voting opened, a seal-at-close event's ballot
/// reading and counting policies and its bulletin board can't change.
#[test]
fn seal_relevant_event_settings_are_locked_once_voting_opened() {
    event_database::run(async {
        let w = world(Options::default()).await;
        for (sql, what) in [
        (
            "UPDATE sequent_backend.election_event
             SET presentation = presentation || '{\"contest_encryption_policy\": \"multiple-contests\"}'
             WHERE id = $1",
            "contest_encryption_policy",
        ),
        (
            "UPDATE sequent_backend.election_event
             SET presentation = presentation || '{\"weighted_voting_policy\": \"voters-weighted-voting\"}'
             WHERE id = $1",
            "weighted_voting_policy",
        ),
        (
            "UPDATE sequent_backend.election_event
             SET bulletin_board_reference = '{\"id\": 2, \"database_name\": \"other\", \"is_archived\": false}'
             WHERE id = $1",
            "bulletin board",
        ),
    ] {
        let (code, message) = refused_sql(&w, sql, &[&w.event]).await;
        assert_eq!(code, "42501");
        assert!(message.contains(what), "{message}");
    }
        // Other settings still change.
        w.client()
            .await
            .execute(
                "UPDATE sequent_backend.election_event
             SET presentation = presentation || '{\"logo_url\": \"x\"}' WHERE id = $1",
                &[&w.event],
            )
            .await
            .unwrap();
        w.finish().await;
    })
}

/// B6b-S1 and D5: a configuration that doesn't parse is retried, never an
/// incident; each attempt records why the box isn't sealed.
#[test]
fn attempts_record_why_and_configuration_errors_are_retried() {
    event_database::run(async {
        let w = world(Options {
            grace_secs: Some(QUARTER_HOUR_SECS),
            ..Default::default()
        })
        .await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = seal_row(&w, w.election()).await;
        let mut client = w.client().await;

        // Before the deadline.
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::NotDue
        );
        let row = seal_row(&w, w.election()).await;
        assert_eq!(row.waiting_reason.as_deref(), Some("deadline"));
        assert!(row.last_attempt_at.is_some());

        // Past it, with a Datafix vote in progress.
        bypass(
        &w,
        "UPDATE sequent_backend.ballot_box_seal SET grace_deadline = now() - interval '1 second' WHERE id = $1",
        &[&seal.id],
    )
    .await;
        let (content, ballot_id) = ballot("in-progress");
        store_ballot(
            &w,
            w.election(),
            w.areas[0],
            "voter-b",
            BallotStatus::Pending,
            &content,
            &ballot_id,
            "ONLINE",
        )
        .await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::WaitingForVotes(1)
        );
        assert_eq!(
            seal_row(&w, w.election()).await.waiting_reason.as_deref(),
            Some("datafix_votes:1")
        );
        assert_eq!(
            w.database
                .store
                .reject_voter_ballots(&w.event.to_string(), "voter-b")
                .await
                .unwrap(),
            1
        );

        // A write of the box is in flight: it holds the box's seal lock.
        {
            let mut holder = w.database.store.client().await.unwrap();
            let held = holder.transaction().await.unwrap();
            held.execute(
                "SELECT pg_advisory_xact_lock_shared(hashtextextended(ballot_box_seal_lock_key(
                 $1::text::uuid, $2::text::uuid, $3::text::uuid), 0))",
                &[
                    &w.event.to_string(),
                    &w.election().to_string(),
                    &w.areas[0].to_string(),
                ],
            )
            .await
            .unwrap();
            assert_eq!(
                seal_box(&mut client, &environment, &seal.id).await.unwrap(),
                SealOutcome::Busy
            );
            assert_eq!(
                seal_row(&w, w.election()).await.waiting_reason.as_deref(),
                Some("busy")
            );
        }

        // A voting status that doesn't parse: an error, the seal stays pending.
        bypass(
        &w,
        "UPDATE sequent_backend.election SET status = '{\"voting_status\": \"SHUT\"}' WHERE id = $1",
        &[&w.election()],
    )
    .await;
        assert!(seal_box(&mut client, &environment, &seal.id).await.is_err());
        let row = seal_row(&w, w.election()).await;
        assert_eq!(row.status, BallotBoxSealStatus::Pending);
        assert!(
            row.waiting_reason
                .as_deref()
                .unwrap_or_default()
                .starts_with("error:"),
            "{:?}",
            row.waiting_reason
        );
        w.finish().await;
    })
}

/// B6c-S1: a seal row another transaction holds is skipped, not waited
/// for, by the sealer and the publisher.
#[test]
fn a_held_seal_row_is_busy_not_waited_for() {
    event_database::run(async {
        let w = world(Options::default()).await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = seal_row(&w, w.election()).await;
        let mut holder = w.client().await;
        let held = holder.transaction().await.unwrap();
        held.execute(
            "SELECT 1 FROM sequent_backend.ballot_box_seal WHERE id = $1 FOR UPDATE",
            &[&seal.id],
        )
        .await
        .unwrap();
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Busy
        );
        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::Busy
        );
        drop(held);
        w.finish().await;
    })
}

/// B6b-S5: a published ballot style that can't be read doesn't stop the
/// close; its area still gets a seal.
#[test]
fn an_unreadable_ballot_style_does_not_stop_the_close() {
    event_database::run(async {
        let w = world(Options::default()).await;
        let publication = Uuid::new_v4();
        bypass(
            &w,
            "INSERT INTO sequent_backend.ballot_publication
             (id, tenant_id, election_event_id, is_generated)
         VALUES ($1, $2, $3, true)",
            &[&publication, &w.tenant, &w.event],
        )
        .await;
        bypass(
            &w,
            "INSERT INTO sequent_backend.ballot_style
             (id, tenant_id, election_event_id, election_id, area_id, ballot_eml,
              ballot_publication_id)
         VALUES ($1, $2, $3, $4, $5, 'not a ballot style', $6)",
            &[
                &Uuid::new_v4(),
                &w.tenant,
                &w.event,
                &w.election(),
                &w.areas[1],
                &publication,
            ],
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let seals = w.seals(w.election()).await;
        assert_eq!(seals.len(), 1);
        assert_eq!(seals[0].area_id, w.areas[1]);
        w.finish().await;
    })
}

/// B6c-S3: a failed seal's entry is on the log with the failure; the
/// dispatcher doesn't list it. E17: the record keeps the names the seal
/// signed, also after a rename.
#[test]
fn the_failure_entry_is_recorded_and_the_record_keeps_the_signed_names() {
    event_database::run(async {
        let w = world(Options {
            elections: 2,
            ..Default::default()
        })
        .await;
        let [failing, renamed] = [w.elections[0], w.elections[1]];
        let (content, _) = ballot("tampered");
        let (_, other_id) = ballot("other");
        store_ballot(
            &w,
            failing,
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            &content,
            &other_id,
            "ONLINE",
        )
        .await;
        cast(&w, renamed, w.areas[1], "voter-b", BallotStatus::Valid, "b").await;
        close_election(&w, failing, Some(&w.user)).await;
        close_election(&w, renamed, Some(&w.user)).await;
        let environment = TestEnvironment::new(&[("voter-a", 1), ("voter-b", 1)]);
        let mut client = w.client().await;

        let failed = seal_row(&w, failing).await;
        assert!(matches!(
            seal_box(&mut client, &environment, &failed.id)
                .await
                .unwrap(),
            SealOutcome::Failed(_)
        ));
        // The failure entry is on the log with the failure: the dispatcher has
        // nothing more to do for it.
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealFailed, failing, w.areas[0]).await,
            1
        );
        let tx = client.transaction().await.unwrap();
        let work = windmill::postgres::ballot_box_seal::list_open_work_ids(&tx, Utc::now(), 1000)
            .await
            .unwrap();
        drop(tx);
        assert!(!work.contains(&failed.id));
        // The incident keeps the names (R7 S3).
        let failed_row = seal_row(&w, failing).await;
        assert_eq!(failed_row.area_name.as_deref(), Some("Area"));
        assert_eq!(
            failed_row.election_name.as_deref(),
            Some(failing.to_string().as_str())
        );

        // Sealed under the area's name, renamed, then published.
        let seal = seal_row(&w, renamed).await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        assert_eq!(
            seal_row(&w, renamed).await.area_name.as_deref(),
            Some("Area")
        );
        client
            .execute(
                "UPDATE sequent_backend.area SET name = 'Renamed' WHERE id = $1",
                &[&w.areas[1]],
            )
            .await
            .unwrap();
        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::Published
        );
        let (_, json) = environment.uploads.lock().unwrap().last().cloned().unwrap();
        let record: SealRecord = serde_json::from_slice(&json).unwrap();
        assert_eq!(record.area.name, "Area");
        let check = verify_record(&record, None).unwrap();
        // The signed description names the area as the record does.
        assert!(
            check
                .message
                .statement
                .head
                .description
                .contains(&record.area.name),
            "{}",
            check.message.statement.head.description
        );
        assert!(check
            .message
            .statement
            .head
            .description
            .contains(&record.election.name));
        // The record's key is the one that signed the message.
        assert_eq!(
            windmill::services::ballot_box_seal::publish::signing_key_of(&check.message)
                .unwrap()
                .to_b64_string()
                .unwrap(),
            record.system_public_key
        );
        w.finish().await;
    })
}

/// D4: the close's schedule finds the new, untried pending seals.
#[test]
fn a_close_schedules_its_new_seals() {
    event_database::run(async {
        let w = world(Options {
            grace_secs: Some(QUARTER_HOUR_SECS),
            ..Default::default()
        })
        .await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let seal = seal_row(&w, w.election()).await;
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        let new = windmill::postgres::ballot_box_seal::list_new_pending(
            &tx,
            Utc::now() - Duration::minutes(10),
        )
        .await
        .unwrap();
        drop(tx);
        assert!(new.contains(&(seal.id, seal.grace_deadline)));
        // Once tried, the beat takes over.
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        seal_box(&mut client, &environment, &seal.id).await.unwrap();
        let tx = client.transaction().await.unwrap();
        let new = windmill::postgres::ballot_box_seal::list_new_pending(
            &tx,
            Utc::now() - Duration::minutes(10),
        )
        .await
        .unwrap();
        assert!(!new.iter().any(|(id, _)| *id == seal.id));
        w.finish().await;
    })
}

/// E18: entries someone else signed don't count, however many come first:
/// the box is sealed, and its record names its genuine entry.
#[test]
fn junk_seal_entries_neither_block_nor_hide_the_genuine_one() {
    event_database::run(async {
        use electoral_log::messages::message::{Message, SigningData};
        use electoral_log::seal::{build, BallotBoxSealManifest, SEAL_FORMAT_V1};
        let w = world(Options::default()).await;
        let (election, area) = (w.election(), w.areas[0]);
        cast(&w, election, area, "voter-a", BallotStatus::Valid, "a").await;
        close_election(&w, election, Some(&w.user)).await;
        let intruder = SigningKey::generate().unwrap();
        let junk = SigningData::new(intruder.clone(), "", intruder);
        // More than one page of board reads (100).
        for index in 0..105u64 {
            let built = build(BallotBoxSealManifest {
                format: SEAL_FORMAT_V1.to_string(),
                tenant_id: w.tenant.to_string(),
                election_event_id: w.event.to_string(),
                election_id: election.to_string(),
                area_id: area.to_string(),
                closed_at: index,
                grace_deadline: index,
                sealed_at: index,
                close_request_id: None,
                eligible_voters: 0,
                entries: vec![],
            })
            .unwrap();
            let message =
                Message::ballot_box_sealed_message(&built.manifest, built.hash, "x", "y", &junk)
                    .unwrap();
            w.database
                .store
                .append(
                    w.board(),
                    &mut std::iter::once(Ok(LogEntry {
                        delivery_id: format!("junk:{election}:{index}"),
                        message: (&message).try_into().unwrap(),
                    })),
                )
                .await
                .unwrap();
        }
        let environment = TestEnvironment::new(&[("voter-a", 1)]);
        let seal = seal_row(&w, election).await;
        let mut client = w.client().await;
        assert_eq!(
            seal_box(&mut client, &environment, &seal.id).await.unwrap(),
            SealOutcome::Sealed
        );
        assert_eq!(
            publish_box(&mut client, &environment, &seal.id)
                .await
                .unwrap(),
            PublishOutcome::Published
        );
        assert_eq!(
            log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
            106
        );
        let record = seal_log_record(&w, election).await;
        let (_, json) = environment.uploads.lock().unwrap().last().cloned().unwrap();
        let published: SealRecord = serde_json::from_slice(&json).unwrap();
        assert_eq!(published.log_entry.id, record.id);
        assert_eq!(published.log_entry.leaf_index, Some(record.leaf_index));
        w.finish().await;
    })
}

/// R7 S7: the attempt bookkeeping doesn't wait for a row another run holds.
#[test]
fn recording_an_attempt_does_not_wait_for_a_held_row() {
    event_database::run(async {
        let w = world(Options::default()).await;
        cast(
            &w,
            w.election(),
            w.areas[0],
            "voter-a",
            BallotStatus::Valid,
            "a",
        )
        .await;
        close_election(&w, w.election(), Some(&w.user)).await;
        let seal = seal_row(&w, w.election()).await;
        let mut holder = w.client().await;
        let held = holder.transaction().await.unwrap();
        held.execute(
            "SELECT 1 FROM sequent_backend.ballot_box_seal WHERE id = $1 FOR UPDATE",
            &[&seal.id],
        )
        .await
        .unwrap();
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        tx.batch_execute("SET LOCAL lock_timeout = '1s'")
            .await
            .unwrap();
        windmill::postgres::ballot_box_seal::record_attempt(
            &tx,
            &seal.id,
            &windmill::postgres::ballot_box_seal::WaitingReason::Busy,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        drop(held);
        w.finish().await;
    })
}

/// R8 B1: a KIOSK-only Post whose (not enabled) ONLINE an event-wide Start
/// marked open never opened: a scheduled END leaves it alone, no seal.
#[test]
fn a_scheduled_close_leaves_a_kiosk_only_post_that_never_opened() {
    event_database::run(async {
        let w = world(Options {
            kiosk: true,
            online_enabled: false,
            ..Default::default()
        })
        .await;
        // The fixture's ONLINE OPEN stands for the event-wide Start.
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        update_scheduled_event_voting_status(
            &tx,
            &w.tenant.to_string(),
            None,
            None,
            &w.event.to_string(),
            &VotingStatus::CLOSED,
            &Some(vec![
                VotingStatusChannel::ONLINE,
                VotingStatusChannel::KIOSK,
            ]),
            &HashSet::new(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let status = w.status(w.election()).await;
        assert_eq!(
            status.status_by_channel(VotingStatusChannel::KIOSK),
            VotingStatus::NOT_STARTED
        );
        assert!(w.seals(w.election()).await.is_empty());
        w.finish().await;
    })
}

/// R7 S2 / R8 S5: a status change that waited for another one reads the
/// event status after it, so it doesn't write a stale copy back.
#[test]
fn a_waiting_status_change_does_not_write_back_a_stale_event_status() {
    event_database::run(async {
        let w = world(Options {
            policy: "do-not-seal",
            elections: 2,
            ..Default::default()
        })
        .await;
        // A: the event-wide pause, held open.
        let mut first = w.client().await;
        let paused = first.transaction().await.unwrap();
        update_event_voting_status(
            &paused,
            &w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &w.event.to_string(),
            &VotingStatus::PAUSED,
            &Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await
        .unwrap();
        // B: a Post's close, which waits for A's locks.
        let pool = w.pool.clone();
        let (tenant, event, election, user) = (w.tenant, w.event, w.elections[1], w.user.clone());
        let closing = tokio::spawn(async move {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            update_election_status(
                tenant.to_string(),
                Some(&user),
                Some(ADMIN),
                &tx,
                &event.to_string(),
                &election.to_string(),
                &VotingStatus::CLOSED,
                &Some(vec![VotingStatusChannel::ONLINE]),
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(!closing.is_finished(), "B waits for A's locks");
        paused.commit().await.unwrap();
        closing.await.unwrap();
        let event_status: Value = w
            .client()
            .await
            .query_one(
                "SELECT status FROM sequent_backend.election_event WHERE id = $1",
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        // A's pause stands: B didn't write back the OPEN it would have read.
        assert_eq!(event_status["voting_status"], json!("PAUSED"));
        w.finish().await;
    })
}

/// R8 S1: a status writer takes no signing lock, so it doesn't wait for (or
/// deadlock with) a claim holding it.
#[test]
fn a_status_change_does_not_wait_for_the_signing_lock() {
    event_database::run(async {
        let w = world(Options {
            policy: "do-not-seal",
            ..Default::default()
        })
        .await;
        let mut holder = w.client().await;
        let held = holder.transaction().await.unwrap();
        windmill::postgres::signing::lock_signing_event(&held, w.tenant, w.event)
            .await
            .unwrap();
        let mut client = w.client().await;
        let tx = client.transaction().await.unwrap();
        tx.batch_execute("SET LOCAL lock_timeout = '1s'")
            .await
            .unwrap();
        update_election_status(
            w.tenant.to_string(),
            Some(&w.user),
            Some(ADMIN),
            &tx,
            &w.event.to_string(),
            &w.election().to_string(),
            &VotingStatus::PAUSED,
            &Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        drop(held);
        w.finish().await;
    })
}
