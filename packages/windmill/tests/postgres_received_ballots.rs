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
use sequent_core::ballot_receipt::{ReceivedBallot, ReceivedStatement};
use uuid::Uuid;
use windmill::postgres::cast_vote;
use windmill::postgres::received_ballot::{
    get_published_ballot_eml, get_received_ballot, insert_received_ballot,
    mark_received_ballot_cast, ReceivedBallotScope, ReceivedBallotStatus,
};
use windmill::services::cast_votes::CastVoteStatus;

const BALLOT_EML: &str = r#"{"id":"style"}"#;

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

fn received_at() -> DateTime<Utc> {
    Utc.timestamp_millis_opt(1_841_367_600_007).unwrap()
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
async fn a_received_ballot_is_cast_once_and_only_by_its_voter() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let id = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    for (voter, hash) in [("another-voter", "hash"), ("voter", "another-hash")] {
        assert_eq!(
            mark_received_ballot_cast(&tx, &f.scope(voter, hash))
                .await
                .unwrap(),
            None,
            "{voter} {hash}"
        );
    }

    let cast = mark_received_ballot_cast(&tx, &f.scope("voter", "hash"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cast.id, id);
    assert_eq!(cast.status, ReceivedBallotStatus::Cast);
    assert_eq!(cast.received.ballot_id, "FTBE-MHRX");
    assert!(tx
        .query_one(
            "SELECT cast_at IS NOT NULL FROM sequent_backend.received_ballot WHERE id = $1",
            &[&id],
        )
        .await
        .unwrap()
        .get::<_, bool>(0));

    assert_eq!(
        mark_received_ballot_cast(&tx, &f.scope("voter", "hash"))
            .await
            .unwrap(),
        None
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

    assert_eq!(
        mark_received_ballot_cast(&tx, &f.scope("voter", "hash"))
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        get_received_ballot(&tx, &f.scope("voter", "hash"))
            .await
            .unwrap()
            .unwrap()
            .status,
        ReceivedBallotStatus::Audited
    );
}

#[tokio::test]
async fn the_cast_vote_names_the_received_ballot_it_was_cast_from() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Election::create(&tx).await;
    let received = f.receive(&tx, "voter", "hash", "FTBE-MHRX").await.unwrap();

    let mut ids = vec![];
    for (voter, received_ballot_id) in [("voter", Some(&received)), ("another-voter", None)] {
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
            received_ballot_id,
        )
        .await
        .unwrap();
        assert_eq!(cast_vote.ballot_id.as_deref(), Some("FTBE-MHRX"));
        ids.push(Uuid::parse_str(&cast_vote.id).unwrap());
    }

    let stored: Vec<Option<Uuid>> = tx
        .query(
            "SELECT received_ballot_id FROM sequent_backend.cast_vote
             WHERE id = ANY($1) ORDER BY voter_id_string DESC",
            &[&ids],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(stored, vec![Some(received), None]);
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
