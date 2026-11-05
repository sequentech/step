// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Receiving and casting ballots with signed receipts, on the migrated schema
//! and the event's ballot box. Every test writes its Hasura rows in a
//! transaction and rolls it back; its election events' ballot boxes are its own.

#[path = "support/ballot_box.rs"]
mod ballot_box;
#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, AcceptReceived, BallotStatus,
};
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedSignature,
};
use electoral_log::adapters::postgres::PostgresStore;
use sequent_core::ballot_receipt::{
    sign_cast_statement, sign_received_ballot, verify_cast_receipt, ReceivedBallot,
    ReceivedStatement,
};
use uuid::Uuid;
use voting_crypto::signing::{SigningKey, VerificationKey};
use windmill::postgres::ballot_style::get_published_ballot_eml;
use windmill::services::ballot_box::BallotFormat;
use windmill::services::ballot_box_key::published_key;
use windmill::services::ballot_checks::{locate_ballot, LocateBallotStatus};
use windmill::services::cast_ballot::{
    find_received_cast, received_cast, sign_receipt, CastBallotInput, FoundCast,
};
use windmill::services::insert_cast_vote::CastVoteError;
use windmill::services::receive_ballot::format_signed_time;

const BALLOT_EML: &str = r#"{"id":"style"}"#;

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

fn received_at() -> DateTime<Utc> {
    Utc.timestamp_millis_opt(1_841_367_600_007).unwrap()
}

/// One election of one area in a new tenant and election event, with its
/// ballot box.
struct Election {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
}

impl Election {
    async fn create(tx: &Transaction<'_>) -> Self {
        let fixture = Self {
            tenant: Uuid::new_v4(),
            event: Uuid::new_v4(),
            election: Uuid::new_v4(),
            area: Uuid::new_v4(),
        };
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&fixture.tenant, &format!("tenant-{}", fixture.tenant)],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election_event
                 (id, tenant_id, encryption_protocol, bulletin_board_reference)
             VALUES ($1, $2, 'RSA256', $3)",
            &[
                &fixture.event,
                &fixture.tenant,
                &ballot_box::board_reference(&format!("board-{}", fixture.event.simple())),
            ],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&fixture.election, &fixture.tenant, &fixture.event],
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

    async fn ballot_box(&self) -> PostgresStore {
        ballot_box::ballot_box(&self.tenant, &self.event).await
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
        // Published rows are written by the publication workflow only.
        windmill::postgres::trusted_write(tx).await.unwrap();
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
        tx.execute(
            "SELECT set_config('sequent.trusted_write', 'off', true)",
            &[],
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

/// A stored ballot that names no voter key and no ballot style that matters here.
const ENVELOPE: &str = r#"{"config":"config","ballot_style_hash":"hash"}"#;

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

/// A voter's device and the ballot box, with their own keys: the ballot is
/// received with a signed receipt, as `receive_ballot` stores it.
struct SignedBallot {
    ballot_box_sk: SigningKey,
    voter_sk: SigningKey,
    received: ReceivedBallot,
}

impl SignedBallot {
    async fn receive(store: &PostgresStore, f: &Election, voter_id: &str, hash: &str) -> Self {
        let ballot_box_sk = SigningKey::generate().unwrap();
        let voter_sk = SigningKey::generate().unwrap();
        let statement = ReceivedStatement {
            tenant_id: f.tenant.to_string(),
            election_event_id: f.event.to_string(),
            election_id: f.election.to_string(),
            ballot_hash: hash.into(),
            voter_signing_pk: VerificationKey::from_sk(&voter_sk)
                .unwrap()
                .to_b64_string()
                .unwrap(),
            voter_ballot_signature: voter_sk.sign(b"ballot").unwrap().to_b64_string().unwrap(),
            received_at: format_signed_time(&received_at()),
            key_id: published_key(&ballot_box_sk).unwrap().key_id,
        };
        let received = sign_received_ballot(&ballot_box_sk, statement.clone()).unwrap();
        let (event, election, area) = (
            f.event.to_string(),
            f.election.to_string(),
            f.area.to_string(),
        );
        let outcome = store
            .receive_ballot(
                &ReceiveBallot {
                    election_event_id: &event,
                    election_id: &election,
                    area_id: &area,
                    voter_id,
                    ballot_hash: hash,
                    content: ENVELOPE,
                    voter_signing_pk: &statement.voter_signing_pk,
                    voter_ballot_signature: &statement.voter_ballot_signature,
            vitrine: None,
        },
                received_at(),
                |_| {
                    Ok(ReceivedSignature {
                        ballot_id: received.ballot_id.clone(),
                        key_id: received.statement.key_id.clone(),
                        received_signature: received.received_signature.clone(),
                    })
                },
            )
            .await
            .unwrap();
        assert!(
            matches!(outcome, ReceiveOutcome::Received(_)),
            "{outcome:?}"
        );

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
    store: &PostgresStore,
    f: &Election,
    voter_id: &str,
    request: &CastBallotInput,
) -> Result<FoundCast, CastVoteError> {
    find_received_cast(store, &f.event.to_string(), voter_id, request).await
}

/// Cast the voter's received ballot with the hash and content it was received
/// with, as the cast path does.
async fn cast(
    store: &PostgresStore,
    f: &Election,
    voter_id: &str,
    received: &ReceivedBallot,
    cast_at: DateTime<Utc>,
    cast_signature: &str,
    cast_receipt_signature: &str,
) -> AcceptOutcome {
    let (event, election, area) = (
        f.event.to_string(),
        f.election.to_string(),
        f.area.to_string(),
    );
    let format = BallotFormat::HashableBallot.to_string();
    store
        .accept_received_ballot(
            &AcceptBallot {
                election_event_id: &event,
                election_id: &election,
                area_id: &area,
                voter_id,
                ballot_id: &received.statement.ballot_hash,
                format: &format,
                content: ENVELOPE,
                voter_signature: None,
                pseudonym_hash: &[0; 64],
                ballot_hash: &[0; 64],
                voting_channel: "ONLINE",
                status: BallotStatus::Valid,
                voter_ip: None,
                voter_country: None,
                username: None,
                allowed_votes: 0,
                vitrine: None,
            },
            &AcceptReceived {
                ballot_id: &received.ballot_id,
                cast_at,
                cast_signature,
                cast_receipt_signature,
            },
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn a_signed_cast_is_stored_with_a_receipt_and_a_retry_returns_the_same_receipt() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let store = f.ballot_box().await;
    let ballot = SignedBallot::receive(&store, &f, "voter", "0abc12").await;
    let request = ballot.cast_request(&f);

    let Ok(FoundCast::ToCast(received_ballot, cast_signature)) =
        find(&store, &f, "voter", &request).await
    else {
        panic!("a received ballot with its voter's signature is to be cast");
    };
    assert_eq!(cast_signature, request.cast_signature);
    let (input, received) = received_cast(
        received_ballot,
        cast_signature,
        ballot.ballot_box_sk.clone(),
    )
    .unwrap();
    assert_eq!(input.ballot_id, "0abc12");
    assert_eq!(input.election_id, f.election);
    assert_eq!(input.content, ENVELOPE);

    let (cast_at, receipt) = sign_receipt(&received).unwrap();
    verify_cast_receipt(&published_key(&ballot.ballot_box_sk).unwrap(), &receipt).unwrap();
    assert_eq!(receipt.statement.ballot_id, ballot.received.ballot_id);
    assert_eq!(receipt.statement.received_at, "2028-05-08T03:00:00.007Z");
    assert_eq!(receipt.statement.cast_signature, request.cast_signature);
    let AcceptOutcome::Accepted { id, .. } = cast(
        &store,
        &f,
        "voter",
        &ballot.received,
        cast_at,
        &request.cast_signature,
        &receipt.cast_receipt_signature,
    )
    .await
    else {
        panic!("the received ballot is cast");
    };

    // The receipt read back from storage is the one that was signed.
    let Ok(FoundCast::AlreadyCast(again)) = find(&store, &f, "voter", &request).await else {
        panic!("the same cast again is answered with its receipt");
    };
    assert_eq!(again.receipt, receipt);
    assert_eq!(again.cast_vote_id, id);

    // The ballot is cast once, whatever the request.
    assert_eq!(
        cast(
            &store,
            &f,
            "voter",
            &ballot.received,
            cast_at,
            &request.cast_signature,
            &receipt.cast_receipt_signature,
        )
        .await,
        AcceptOutcome::DuplicateBallotId
    );

    // The voter finds the cast ballot by its Tracking ID, which the ballot hash and the
    // key in the stored ballot give, not by the key of its receipt.
    let tracking_id = sequent_core::ballot_receipt::tracking_id("0abc12", None).unwrap();
    let located = locate_ballot(
        &tx,
        &f.tenant.to_string(),
        &f.event.to_string(),
        &f.election.to_string(),
        &f.area.to_string(),
        "voter",
        &tracking_id.to_lowercase(),
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!(located.status, LocateBallotStatus::Found);
    assert_eq!(located.ballot_id, Some(tracking_id));
    assert_eq!(located.content.as_deref(), Some(ENVELOPE));
    assert_eq!(
        located
            .cast_at
            .map(|at| DateTime::parse_from_rfc3339(&at).unwrap()),
        Some(cast_at.fixed_offset())
    );
}

#[tokio::test]
async fn a_cast_is_refused_for_another_voter_an_unknown_id_and_a_bad_signature() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let store = f.ballot_box().await;
    let ballot = SignedBallot::receive(&store, &f, "voter", "0abc12").await;
    let request = ballot.cast_request(&f);
    let with = |ballot_id: &str, cast_signature: &str| CastBallotInput {
        election_id: f.election,
        ballot_id: ballot_id.into(),
        cast_signature: cast_signature.into(),
    };
    let another_key = SigningKey::generate().unwrap();
    let forged = sign_cast_statement(
        &another_key,
        &f.election.to_string(),
        &ballot.received.ballot_id,
    )
    .unwrap();

    assert!(matches!(
        find(&store, &f, "another-voter", &request).await,
        Err(CastVoteError::BallotNotReceived)
    ));
    for unknown in ["0000-0000", "not a ballot id", ""] {
        assert!(
            matches!(
                find(&store, &f, "voter", &with(unknown, &request.cast_signature)).await,
                Err(CastVoteError::BallotNotReceived)
            ),
            "{unknown}"
        );
    }
    assert!(matches!(
        find(
            &store,
            &f,
            "voter",
            &with(&ballot.received.ballot_id, &forged)
        )
        .await,
        Err(CastVoteError::BallotCastSignatureFailed(_))
    ));

    // A typed Ballot ID is read the way people write it.
    let typed = ballot.received.ballot_id.replace('-', "").to_lowercase();
    let Ok(FoundCast::ToCast(received, _)) =
        find(&store, &f, "voter", &with(&typed, &request.cast_signature)).await
    else {
        panic!("the typed Ballot ID names the received ballot");
    };
    assert_eq!(received.cast_at, None);
}

#[tokio::test]
async fn an_audited_ballot_and_a_ballot_cast_with_another_signature_are_refused() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let store = f.ballot_box().await;
    let cast_ballot = SignedBallot::receive(&store, &f, "voter", "0abc12").await;
    let audited = SignedBallot::receive(&store, &f, "voter", "0abc34").await;

    assert!(matches!(
        cast(
            &store,
            &f,
            "voter",
            &cast_ballot.received,
            Utc::now(),
            "another-signature",
            "another-receipt",
        )
        .await,
        AcceptOutcome::Accepted { .. }
    ));
    assert!(matches!(
        find(&store, &f, "voter", &cast_ballot.cast_request(&f)).await,
        Err(CastVoteError::BallotAlreadyCast)
    ));

    store
        .client()
        .await
        .unwrap()
        .execute(
            "UPDATE ballot_box_received SET status = 'audited'
             WHERE election_event_id = $1::text::uuid AND ballot_id = $2",
            &[&f.event.to_string(), &audited.received.ballot_id],
        )
        .await
        .unwrap();
    assert!(matches!(
        find(&store, &f, "voter", &audited.cast_request(&f)).await,
        Err(CastVoteError::BallotAudited)
    ));
}

#[tokio::test]
async fn a_receipt_is_not_signed_with_a_key_other_than_the_one_that_received_the_ballot() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let store = f.ballot_box().await;
    let ballot = SignedBallot::receive(&store, &f, "voter", "0abc12").await;
    let Ok(FoundCast::ToCast(received_ballot, cast_signature)) =
        find(&store, &f, "voter", &ballot.cast_request(&f)).await
    else {
        panic!("the ballot is to be cast");
    };

    assert!(matches!(
        received_cast(
            received_ballot,
            cast_signature,
            SigningKey::generate().unwrap()
        ),
        Err(CastVoteError::BallotSignFailed(_))
    ));
}
