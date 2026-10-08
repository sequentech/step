// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box key and the ballot checks of an election event against the
//! migrated schema. Every test writes its own rows in a transaction and rolls
//! it back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use sequent_core::ballot::{ChecksPeriod, ReceiptsPolicy, VotingStatusChannel};
use sequent_core::types::hasura::core::ElectionEvent;
use serde_json::{json, Value};
use std::sync::Once;
use uuid::Uuid;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::ballot_box_key::{
    ballot_box_key_for_publication, get_ballot_box_secret_path, get_ballot_box_signing_key,
    get_or_create_ballot_box_signing_key, published_key, receipts_policy,
};
use windmill::services::ballot_checks::{
    ballot_id_match, get_checks_period, locate_ballot, BallotIdMatch, LocateBallotStatus,
};
use windmill::services::receive_ballot::must_be_received;

const SIGNED_RECEIPTS: &str = "signed-by-ballot-box";

/// The secrets of an event are encrypted with the deployment's master secret.
fn master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2028, 4, 8, 12, 0, 0).unwrap()
}

/// Whether the event has its bulletin board, which names its secrets.
enum Board {
    Created,
    Missing,
}

struct Event {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
}

impl Event {
    async fn create(
        tx: &Transaction<'_>,
        presentation: Option<Value>,
        board: Board,
        voting_channels: Option<Value>,
    ) -> Self {
        let fixture = Self {
            tenant: Uuid::new_v4(),
            event: Uuid::new_v4(),
            election: Uuid::new_v4(),
        };
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&fixture.tenant, &format!("tenant-{}", fixture.tenant)],
        )
        .await
        .unwrap();
        let board = match board {
            Board::Created => Some(json!({
                "id": 1,
                "database_name": fixture.board(),
                "is_archived": false,
            })),
            Board::Missing => None,
        };
        tx.execute(
            "INSERT INTO sequent_backend.election_event
                 (id, tenant_id, encryption_protocol, presentation, bulletin_board_reference)
             VALUES ($1, $2, 'RSA256', $3, $4)",
            &[&fixture.event, &fixture.tenant, &presentation, &board],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, voting_channels)
             VALUES ($1, $2, $3, $4)",
            &[
                &fixture.election,
                &fixture.tenant,
                &fixture.event,
                &voting_channels,
            ],
        )
        .await
        .unwrap();
        fixture
    }

    async fn with_receipts(tx: &Transaction<'_>, policy: &str) -> Self {
        Self::create(
            tx,
            Some(json!({"receipts": {"policy": policy}})),
            Board::Created,
            None,
        )
        .await
    }

    /// Board names are unique, and so is the secret each one names.
    fn board(&self) -> String {
        format!("board-{}", self.event.simple())
    }

    async fn stored(&self, tx: &Transaction<'_>) -> ElectionEvent {
        get_election_event_by_id(tx, &self.tenant.to_string(), &self.event.to_string())
            .await
            .unwrap()
    }

    async fn secrets(&self, tx: &Transaction<'_>) -> i64 {
        tx.query_one(
            "SELECT count(*) FROM sequent_backend.secret
             WHERE tenant_id = $1 AND election_event_id = $2 AND key = $3",
            &[
                &self.tenant,
                &self.event,
                &get_ballot_box_secret_path(&self.board()),
            ],
        )
        .await
        .unwrap()
        .get(0)
    }

    async fn locate(&self, tx: &Transaction<'_>, ballot_id: &str) -> LocateBallotStatus {
        locate_ballot(
            tx,
            &self.tenant.to_string(),
            &self.event.to_string(),
            &self.election.to_string(),
            &Uuid::new_v4().to_string(),
            "voter",
            ballot_id,
            now(),
        )
        .await
        .unwrap()
        .status
    }
}

#[tokio::test]
async fn an_event_without_signed_receipts_publishes_no_key_and_creates_none() {
    master_secret();
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    for presentation in [None, Some(json!({"receipts": {"policy": "disabled"}}))] {
        let f = Event::create(&tx, presentation, Board::Created, None).await;
        let event = f.stored(&tx).await;

        assert_eq!(receipts_policy(&event).unwrap(), ReceiptsPolicy::DISABLED);
        assert_eq!(
            ballot_box_key_for_publication(&tx, &event).await.unwrap(),
            None
        );
        assert!(get_ballot_box_signing_key(&tx, &event)
            .await
            .unwrap()
            .is_none());
        assert_eq!(f.secrets(&tx).await, 0);
    }
}

#[tokio::test]
async fn publishing_with_signed_receipts_creates_the_key_once_and_publishes_it_again() {
    master_secret();
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Event::with_receipts(&tx, SIGNED_RECEIPTS).await;
    let event = f.stored(&tx).await;
    assert_eq!(
        receipts_policy(&event).unwrap(),
        ReceiptsPolicy::SIGNED_BY_BALLOT_BOX
    );

    let published = ballot_box_key_for_publication(&tx, &event)
        .await
        .unwrap()
        .expect("signed receipts publish the ballot box key");
    assert_eq!(f.secrets(&tx).await, 1);

    // A later publication, and the ballot box when it signs, find that key.
    assert_eq!(
        ballot_box_key_for_publication(&tx, &event).await.unwrap(),
        Some(published.clone())
    );
    let signing_key = get_ballot_box_signing_key(&tx, &event)
        .await
        .unwrap()
        .expect("the key was stored");
    assert_eq!(published_key(&signing_key).unwrap(), published);
    let again = get_or_create_ballot_box_signing_key(&tx, &event)
        .await
        .unwrap();
    assert_eq!(published_key(&again).unwrap(), published);
    assert_eq!(f.secrets(&tx).await, 1);
}

#[tokio::test]
async fn each_event_has_its_own_ballot_box_key() {
    master_secret();
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let one = Event::with_receipts(&tx, SIGNED_RECEIPTS).await;
    let other = Event::with_receipts(&tx, SIGNED_RECEIPTS).await;

    let key = ballot_box_key_for_publication(&tx, &one.stored(&tx).await)
        .await
        .unwrap()
        .unwrap();
    let other_key = ballot_box_key_for_publication(&tx, &other.stored(&tx).await)
        .await
        .unwrap()
        .unwrap();

    assert_ne!(key.key_id, other_key.key_id);
    assert_ne!(key.public_key, other_key.public_key);
}

#[tokio::test]
async fn an_event_without_a_board_or_with_an_unknown_policy_has_no_key() {
    master_secret();
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();

    let boardless = Event::create(
        &tx,
        Some(json!({"receipts": {"policy": SIGNED_RECEIPTS}})),
        Board::Missing,
        None,
    )
    .await;
    let event = boardless.stored(&tx).await;
    let error = get_ballot_box_signing_key(&tx, &event)
        .await
        .err()
        .expect("an event without a board has no key path");
    assert!(error.to_string().contains("missing bulletin board"));
    assert!(ballot_box_key_for_publication(&tx, &event).await.is_err());

    let unknown = Event::with_receipts(&tx, "sometimes").await;
    let event = unknown.stored(&tx).await;
    assert!(receipts_policy(&event).is_err());
    assert!(ballot_box_key_for_publication(&tx, &event).await.is_err());
    assert_eq!(unknown.secrets(&tx).await, 0);
}

#[test]
fn only_signed_receipts_need_the_ballot_received_and_never_by_telephone() {
    for channel in [VotingStatusChannel::ONLINE, VotingStatusChannel::TELEPHONE] {
        assert!(!must_be_received(&ReceiptsPolicy::DISABLED, channel));
    }
    assert!(must_be_received(
        &ReceiptsPolicy::SIGNED_BY_BALLOT_BOX,
        VotingStatusChannel::ONLINE
    ));
    assert!(!must_be_received(
        &ReceiptsPolicy::SIGNED_BY_BALLOT_BOX,
        VotingStatusChannel::TELEPHONE
    ));
}

#[tokio::test]
async fn the_checks_period_is_read_from_the_stored_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let period = |f: &Event| {
        let (tenant, event) = (f.tenant.to_string(), f.event.to_string());
        let tx = &tx;
        async move { get_checks_period(tx, &tenant, &event, now()).await }
    };

    let unlimited = Event::create(&tx, None, Board::Missing, None).await;
    assert_eq!(period(&unlimited).await.unwrap(), ChecksPeriod::Unlimited);

    for (until, ended) in [
        ("2028-04-08T12:00:01+00:00", false),
        ("2028-04-08T19:59:59+08:00", true),
    ] {
        let f = Event::create(
            &tx,
            Some(json!({"receipts": {
                "checks_period_policy": "until-date",
                "checks_available_until": until,
            }})),
            Board::Missing,
            None,
        )
        .await;
        let until = DateTime::parse_from_rfc3339(until)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            period(&f).await.unwrap(),
            if ended {
                ChecksPeriod::Ended(until)
            } else {
                ChecksPeriod::OpenUntil(until)
            }
        );
    }

    let undated = Event::create(
        &tx,
        Some(json!({"receipts": {"checks_period_policy": "until-date"}})),
        Board::Missing,
        None,
    )
    .await;
    assert!(period(&undated).await.is_err());
}

#[tokio::test]
async fn a_ballot_the_voter_did_not_cast_is_not_found_and_nothing_is_looked_up_after_the_period() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();

    let open = Event::create(&tx, None, Board::Missing, Some(json!({"telephone": true}))).await;
    for ballot_id in ["0abc12", "0ABC", "FTBE-MHRX", "not a ballot id", ""] {
        assert_eq!(
            open.locate(&tx, ballot_id).await,
            LocateBallotStatus::NotFound
        );
    }

    let ended = Event::create(
        &tx,
        Some(json!({"receipts": {
            "checks_period_policy": "until-date",
            "checks_available_until": "2028-04-08T11:59:59Z",
        }})),
        Board::Missing,
        None,
    )
    .await;
    let output = locate_ballot(
        &tx,
        &ended.tenant.to_string(),
        &ended.event.to_string(),
        // Not even the election is read once checks have ended.
        &Uuid::new_v4().to_string(),
        &Uuid::new_v4().to_string(),
        "voter",
        "0abc12",
        now(),
    )
    .await
    .unwrap();
    assert_eq!(output.status, LocateBallotStatus::ChecksEnded);
    assert_eq!(
        output.checks_available_until.as_deref(),
        Some("2028-04-08T11:59:59+00:00")
    );
    assert_eq!(output.ballot_id, None);

    let unknown_election = locate_ballot(
        &tx,
        &open.tenant.to_string(),
        &open.event.to_string(),
        &Uuid::new_v4().to_string(),
        &Uuid::new_v4().to_string(),
        "voter",
        "0abc12",
        now(),
    )
    .await;
    assert!(unknown_election.is_err());
}

#[test]
fn a_typed_ballot_id_becomes_an_exact_match_or_a_telephone_prefix() {
    assert_eq!(
        ballot_id_match(" 0ABC12 ", false),
        Some(BallotIdMatch::Exact("0abc12".to_string()))
    );
    let prefix = ballot_id_match("0ABC", true).unwrap();
    assert_eq!(prefix, BallotIdMatch::Prefix("0abc".to_string()));
    assert_eq!(prefix.like_pattern(), "0abc%");
    let exact = ballot_id_match("0abc", false).unwrap();
    assert_eq!(exact.like_pattern(), "0abc");
    for text in ["", "50%", "not hex"] {
        assert_eq!(ballot_id_match(text, true), None);
    }
}
