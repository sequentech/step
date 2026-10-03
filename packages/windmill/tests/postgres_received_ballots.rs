// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The received ballot adapter against the migrated schema. Every test writes
//! its own rows in a transaction, checks them with independent SQL and rolls
//! the transaction back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot_receipt::{
    sign_cast_statement, sign_received_ballot, verify_cast_receipt, ReceivedBallot,
    ReceivedStatement,
};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use uuid::Uuid;
use windmill::postgres::cast_vote::{self, CastVoteReceipt};
use windmill::postgres::received_ballot::{
    get_published_ballot_eml, get_received_ballot, get_received_ballot_to_cast,
    insert_received_ballot, mark_received_ballot_cast, ReceivedBallotScope, ReceivedBallotStatus,
    ReceivedBallotToCast, StoredCast,
};
use windmill::services::ballot_box_key::published_key;
use windmill::services::cast_ballot::{
    find_received_cast, received_cast, store_cast_receipt, CastBallotInput, FoundCast,
};
use windmill::services::cast_votes::CastVoteStatus;
use windmill::services::insert_cast_vote::CastVoteError;

const BALLOT_EML: &str = r#"{"id":"style"}"#;

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

fn received_at() -> DateTime<Utc> {
    Utc.timestamp_millis_opt(1_841_367_600_007).unwrap()
}

fn cast_at() -> DateTime<Utc> {
    Utc.timestamp_millis_opt(1_841_367_845_678).unwrap()
}

/// One election of one area in a new tenant and election event.
struct Election {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
}

impl Election {
    async fn create(tx: &Transaction<'_>) -> Self {
        let tenant = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
        Self::create_in(tx, tenant).await
    }

    async fn create_in(tx: &Transaction<'_>, tenant: Uuid) -> Self {
        let fixture = Self {
            tenant,
            event: Uuid::new_v4(),
            election: Uuid::new_v4(),
            area: Uuid::new_v4(),
        };
        tx.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&fixture.event, &tenant],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&fixture.election, &tenant, &fixture.event],
        )
        .await
        .unwrap();
        fixture.area_as(tx, fixture.area).await;
        fixture
    }

    async fn area_as(&self, tx: &Transaction<'_>, area: Uuid) {
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, $4)",
            &[&area, &self.tenant, &self.event, &format!("area-{area}")],
        )
        .await
        .unwrap();
    }

    fn scope<'a>(&'a self, voter_id: &'a str, ballot_hash: &'a str) -> ReceivedBallotScope<'a> {
        ReceivedBallotScope {
            tenant_id: &self.tenant,
            election_event_id: &self.event,
            election_id: &self.election,
            voter_id,
            ballot_hash,
        }
    }

    fn receipt(&self, ballot_hash: &str, ballot_id: &str) -> ReceivedBallot {
        ReceivedBallot {
            statement: ReceivedStatement {
                tenant_id: self.tenant.to_string(),
                election_event_id: self.event.to_string(),
                election_id: self.election.to_string(),
                ballot_hash: ballot_hash.into(),
                voter_signing_pk: "voter-key".into(),
                voter_ballot_signature: "voter-signature".into(),
                received_at: "2028-05-08T03:00:00.007Z".into(),
                key_id: "fd110d301d2f077d".into(),
            },
            received_signature: "ballot-box-signature".into(),
            ballot_id: ballot_id.into(),
        }
    }

    async fn receive(
        &self,
        tx: &Transaction<'_>,
        voter_id: &str,
        ballot_hash: &str,
        ballot_id: &str,
    ) -> Option<Uuid> {
        insert_received_ballot(
            tx,
            &self.scope(voter_id, ballot_hash),
            &self.area,
            "ciphertext",
            &received_at(),
            &self.receipt(ballot_hash, ballot_id),
        )
        .await
        .unwrap()
    }

    async fn to_cast(
        &self,
        tx: &Transaction<'_>,
        voter_id: &str,
        ballot_id: &str,
    ) -> Option<ReceivedBallotToCast> {
        get_received_ballot_to_cast(
            tx,
            &self.tenant,
            &self.event,
            &self.election,
            voter_id,
            ballot_id,
        )
        .await
        .unwrap()
    }

    async fn cast(
        &self,
        tx: &Transaction<'_>,
        received_ballot_id: &Uuid,
        cast_signature: &str,
        cast_receipt_signature: &str,
    ) -> bool {
        mark_received_ballot_cast(
            tx,
            &self.tenant,
            &self.event,
            received_ballot_id,
            &cast_at(),
            cast_signature,
            cast_receipt_signature,
        )
        .await
        .unwrap()
    }

    async fn stored(&self, tx: &Transaction<'_>) -> i64 {
        tx.query_one(
            "SELECT count(*) FROM sequent_backend.received_ballot
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&self.tenant, &self.event],
        )
        .await
        .unwrap()
        .get(0)
    }

    /// A ballot style of `area` in a publication with the given dates.
    async fn style(
        &self,
        tx: &Transaction<'_>,
        area: Uuid,
        published_at: Option<DateTime<Utc>>,
        publication_deleted_at: Option<DateTime<Utc>>,
        style_deleted_at: Option<DateTime<Utc>>,
    ) -> Uuid {
        let publication = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.ballot_publication
                 (id, tenant_id, election_event_id, election_ids, published_at, deleted_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &publication,
                &self.tenant,
                &self.event,
                &vec![self.election],
                &published_at,
                &publication_deleted_at,
            ],
        )
        .await
        .unwrap();
        let style = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.ballot_style
                 (id, tenant_id, election_event_id, election_id, area_id,
                  ballot_publication_id, deleted_at, ballot_eml)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &style,
                &self.tenant,
                &self.event,
                &self.election,
                &area,
                &publication,
                &style_deleted_at,
                &BALLOT_EML,
            ],
        )
        .await
        .unwrap();
        style
    }

    async fn published_eml(
        &self,
        tx: &Transaction<'_>,
        area: Uuid,
        style: Uuid,
        with_ballot_eml: bool,
    ) -> Option<Option<String>> {
        get_published_ballot_eml(
            tx,
            &self.tenant,
            &self.event,
            &self.election,
            &area,
            &style,
            with_ballot_eml,
        )
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn a_received_ballot_is_read_back_with_the_receipt_the_ballot_box_signed() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;

    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();
    let stored = get_received_ballot(&tx, &f.scope("voter", "hash"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(stored.id, id);
    assert_eq!(stored.status, ReceivedBallotStatus::Received);
    assert_eq!(stored.received, f.receipt("hash", "FTBE-MHRX"));
    let row = tx
        .query_one(
            "SELECT content, area_id, voter_id_string, cast_at
             FROM sequent_backend.received_ballot WHERE id = $1",
            &[&id],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>("content"), "ciphertext");
    assert_eq!(row.get::<_, Uuid>("area_id"), f.area);
    assert_eq!(row.get::<_, String>("voter_id_string"), "voter");
    assert_eq!(row.get::<_, Option<DateTime<Utc>>>("cast_at"), None);
}

#[tokio::test]
async fn a_ballot_the_ballot_box_does_not_have_is_not_found() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    for (voter, hash) in [("another-voter", "hash"), ("voter", "another-hash")] {
        assert_eq!(
            get_received_ballot(&tx, &f.scope(voter, hash))
                .await
                .unwrap(),
            None,
            "{voter} {hash}"
        );
    }
}

#[tokio::test]
async fn the_same_ballot_is_stored_once_and_keeps_its_first_receipt() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;

    assert!(f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.is_some());
    assert_eq!(f.receive(&tx, "voter", "hash", "0000-0001").await, None);

    assert_eq!(f.stored(&tx).await, 1);
    let stored = get_received_ballot(&tx, &f.scope("voter", "hash"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.received.ballot_id, "FTBE-MHRX");
}

#[tokio::test]
async fn a_ballot_id_names_one_ballot_in_its_election_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let another_event = Election::create_in(&tx, f.tenant).await;

    assert!(f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.is_some());
    assert_eq!(
        f.receive(&tx, "another-voter", "another-hash", "FTBE-MHRX")
            .await,
        None
    );
    assert_eq!(f.stored(&tx).await, 1);
    // Signing a millisecond later gives the ballot another ID.
    assert!(f
        .receive(&tx, "another-voter", "another-hash", "0000-0001")
        .await
        .is_some());
    assert!(another_event
        .receive(&tx, "voter", "hash", "FTBE-MHRX")
        .await
        .is_some());
}

#[tokio::test]
async fn a_received_ballot_is_found_by_its_ballot_id_only_for_its_voter() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();
    let another_election = Uuid::new_v4();

    let found = f.to_cast(&tx, "voter", "FTBE-MHRX").await.unwrap();
    assert_eq!(found.stored.id, id);
    assert_eq!(found.stored.status, ReceivedBallotStatus::Received);
    assert_eq!(found.stored.received, f.receipt("hash", "FTBE-MHRX"));
    assert_eq!(found.content, "ciphertext");
    assert_eq!(found.cast, None);

    assert_eq!(f.to_cast(&tx, "another-voter", "FTBE-MHRX").await, None);
    assert_eq!(f.to_cast(&tx, "voter", "0000-0000").await, None);
    assert_eq!(
        get_received_ballot_to_cast(
            &tx,
            &f.tenant,
            &f.event,
            &another_election,
            "voter",
            "FTBE-MHRX"
        )
        .await
        .unwrap(),
        None
    );
}

#[tokio::test]
async fn a_received_ballot_is_cast_once_and_keeps_its_first_receipt() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    assert!(
        f.cast(&tx, &id, "cast-signature", "receipt-signature")
            .await
    );
    let cast = f.to_cast(&tx, "voter", "FTBE-MHRX").await.unwrap();
    assert_eq!(cast.stored.status, ReceivedBallotStatus::Cast);
    assert_eq!(
        cast.cast,
        Some(StoredCast {
            cast_at: cast_at(),
            cast_signature: "cast-signature".into(),
            cast_receipt_signature: "receipt-signature".into(),
        })
    );

    assert!(
        !f.cast(&tx, &id, "another-signature", "another-receipt")
            .await
    );
    assert_eq!(f.to_cast(&tx, "voter", "FTBE-MHRX").await, Some(cast));
}

#[tokio::test]
async fn a_ballot_of_another_election_event_is_not_cast() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let other = Election::create_in(&tx, f.tenant).await;
    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    assert!(
        !other
            .cast(&tx, &id, "cast-signature", "receipt-signature")
            .await
    );
    assert!(
        !f.cast(&tx, &Uuid::new_v4(), "cast-signature", "receipt-signature")
            .await
    );
    assert_eq!(
        f.to_cast(&tx, "voter", "FTBE-MHRX")
            .await
            .unwrap()
            .stored
            .status,
        ReceivedBallotStatus::Received
    );
}

#[tokio::test]
async fn an_audited_ballot_cannot_be_cast() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.received_ballot SET status = 'audited' WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();

    assert!(
        !f.cast(&tx, &id, "cast-signature", "receipt-signature")
            .await
    );
    let audited = f.to_cast(&tx, "voter", "FTBE-MHRX").await.unwrap();
    assert_eq!(audited.stored.status, ReceivedBallotStatus::Audited);
    assert_eq!(audited.cast, None);
}

#[tokio::test]
async fn the_table_refuses_a_cast_receipt_on_a_ballot_that_is_not_cast() {
    let mut client = connect().await;
    let mut tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    for assignment in [
        "cast_signature = 's', cast_receipt_signature = 'r'",
        "status = 'cast', cast_at = now(), cast_signature = 's'",
        "status = 'cast'",
        "cast_at = now()",
    ] {
        let savepoint = tx.savepoint("attempt").await.unwrap();
        let error = savepoint
            .execute(
                &format!(
                    "UPDATE sequent_backend.received_ballot SET {assignment}
                     WHERE tenant_id = $1 AND election_event_id = $2"
                ),
                &[&f.tenant, &f.event],
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.as_db_error().unwrap().constraint(),
            Some("received_ballot_cast_receipt_check"),
            "{assignment}"
        );
        savepoint.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn the_cast_vote_keeps_the_receipt_and_its_time() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let received = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();
    let receipt = CastVoteReceipt {
        received_ballot_id: &received,
        cast_at: &cast_at(),
        cast_receipt_signature: "receipt-signature",
    };

    let mut ids = vec![];
    for (voter, cast_receipt) in [("voter", Some(&receipt)), ("another-voter", None)] {
        let cast_vote = cast_vote::insert_cast_vote(
            &tx,
            &f.tenant,
            &f.event,
            &f.election,
            &f.area,
            "ciphertext",
            voter,
            "FTBE-MHRX",
            &[0; 64],
            &None,
            &None,
            VotingStatusChannel::ONLINE,
            CastVoteStatus::Valid,
            cast_receipt,
        )
        .await
        .unwrap();
        assert_eq!(cast_vote.ballot_id.as_deref(), Some("FTBE-MHRX"));
        ids.push(Uuid::parse_str(&cast_vote.id).unwrap());
    }

    assert_eq!(
        cast_vote::get_cast_vote_id_of_received_ballot(
            &tx,
            &f.tenant,
            &f.event,
            &f.election,
            "voter",
            &received
        )
        .await
        .unwrap(),
        Some(ids[0])
    );
    for (voter, received_ballot_id) in [("another-voter", received), ("voter", Uuid::new_v4())] {
        assert_eq!(
            cast_vote::get_cast_vote_id_of_received_ballot(
                &tx,
                &f.tenant,
                &f.event,
                &f.election,
                voter,
                &received_ballot_id
            )
            .await
            .unwrap(),
            None,
            "{voter}"
        );
    }

    let stored: Vec<(Option<Uuid>, Option<String>, bool)> = tx
        .query(
            "SELECT received_ballot_id, cast_receipt_signature, created_at = $2
             FROM sequent_backend.cast_vote
             WHERE id = ANY($1) ORDER BY voter_id_string DESC",
            &[&ids, &cast_at()],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect();
    assert_eq!(
        stored,
        vec![
            (Some(received), Some("receipt-signature".into()), true),
            (None, None, false)
        ]
    );
}

#[tokio::test]
async fn only_the_style_published_for_the_voters_area_and_election_is_found() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let another_area = Uuid::new_v4();
    f.area_as(&tx, another_area).await;
    let now = Utc::now();

    let published = f.style(&tx, f.area, Some(now), None, None).await;
    assert_eq!(
        f.published_eml(&tx, f.area, published, true).await,
        Some(Some(BALLOT_EML.to_string()))
    );
    // The style is known to be published without reading its ballot.
    assert_eq!(
        f.published_eml(&tx, f.area, published, false).await,
        Some(None)
    );
    assert_eq!(
        f.published_eml(&tx, another_area, published, true).await,
        None
    );
    assert_eq!(
        get_published_ballot_eml(
            &tx,
            &f.tenant,
            &f.event,
            &Uuid::new_v4(),
            &f.area,
            &published,
            true
        )
        .await
        .unwrap(),
        None
    );

    let not_published = f.style(&tx, f.area, None, None, None).await;
    let publication_deleted = f.style(&tx, f.area, Some(now), Some(now), None).await;
    let style_deleted = f.style(&tx, f.area, Some(now), None, Some(now)).await;
    for (style, why) in [
        (not_published, "not published"),
        (publication_deleted, "publication deleted"),
        (style_deleted, "style deleted"),
        (Uuid::new_v4(), "unknown style"),
    ] {
        assert_eq!(
            f.published_eml(&tx, f.area, style, true).await,
            None,
            "{why}"
        );
    }
}

#[tokio::test]
async fn a_status_outside_the_known_ones_is_refused_by_the_table() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    let error = tx
        .execute(
            "UPDATE sequent_backend.received_ballot SET status = 'counted' WHERE id = $1",
            &[&id],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.as_db_error().unwrap().constraint(),
        Some("received_ballot_status_check")
    );
}

/// A voter's device and the ballot box, with their own keys: the ballot is
/// received with a signed receipt, as `receive_ballot` stores it.
struct SignedBallot {
    ballot_box_sk: StrandSignatureSk,
    voter_sk: StrandSignatureSk,
    received: ReceivedBallot,
}

impl SignedBallot {
    async fn receive(tx: &Transaction<'_>, f: &Election, voter_id: &str) -> Self {
        let ballot_box_sk = StrandSignatureSk::generate().unwrap();
        let voter_sk = StrandSignatureSk::generate().unwrap();
        let received = sign_received_ballot(
            &ballot_box_sk,
            ReceivedStatement {
                tenant_id: f.tenant.to_string(),
                election_event_id: f.event.to_string(),
                election_id: f.election.to_string(),
                ballot_hash: "hash".into(),
                voter_signing_pk: StrandSignaturePk::from_sk(&voter_sk)
                    .unwrap()
                    .to_der_b64_string()
                    .unwrap(),
                voter_ballot_signature: voter_sk.sign(b"ballot").unwrap().to_b64_string().unwrap(),
                received_at: "2028-05-08T03:00:00.007Z".into(),
                key_id: published_key(&ballot_box_sk).unwrap().key_id,
            },
        )
        .unwrap();
        insert_received_ballot(
            tx,
            &f.scope(voter_id, "hash"),
            &f.area,
            "ciphertext",
            &received_at(),
            &received,
        )
        .await
        .unwrap()
        .unwrap();

        Self {
            ballot_box_sk,
            voter_sk,
            received,
        }
    }

    fn cast_request(&self, f: &Election) -> CastBallotInput {
        CastBallotInput {
            election_id: f.election,
            ballot_id: self.received.ballot_id.clone(),
            cast_signature: sign_cast_statement(
                &self.voter_sk,
                &f.election.to_string(),
                &self.received.ballot_id,
            )
            .unwrap(),
        }
    }
}

async fn find(
    tx: &Transaction<'_>,
    f: &Election,
    voter_id: &str,
    request: &CastBallotInput,
) -> Result<FoundCast, CastVoteError> {
    find_received_cast(tx, &f.tenant, &f.event, voter_id, request).await
}

#[tokio::test]
async fn a_signed_cast_is_stored_with_a_receipt_and_a_retry_returns_the_same_receipt() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let ballot = SignedBallot::receive(&tx, &f, "voter").await;
    let request = ballot.cast_request(&f);

    let Ok(FoundCast::ToCast(received_ballot, cast_signature)) =
        find(&tx, &f, "voter", &request).await
    else {
        panic!("a received ballot with its voter's signature is to be cast");
    };
    assert_eq!(cast_signature, request.cast_signature);
    let (input, cast) = received_cast(
        received_ballot,
        cast_signature,
        ballot.ballot_box_sk.clone(),
    )
    .unwrap();
    assert_eq!(input.ballot_id, "hash");
    assert_eq!(input.election_id, f.election);
    assert_eq!(input.content, "ciphertext");

    let (cast_at, receipt) = store_cast_receipt(&tx, &f.tenant, &f.event, &cast)
        .await
        .unwrap();
    verify_cast_receipt(&published_key(&ballot.ballot_box_sk).unwrap(), &receipt).unwrap();
    assert_eq!(receipt.statement.ballot_id, ballot.received.ballot_id);
    assert_eq!(receipt.statement.received_at, "2028-05-08T03:00:00.007Z");
    assert_eq!(receipt.statement.cast_signature, request.cast_signature);
    let cast_vote = cast_vote::insert_cast_vote(
        &tx,
        &f.tenant,
        &f.event,
        &f.election,
        &f.area,
        &input.content,
        "voter",
        &ballot.received.ballot_id,
        &[0; 64],
        &None,
        &None,
        VotingStatusChannel::ONLINE,
        CastVoteStatus::Valid,
        Some(&CastVoteReceipt {
            received_ballot_id: &cast.received_ballot.id,
            cast_at: &cast_at,
            cast_receipt_signature: &receipt.cast_receipt_signature,
        }),
    )
    .await
    .unwrap();

    // The receipt read back from storage is the one that was signed.
    let Ok(FoundCast::AlreadyCast(again)) = find(&tx, &f, "voter", &request).await else {
        panic!("the same cast again is answered with its receipt");
    };
    assert_eq!(again.receipt, receipt);
    assert_eq!(again.cast_vote_id, cast_vote.id);

    // The ballot is cast once, whatever the request.
    assert!(matches!(
        store_cast_receipt(&tx, &f.tenant, &f.event, &cast).await,
        Err(CastVoteError::BallotAlreadyCast)
    ));
}

#[tokio::test]
async fn a_cast_is_refused_for_another_voter_an_unknown_id_and_a_bad_signature() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let ballot = SignedBallot::receive(&tx, &f, "voter").await;
    let request = ballot.cast_request(&f);
    let with = |ballot_id: &str, cast_signature: &str| CastBallotInput {
        election_id: f.election,
        ballot_id: ballot_id.into(),
        cast_signature: cast_signature.into(),
    };
    let another_key = StrandSignatureSk::generate().unwrap();
    let forged = sign_cast_statement(
        &another_key,
        &f.election.to_string(),
        &ballot.received.ballot_id,
    )
    .unwrap();

    assert!(matches!(
        find(&tx, &f, "another-voter", &request).await,
        Err(CastVoteError::BallotNotReceived)
    ));
    for unknown in ["0000-0000", "not a ballot id", ""] {
        assert!(
            matches!(
                find(&tx, &f, "voter", &with(unknown, &request.cast_signature)).await,
                Err(CastVoteError::BallotNotReceived)
            ),
            "{unknown}"
        );
    }
    assert!(matches!(
        find(&tx, &f, "voter", &with(&ballot.received.ballot_id, &forged)).await,
        Err(CastVoteError::BallotCastSignatureFailed(_))
    ));

    // A typed Ballot ID is read the way people write it.
    let typed = ballot.received.ballot_id.replace('-', "").to_lowercase();
    assert!(matches!(
        find(&tx, &f, "voter", &with(&typed, &request.cast_signature)).await,
        Ok(FoundCast::ToCast(..))
    ));
    assert_eq!(
        f.to_cast(&tx, "voter", &ballot.received.ballot_id)
            .await
            .unwrap()
            .cast,
        None
    );
}

#[tokio::test]
async fn an_audited_ballot_and_a_ballot_cast_with_another_signature_are_refused() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let ballot = SignedBallot::receive(&tx, &f, "voter").await;
    let request = ballot.cast_request(&f);
    let id = f
        .to_cast(&tx, "voter", &ballot.received.ballot_id)
        .await
        .unwrap()
        .stored
        .id;

    assert!(
        f.cast(&tx, &id, "another-signature", "another-receipt")
            .await
    );
    assert!(matches!(
        find(&tx, &f, "voter", &request).await,
        Err(CastVoteError::BallotAlreadyCast)
    ));

    tx.execute(
        "UPDATE sequent_backend.received_ballot
         SET status = 'audited', cast_at = NULL, cast_signature = NULL,
             cast_receipt_signature = NULL
         WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    assert!(matches!(
        find(&tx, &f, "voter", &request).await,
        Err(CastVoteError::BallotAudited)
    ));
}

#[tokio::test]
async fn a_receipt_is_not_signed_with_a_key_other_than_the_one_that_received_the_ballot() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let ballot = SignedBallot::receive(&tx, &f, "voter").await;
    let Ok(FoundCast::ToCast(received_ballot, cast_signature)) =
        find(&tx, &f, "voter", &ballot.cast_request(&f)).await
    else {
        panic!("the ballot is to be cast");
    };

    assert!(matches!(
        received_cast(
            received_ballot,
            cast_signature,
            StrandSignatureSk::generate().unwrap()
        ),
        Err(CastVoteError::BallotSignFailed(_))
    ));
}
