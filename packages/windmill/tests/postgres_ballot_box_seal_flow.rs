// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! VOTE-FREEZE end to end on a private database of the stack's Postgres and
//! a private board of the stack's immudb: every close path creates the
//! seals with its provenance, closed voting never reopens, sealed elections
//! and events can't be deleted, a cast after the seal deadline or into a
//! sealed box gets the closed-voting error, and the sealer and publisher
//! seal, sign, post and publish each box once.
//!
//! Needs `HASURA_DB__*`, `IMMUDB_*` and `MASTER_SECRET` (the stack's
//! `.devcontainer/.env`). The census and the public bucket are the test's
//! own ([`TestEnvironment`]); the signing key is the event's real protocol
//! manager key from the database vault.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use deadpool_postgres::{Client, Pool, Transaction};
use electoral_log::client::board_client::{
    ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use electoral_log::messages::statement::StatementType;
use electoral_log::seal::{verify_record, SealDisposition, SealRecord};
use sequent_core::ballot::ContestEncryptionPolicy;
use sequent_core::ballot::{
    EGracePeriodPolicy, ElectionEventStatus, ElectionPresentation, ElectionStatus,
    SignedHashableBallot, VotingPeriodEnd, VotingStatus, VotingStatusChannel, TYPES_VERSION,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Mutex;
use strand::signature::StrandSignatureSk;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::{
    list_for_elections, BallotBoxSeal, BallotBoxSealStatus, ClosedBy, ClosedBySigner,
};
use windmill::postgres::election_event::{delete_election_event, SEALED_EVENT_DELETE_REFUSAL};
use windmill::postgres::trusted_write;
use windmill::services::ballot_box_seal::publish::{publish_box, sealed_entries, PublishOutcome};
use windmill::services::ballot_box_seal::seal::{
    computed_ballot_id, post_failure, seal_box, SealOutcome,
};
use windmill::services::ballot_box_seal::sink::{closing_ballot_boxes, PendingSealSink};
use windmill::services::ballot_box_seal::{
    Census, ProductionSealEnvironment, SealEnvironment, BALLOT_BOX_SEALED_MESSAGE,
};
use windmill::services::election_event_status::{
    scheduled_change_applies, update_event_voting_status, update_scheduled_event_voting_status,
    update_scheduled_event_voting_status_for, SkippedElection, TransitionRefusal,
};
use windmill::services::insert_cast_vote::{check_seal_deadline, map_insert_error, CastVoteError};
use windmill::services::protocol_manager::{create_protocol_manager_keys, get_board_client};
use windmill::services::signing::actions::voting::{
    ClosingSignature, PostCloser, SealRecord as CloseRecord, StatusCloser,
};
use windmill::services::signing::actions::SealRecordSink;
use windmill::services::voting_status::update_election_status;

const ADMIN: &str = "admin";
const QUARTER_HOUR_SECS: u64 = 15 * 60;
const BOARD_ATTEMPTS: u32 = 10;

/// The census, the public bucket and the key: the key is the event's
/// real one, the rest is the test's.
struct TestEnvironment {
    census: Census,
    uploads: Mutex<Vec<(String, Vec<u8>)>>,
}

impl TestEnvironment {
    fn new(census: &[(&str, u64)]) -> Self {
        TestEnvironment {
            census: census
                .iter()
                .map(|(voter, weight)| (voter.to_string(), *weight))
                .collect(),
            uploads: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl SealEnvironment for TestEnvironment {
    async fn signing_key(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> Result<StrandSignatureSk> {
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
    ) -> Result<Uuid> {
        self.uploads
            .lock()
            .unwrap()
            .push((name.to_string(), json.to_vec()));
        Ok(document_id)
    }
}

/// An event with its board and key, its elections and two areas.
struct World {
    pool: Pool,
    board: String,
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

    /// Drops the private board.
    async fn finish(self) {
        if let Ok(mut board) = get_board_client().await {
            let _ = board.delete_database(&self.board).await;
        }
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
        }
    }
}

/// Runs `call` with a new board client, trying again when immudb drops a
/// session under parallel load ("already closed"); panics after
/// [`BOARD_ATTEMPTS`] tries.
async fn with_immudb<T, F, Fut>(what: &str, mut call: F) -> T
where
    F: FnMut(electoral_log::BoardClient) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut attempts = 0;
    loop {
        attempts += 1;
        let result = match get_board_client().await {
            Ok(client) => call(client).await,
            Err(error) => Err(error),
        };
        match result {
            Ok(value) => return value,
            Err(error) if attempts < BOARD_ATTEMPTS => {
                eprintln!("{what}: {error}; retrying");
                tokio::time::sleep(std::time::Duration::from_millis(200 * u64::from(attempts)))
                    .await;
            }
            Err(error) => panic!("{what}: {error}"),
        }
    }
}

/// The prefix of the boards these tests create.
const BOARD_PREFIX: &str = "sealtest";
static BOARDS_SWEPT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// Drops the boards an earlier, interrupted run left behind, once per run
/// and before this run creates any.
async fn sweep_boards() {
    let Ok(mut immudb) = windmill::services::protocol_manager::get_immudb_client().await else {
        return;
    };
    let Ok(list) = immudb.list_databases().await else {
        return;
    };
    for database in &list.get_ref().databases {
        if database.name.starts_with(BOARD_PREFIX) {
            let _ = immudb.delete_database(&database.name).await;
        }
    }
}

async fn world(options: Options) -> World {
    BOARDS_SWEPT.get_or_init(sweep_boards).await;
    let pool = schema::pool().await;
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let elections: Vec<Uuid> = (0..options.elections).map(|_| Uuid::new_v4()).collect();
    let areas = [Uuid::new_v4(), Uuid::new_v4()];
    let board = format!("{BOARD_PREFIX}{}", event.simple());
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
            &json!({ "ballot_box_seal_policy": options.policy }),
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
    create_protocol_manager_keys(&tx, &tenant.to_string(), &event.to_string(), &board)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    with_immudb(&format!("creating board {board}"), |mut client| {
        let board = board.clone();
        async move { client.upsert_electoral_log_db(&board).await }
    })
    .await;
    World {
        pool,
        board,
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
    let ballot_id = computed_ballot_id(&content, &ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
    (content, ballot_id)
}

/// Stores a ballot of `voter` in the box (election, area); returns its
/// Ballot ID.
async fn cast(
    w: &World,
    election: Uuid,
    area: Uuid,
    voter: &str,
    status: &str,
    seed: &str,
    minutes_ago: i64,
) -> String {
    let (content, ballot_id) = ballot(seed);
    insert_ballot(
        w,
        election,
        area,
        voter,
        status,
        &content,
        &ballot_id,
        minutes_ago,
    )
    .await
    .unwrap();
    ballot_id
}

#[allow(clippy::too_many_arguments)]
async fn insert_ballot(
    w: &World,
    election: Uuid,
    area: Uuid,
    voter: &str,
    status: &str,
    content: &str,
    ballot_id: &str,
    minutes_ago: i64,
) -> Result<u64, tokio_postgres::Error> {
    w.client()
        .await
        .execute(
            "INSERT INTO sequent_backend.cast_vote
                 (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                  status, content, ballot_id, created_at, annotations)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &w.tenant,
                &w.event,
                &election,
                &area,
                &voter,
                &status,
                &content,
                &ballot_id,
                &(Utc::now() - Duration::minutes(minutes_ago)),
                &json!({"voting_channel": "ONLINE"}),
            ],
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

#[tokio::test]
async fn a_manual_post_close_creates_one_pending_seal_per_box() {
    let w = world(Options {
        grace_secs: Some(QUARTER_HOUR_SECS),
        ..Default::default()
    })
    .await;
    // One box has ballots; the other has none and no ballot style, so only
    // the first is a box of the election here.
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

#[tokio::test]
async fn a_scheduled_post_close_and_a_signed_scheduled_close_are_scheduled() {
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
            "valid",
            &election.to_string(),
            1,
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
}

#[tokio::test]
async fn an_event_close_seals_every_election_with_its_provenance() {
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
                "valid",
                "e",
                1,
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
}

#[tokio::test]
async fn a_signed_close_records_its_request_and_signers() {
    let w = world(Options::default()).await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

#[tokio::test]
async fn policy_off_creates_no_seal_and_lets_voting_reopen() {
    let w = world(Options {
        policy: "do-not-seal",
        ..Default::default()
    })
    .await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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

#[tokio::test]
async fn closed_voting_never_reopens() {
    let w = world(Options {
        kiosk: true,
        ..Default::default()
    })
    .await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

#[tokio::test]
async fn an_event_opening_leaves_closed_posts_closed() {
    let w = world(Options {
        elections: 2,
        ..Default::default()
    })
    .await;
    let [closed, paused] = [w.elections[0], w.elections[1]];
    cast(&w, closed, w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

#[tokio::test]
async fn an_election_or_event_with_seals_cannot_be_deleted() {
    let w = world(Options::default()).await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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

#[tokio::test]
async fn a_cast_after_the_seal_deadline_is_refused() {
    let w = world(Options {
        grace_secs: Some(QUARTER_HOUR_SECS),
        ..Default::default()
    })
    .await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

#[tokio::test]
async fn a_cast_into_a_sealed_box_gets_the_closed_voting_error() {
    let w = world(Options::default()).await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
    close_election(&w, w.election(), Some(&w.user)).await;
    let environment = TestEnvironment::new(&[("voter-a", 1)]);
    let seal = w.seals(w.election()).await.remove(0);
    let mut client = w.client().await;
    assert_eq!(
        seal_box(&mut client, &environment, &seal.id).await.unwrap(),
        SealOutcome::Sealed
    );
    let (content, ballot_id) = ballot("late");
    let error = insert_ballot(
        &w,
        w.election(),
        w.areas[0],
        "voter-late",
        "valid",
        &content,
        &ballot_id,
        0,
    )
    .await
    .unwrap_err();
    match map_insert_error(anyhow::Error::from(error)) {
        CastVoteError::CheckStatusFailed(message) => {
            assert_eq!(message, BALLOT_BOX_SEALED_MESSAGE)
        }
        other => panic!("{other:?}"),
    }
    w.finish().await;
}

// ---------------------------------------------------------------------------
// Sealer and publisher
// ---------------------------------------------------------------------------

async fn log_entries(w: &World, kind: StatementType, election: Uuid, area: Uuid) -> usize {
    let filter: WhereClauseBTreeMap = BTreeMap::from([
        (
            ElectoralLogVarCharColumn::StatementKind,
            (SqlCompOperators::Equal, kind.to_string()),
        ),
        (
            ElectoralLogVarCharColumn::ElectionId,
            (SqlCompOperators::Equal, election.to_string()),
        ),
        (
            ElectoralLogVarCharColumn::AreaId,
            (SqlCompOperators::Equal, area.to_string()),
        ),
    ]);
    with_immudb("reading the board", |mut client| {
        let (board, filter) = (w.board.clone(), filter.clone());
        async move {
            client
                .get_electoral_log_messages_filtered::<String, String>(
                    &board,
                    Some(filter),
                    None,
                    None,
                    Some(100),
                    None,
                    None,
                )
                .await
        }
    })
    .await
    .len()
}

#[tokio::test]
async fn a_box_is_sealed_and_published_once() {
    let w = world(Options::default()).await;
    let (election, area) = (w.election(), w.areas[0]);
    let replaced = cast(&w, election, area, "voter-alice", "valid", "a1", 3).await;
    let counted = cast(&w, election, area, "voter-alice", "valid", "a2", 2).await;
    let weighted = cast(&w, election, area, "voter-bob", "valid", "b1", 2).await;
    let not_eligible = cast(&w, election, area, "voter-carol", "valid", "c1", 2).await;
    let discarded = cast(&w, election, area, "voter-dave", "discarded", "d1", 2).await;
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
    let entries = sealed_entries(&w.board, &election.to_string(), &area.to_string())
        .await
        .unwrap();
    assert_eq!(Some(entries[0].id), published.log_entry_id);

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
    // Signed by the event's key as both sender and system.
    assert_eq!(
        check.message.sender.pk.to_der_b64_string().unwrap(),
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
}

#[tokio::test]
async fn a_ballot_id_mismatch_fails_the_seal_and_logs_it_once() {
    let w = world(Options::default()).await;
    let (election, area) = (w.election(), w.areas[0]);
    let (content, _) = ballot("tampered");
    let (_, other_id) = ballot("other");
    insert_ballot(
        &w, election, area, "voter-a", "valid", &content, &other_id, 1,
    )
    .await
    .unwrap();
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
    // The box stays locked.
    let (late, late_id) = ballot("late");
    let refusal = insert_ballot(&w, election, area, "voter-b", "valid", &late, &late_id, 0)
        .await
        .unwrap_err();
    assert_eq!(
        refusal.as_db_error().unwrap().message(),
        "ballot_box_sealed"
    );
    // The failure entry, posted once however often the dispatcher repeats it.
    assert!(post_failure(&mut client, &environment, &seal.id)
        .await
        .unwrap());
    assert!(!post_failure(&mut client, &environment, &seal.id)
        .await
        .unwrap());
    assert_eq!(
        log_entries(&w, StatementType::BallotBoxSealFailed, election, area).await,
        1
    );
    assert_eq!(
        log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
        0
    );
    w.finish().await;
}

#[tokio::test]
async fn a_seal_already_on_the_board_fails_instead_of_a_second_seal() {
    let w = world(Options::default()).await;
    let (election, area) = (w.election(), w.areas[0]);
    cast(&w, election, area, "voter-a", "valid", "a", 1).await;
    close_election(&w, election, Some(&w.user)).await;
    let environment = TestEnvironment::new(&[("voter-a", 1)]);
    let seal = w.seals(election).await.remove(0);
    let mut client = w.client().await;
    assert_eq!(
        seal_box(&mut client, &environment, &seal.id).await.unwrap(),
        SealOutcome::Sealed
    );
    publish_box(&mut client, &environment, &seal.id)
        .await
        .unwrap();
    // A restore older than the seal: a new pending row for the same box.
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL session_replication_role = replica")
        .await
        .unwrap();
    tx.execute(
        "DELETE FROM sequent_backend.ballot_box_seal WHERE id = $1",
        &[&seal.id],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    windmill::postgres::ballot_box_seal::insert_pending(
        &tx,
        &[windmill::postgres::ballot_box_seal::NewPendingSeal {
            tenant_id: w.tenant,
            election_event_id: w.event,
            election_id: election,
            area_id: area,
            closed_at: seal.closed_at,
            grace_deadline: seal.grace_deadline,
            close_request_id: None,
            closed_by: ClosedBy::Scheduled,
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let restored = w.seals(election).await.remove(0);
    let outcome = seal_box(&mut client, &environment, &restored.id)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        SealOutcome::Failed(
            windmill::services::ballot_box_seal::seal::ALREADY_ON_BOARD_REASON.to_string()
        )
    );
    assert_eq!(
        log_entries(&w, StatementType::BallotBoxSealed, election, area).await,
        1
    );
    w.finish().await;
}

/// A refused delete leaves the boards in place: the seal check runs before
/// the task deletes the event's boards (B3, immudb) or anything else.
#[tokio::test]
async fn a_refused_event_delete_touches_no_board() {
    let w = world(Options::default()).await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
    // The event's electoral log board is still there.
    let board = w.board.clone();
    assert!(
        with_immudb("checking the board", |mut client| {
            let board = board.clone();
            async move { client.has_database(&board).await }
        })
        .await
    );
    w.finish().await;
}

/// A scheduled event-wide start doesn't open a channel of a Post that has
/// seals (one that never started counts as finished), and says why.
#[tokio::test]
async fn a_scheduled_event_opening_leaves_sealed_posts_closed() {
    let w = world(Options {
        elections: 2,
        kiosk: true,
        ..Default::default()
    })
    .await;
    let [sealed, open] = [w.elections[0], w.elections[1]];
    cast(&w, sealed, w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

/// Disabling a voter discards their ballots in unsealed boxes and keeps
/// those in sealed boxes as they were sealed.
#[tokio::test]
async fn a_discard_keeps_the_ballots_in_sealed_boxes() {
    let w = world(Options {
        elections: 2,
        ..Default::default()
    })
    .await;
    let [sealed, open] = [w.elections[0], w.elections[1]];
    cast(&w, sealed, w.areas[0], "voter-a", "valid", "a1", 2).await;
    cast(&w, open, w.areas[1], "voter-a", "valid", "a2", 1).await;
    close_election(&w, sealed, Some(&w.user)).await;
    let environment = TestEnvironment::new(&[("voter-a", 1)]);
    let seal = w.seals(sealed).await.remove(0);
    let mut client = w.client().await;
    assert_eq!(
        seal_box(&mut client, &environment, &seal.id).await.unwrap(),
        SealOutcome::Sealed
    );
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        windmill::services::ballot_box_seal::voter_sealed_boxes(
            &tx, &w.tenant, &w.event, "voter-a"
        )
        .await
        .unwrap(),
        vec![(sealed, w.areas[0])]
    );
    let discarded = windmill::postgres::cast_vote::discard_voter_cast_votes(
        &tx, &w.tenant, &w.event, "voter-a",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(discarded, 1);
    let statuses: Vec<(Uuid, String)> = client
        .query(
            "SELECT election_id, status FROM sequent_backend.cast_vote
             WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY election_id = $3",
            &[&w.tenant, &w.event, &open],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    assert_eq!(
        statuses,
        vec![
            (sealed, "valid".to_string()),
            (open, "discarded".to_string())
        ]
    );
    w.finish().await;
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
#[tokio::test]
async fn policy_off_still_refuses_stopping_a_never_started_channel() {
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
}

/// D1 (R7 B1): a scheduled close closes a never-started channel only when
/// the schedule names it and the Post actually ran; a Post that never
/// opened is left alone.
#[tokio::test]
async fn a_scheduled_close_closes_only_named_never_started_channels_of_posts_that_ran() {
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
            "valid",
            "s",
            1,
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
}

/// R6b-B1 / R7 B1: early voting runs and its scheduled end closes it; ONLINE
/// stays never started, nothing is due, and the Post is sealed only once
/// ONLINE has opened and closed.
#[tokio::test]
async fn early_voting_end_leaves_online_to_open_and_seal_later() {
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
    cast(&w, w.election(), w.areas[0], "voter-early", "valid", "e", 1).await;
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
}

/// R7 B2: after voting opened, writes that keep the effective policies
/// (a missing key, a JSON null or the default spelled out) pass; a real
/// change is refused.
#[tokio::test]
async fn presentation_writes_keep_working_after_voting_opened() {
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
}

/// B6b-S3: on a seal-at-close event a CLOSED channel stays CLOSED, also for
/// a trusted write (a stale read-modify-write).
#[tokio::test]
async fn a_closed_channel_stays_closed_in_the_database() {
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
}

/// B6b-S1 and D3: once voting opened, a seal-at-close event's ballot
/// reading and counting policies and its bulletin board can't change.
#[tokio::test]
async fn seal_relevant_event_settings_are_locked_once_voting_opened() {
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
}

/// B6b-S2: the guard takes no advisory lock for a policy-off event, so a
/// cast doesn't wait on a held box key there.
#[tokio::test]
async fn the_guard_takes_no_box_lock_when_the_policy_is_off() {
    let w = world(Options {
        policy: "do-not-seal",
        ..Default::default()
    })
    .await;
    let mut holder = w.client().await;
    let held = holder.transaction().await.unwrap();
    held.execute(
        "SELECT pg_advisory_xact_lock(hashtextextended(sequent_backend.ballot_box_lock_key($1, $2, $3, $4), 0))",
        &[&w.tenant, &w.event, &w.election(), &w.areas[0]],
    )
    .await
    .unwrap();
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL lock_timeout = '2s'")
        .await
        .unwrap();
    let (content, ballot_id) = ballot("unlocked");
    tx.execute(
        "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string, status,
              content, ballot_id)
         VALUES ($1, $2, $3, $4, 'voter-x', 'valid', $5, $6)",
        &[
            &w.tenant,
            &w.event,
            &w.election(),
            &w.areas[0],
            &content,
            &ballot_id,
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    drop(held);
    w.finish().await;
}

/// B6b-S1 and D5: a configuration that doesn't parse is retried, never an
/// incident; each attempt records why the box isn't sealed.
#[tokio::test]
async fn attempts_record_why_and_configuration_errors_are_retried() {
    let w = world(Options {
        grace_secs: Some(QUARTER_HOUR_SECS),
        ..Default::default()
    })
    .await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
    insert_ballot(
        &w,
        w.election(),
        w.areas[0],
        "voter-b",
        "in-progress",
        &content,
        &ballot_id,
        0,
    )
    .await
    .unwrap();
    assert_eq!(
        seal_box(&mut client, &environment, &seal.id).await.unwrap(),
        SealOutcome::WaitingForVotes(1)
    );
    assert_eq!(
        seal_row(&w, w.election()).await.waiting_reason.as_deref(),
        Some("datafix_votes:1")
    );
    bypass(
        &w,
        "UPDATE sequent_backend.cast_vote SET status = 'discarded' WHERE voter_id_string = 'voter-b' AND election_id = $1",
        &[&w.election()],
    )
    .await;

    // Another run holds the box.
    {
        let mut holder = w.client().await;
        let held = holder.transaction().await.unwrap();
        held.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&seal.lock_key()],
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

    // A presentation that doesn't parse: an error, the seal stays pending.
    bypass(
        &w,
        "UPDATE sequent_backend.election_event
         SET presentation = presentation || '{\"contest_encryption_policy\": \"bogus\"}' WHERE id = $1",
        &[&w.event],
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
}

/// B6c-S1: a seal row another transaction holds is skipped, not waited
/// for, by the sealer and the publisher.
#[tokio::test]
async fn a_held_seal_row_is_busy_not_waited_for() {
    let w = world(Options::default()).await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

/// B6b-S5: a published ballot style that can't be read doesn't stop the
/// close; its area still gets a seal.
#[tokio::test]
async fn an_unreadable_ballot_style_does_not_stop_the_close() {
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
}

/// B6c-S3: a failed seal's entry is posted once and recorded; the
/// dispatcher no longer lists it. E17: the record keeps the names the seal
/// signed, also after a rename.
#[tokio::test]
async fn the_failure_entry_is_recorded_and_the_record_keeps_the_signed_names() {
    let w = world(Options {
        elections: 2,
        ..Default::default()
    })
    .await;
    let [failing, renamed] = [w.elections[0], w.elections[1]];
    let (content, _) = ballot("tampered");
    let (_, other_id) = ballot("other");
    insert_ballot(
        &w, failing, w.areas[0], "voter-a", "valid", &content, &other_id, 1,
    )
    .await
    .unwrap();
    cast(&w, renamed, w.areas[1], "voter-b", "valid", "b", 1).await;
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
    let tx = client.transaction().await.unwrap();
    let work = windmill::postgres::ballot_box_seal::list_open_work_ids(&tx, Utc::now(), 1000)
        .await
        .unwrap();
    drop(tx);
    assert!(work.contains(&failed.id), "not posted yet");
    // The incident keeps the names (R7 S3).
    let failed_row = seal_row(&w, failing).await;
    assert_eq!(failed_row.area_name.as_deref(), Some("Area"));
    assert_eq!(
        failed_row.election_name.as_deref(),
        Some(failing.to_string().as_str())
    );
    assert!(post_failure(&mut client, &environment, &failed.id)
        .await
        .unwrap());
    assert!(seal_row(&w, failing).await.failure_posted_at.is_some());
    let tx = client.transaction().await.unwrap();
    let work = windmill::postgres::ballot_box_seal::list_open_work_ids(&tx, Utc::now(), 1000)
        .await
        .unwrap();
    drop(tx);
    assert!(!work.contains(&failed.id), "posted: no more re-posting");
    assert!(!post_failure(&mut client, &environment, &failed.id)
        .await
        .unwrap());

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
            .to_der_b64_string()
            .unwrap(),
        record.system_public_key
    );
    w.finish().await;
}

/// D4: the close's schedule finds the new, untried pending seals.
#[tokio::test]
async fn a_close_schedules_its_new_seals() {
    let w = world(Options {
        grace_secs: Some(QUARTER_HOUR_SECS),
        ..Default::default()
    })
    .await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

/// E18: entries someone else signed don't count, however many come first:
/// the box is sealed, and its genuine entry is found and published.
#[tokio::test]
async fn junk_seal_entries_neither_block_nor_hide_the_genuine_one() {
    use electoral_log::messages::message::{Message, SigningData};
    use electoral_log::seal::{build, BallotBoxSealManifest, SEAL_FORMAT_V1};
    let w = world(Options::default()).await;
    let (election, area) = (w.election(), w.areas[0]);
    cast(&w, election, area, "voter-a", "valid", "a", 1).await;
    close_election(&w, election, Some(&w.user)).await;
    let intruder = StrandSignatureSk::generate().unwrap();
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
        let id = windmill::services::ballot_box_seal::publish::sha256_hex(
            format!("junk:{election}:{index}").as_bytes(),
        );
        windmill::services::ballot_box_seal::publish::deliver(&w.board, &id, &id, &message)
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
    let entries = sealed_entries(&w.board, &election.to_string(), &area.to_string())
        .await
        .unwrap();
    assert_eq!(entries.len(), 106);
    assert_eq!(
        Some(entries.last().unwrap().id),
        seal_row(&w, election).await.log_entry_id
    );
    w.finish().await;
}

/// R7 S7: the attempt bookkeeping doesn't wait for a row another run holds.
#[tokio::test]
async fn recording_an_attempt_does_not_wait_for_a_held_row() {
    let w = world(Options::default()).await;
    cast(&w, w.election(), w.areas[0], "voter-a", "valid", "a", 1).await;
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
}

/// R8 B1: a KIOSK-only Post whose (not enabled) ONLINE an event-wide Start
/// marked open never opened: a scheduled END leaves it alone, no seal.
#[tokio::test]
async fn a_scheduled_close_leaves_a_kiosk_only_post_that_never_opened() {
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
}

/// R8 S2: a status change locks its rows FOR NO KEY UPDATE, so a cast
/// (whose foreign-key checks take FOR KEY SHARE) doesn't wait for it.
#[tokio::test]
async fn a_cast_does_not_wait_for_a_status_change() {
    let w = world(Options {
        policy: "do-not-seal",
        ..Default::default()
    })
    .await;
    let mut holder = w.client().await;
    let held = holder.transaction().await.unwrap();
    windmill::services::election_event_status::lock_status_rows(
        &held,
        &w.tenant.to_string(),
        &w.event.to_string(),
        None,
    )
    .await
    .unwrap();
    let mut client = w.client().await;
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL lock_timeout = '1s'")
        .await
        .unwrap();
    let (content, ballot_id) = ballot("not-blocked");
    tx.execute(
        "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string, status,
              content, ballot_id)
         VALUES ($1, $2, $3, $4, 'voter-x', 'valid', $5, $6)",
        &[
            &w.tenant,
            &w.event,
            &w.election(),
            &w.areas[0],
            &content,
            &ballot_id,
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    drop(held);
    w.finish().await;
}

/// R7 S2 / R8 S5: a status change that waited for another one reads the
/// event status after it, so it doesn't write a stale copy back.
#[tokio::test]
async fn a_waiting_status_change_does_not_write_back_a_stale_event_status() {
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
}

/// R8 S1: a status writer takes no signing lock, so it doesn't wait for (or
/// deadlock with) a claim holding it.
#[tokio::test]
async fn a_status_change_does_not_wait_for_the_signing_lock() {
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
}
