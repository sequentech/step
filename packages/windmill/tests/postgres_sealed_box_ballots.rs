// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally of a sealed ballot box (VOTE-FREEZE) against the stack's
//! PostgreSQL (a private migrated database) and immudb (a board of its own
//! per test): a box that matches its seal is counted from the manifest and
//! verified once; any difference is rejected on the log and refused.
//!
//! Needs `HASURA_DB__*` and `IMMUDB_SERVER_URL`, `IMMUDB_USER`,
//! `IMMUDB_PASSWORD` (from `.devcontainer/.env`, or the immudb service of
//! the CI jobs) and sets its own `MASTER_SECRET`.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use chrono::Utc;
use deadpool_postgres::Transaction;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::statement::StatementType;
use electoral_log::seal::{
    ballot_hash, build, BallotBoxSealManifest, SealDisposition, SealEntry, SEAL_FORMAT_V1,
};
use electoral_log::{
    BoardClient, ElectoralLogMessage, ElectoralLogVarCharColumn, SqlCompOperators,
    WhereClauseBTreeMap,
};
use sequent_core::ballot::WeightedVotingPolicy;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::participation::ParticipationChannel;
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Once;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::{
    insert_pending, list_for_elections, mark_published, mark_sealed, ClosedBy, NewPendingSeal,
    PublishedFields, SealedFields,
};
use windmill::postgres::tally_session::get_tally_session_by_id;
use windmill::postgres::tally_session_contest::insert_tally_session_contest;
use windmill::services::ceremonies::insert_ballots::sealed_boxes_for_execution;
use windmill::services::ceremonies::sealed_box_ballots::{
    check_sealed_boxes_tallied, sealed_box_ballots, sealed_elections, SealedBox, SealedBoxBallots,
    SealedBoxLog,
};
use windmill::services::election::get_election_event_elections;
use windmill::services::protocol_manager::create_protocol_manager_keys;
use windmill::services::protocol_manager::get_immudb_client;

const SESSION: &str = "tally-session";
const ELECTION_NAME: &str = "Mayor";
const AREA_NAME: &str = "North";
const REFUSAL: &str = "The ballot box of Mayor, North does not match its seal";

/// The secrets of an event are encrypted with the deployment's master secret.
fn master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

/// The prefix of the boards these tests create.
const BOARD_PREFIX: &str = "sealedbox";
static BOARDS_SWEPT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// Drops the boards an earlier, interrupted run left behind, once per run
/// and before this run creates any.
async fn sweep_boards() {
    let mut immudb = match get_immudb_client().await {
        Ok(immudb) => immudb,
        Err(error) => return eprintln!("not sweeping {BOARD_PREFIX} boards: {error}"),
    };
    let list = match immudb.list_databases().await {
        Ok(list) => list,
        Err(error) => return eprintln!("not sweeping {BOARD_PREFIX} boards: {error}"),
    };
    let mut removed = 0;
    for database in &list.get_ref().databases {
        if database.name.starts_with(BOARD_PREFIX)
            && immudb.delete_database(&database.name).await.is_ok()
        {
            removed += 1;
        }
    }
    eprintln!("removed {removed} leftover {BOARD_PREFIX} boards");
}

async fn board_client() -> BoardClient {
    let variable =
        |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
    BoardClient::new(
        &variable("IMMUDB_SERVER_URL"),
        &variable("IMMUDB_USER"),
        &variable("IMMUDB_PASSWORD"),
    )
    .await
    .expect("immudb connection")
}

/// One ballot box of a seal-at-close event, with its own electoral log.
struct World {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
    board: String,
    key: StrandSignatureSk,
}

impl World {
    async fn new(tx: &Transaction<'_>, policy: &str) -> World {
        BOARDS_SWEPT.get_or_init(sweep_boards).await;
        let w = World {
            tenant: Uuid::new_v4(),
            event: Uuid::new_v4(),
            election: Uuid::new_v4(),
            area: Uuid::new_v4(),
            board: format!("{BOARD_PREFIX}{}", Uuid::new_v4().simple()),
            key: StrandSignatureSk::generate().unwrap(),
        };
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&w.tenant, &format!("tenant-{}", w.tenant)],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election_event
                 (id, tenant_id, encryption_protocol, status, presentation)
             VALUES ($1, $2, 'RSA256', $3, $4)",
            &[
                &w.event,
                &w.tenant,
                &json!({"is_published": true, "voting_status": "NOT_STARTED"}),
                &json!({ "ballot_box_seal_policy": policy }),
            ],
        )
        .await
        .unwrap();
        tx.execute(
            // Alice votes twice.
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, num_allowed_revotes)
             VALUES ($1, $2, $3, 2)",
            &[&w.election, &w.tenant, &w.event],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, $4)",
            &[&w.area, &w.tenant, &w.event, &AREA_NAME],
        )
        .await
        .unwrap();
        board_client()
            .await
            .upsert_electoral_log_db(&w.board)
            .await
            .unwrap();
        w
    }

    fn event(&self, policy: &str) -> ElectionEvent {
        serde_json::from_value(json!({
            "id": self.event.to_string(), "tenant_id": self.tenant.to_string(),
            "is_archived": false, "encryption_protocol": "RSA256",
            "presentation": {"ballot_box_seal_policy": policy},
            "bulletin_board_reference":
                {"id": 1, "database_name": self.board, "is_archived": false},
        }))
        .unwrap()
    }

    /// Drops the test's board.
    async fn finish(self) {
        if let Err(error) = board_client().await.delete_database(&self.board).await {
            eprintln!("dropping board {}: {error}", self.board);
        }
    }

    fn log(&self) -> SealedBoxLog {
        SealedBoxLog::with_signing_key(self.board.clone(), self.key.clone()).unwrap()
    }

    async fn cast(&self, tx: &Transaction<'_>, ballot: &Stored) {
        tx.execute(
            "INSERT INTO sequent_backend.cast_vote
                 (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                  status, content, ballot_id, annotations)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &[
                &self.tenant,
                &self.event,
                &self.election,
                &self.area,
                &ballot.voter,
                &ballot.status,
                &ballot.content,
                &ballot.ballot_id(),
                &json!({"voting_channel": ballot.channel}),
            ],
        )
        .await
        .unwrap();
    }

    fn manifest(&self, entries: Vec<SealEntry>, eligible_voters: u64) -> BallotBoxSealManifest {
        BallotBoxSealManifest {
            format: SEAL_FORMAT_V1.into(),
            tenant_id: self.tenant.to_string(),
            election_event_id: self.event.to_string(),
            election_id: self.election.to_string(),
            area_id: self.area.to_string(),
            closed_at: 1_800_000_000,
            grace_deadline: 1_800_000_900,
            sealed_at: 1_800_000_960,
            close_request_id: None,
            eligible_voters,
            entries,
        }
    }

    /// The signed `BallotBoxSealed` message of `manifest`.
    fn signed(&self, manifest: BallotBoxSealManifest, signer: &StrandSignatureSk) -> Message {
        let built = build(manifest).unwrap();
        Message::ballot_box_sealed_message(
            &built.manifest,
            built.hash,
            ELECTION_NAME,
            AREA_NAME,
            &SigningData::new(signer.clone(), "", signer.clone()),
        )
        .unwrap()
    }

    /// Stores the seal row (published) and returns its signed message.
    async fn seal(
        &self,
        tx: &Transaction<'_>,
        manifest: BallotBoxSealManifest,
        signer: &StrandSignatureSk,
    ) -> Message {
        let built = build(manifest).unwrap();
        let message = self.signed(built.manifest.clone(), signer);
        let now = Utc::now();
        insert_pending(
            tx,
            &[NewPendingSeal {
                tenant_id: self.tenant,
                election_event_id: self.event,
                election_id: self.election,
                area_id: self.area,
                closed_at: now,
                grace_deadline: now,
                close_request_id: None,
                closed_by: ClosedBy::Scheduled,
            }],
        )
        .await
        .unwrap();
        let seal = list_for_elections(tx, &self.tenant, &self.event, &[self.election])
            .await
            .unwrap()
            .remove(0);
        mark_sealed(
            tx,
            &seal.id,
            &SealedFields {
                sealed_at: now,
                ballots_in_box: built.manifest.ballots_in_box() as i64,
                ballots_counted: built.manifest.ballots_counted() as i64,
                seal_hash: hex::encode(built.hash),
                manifest: built.bytes.clone(),
                signed_message: strand::serialization::StrandSerialize::strand_serialize(&message)
                    .unwrap(),
            },
        )
        .await
        .unwrap();
        mark_published(
            tx,
            &seal.id,
            &PublishedFields {
                log_entry_id: 1,
                public_document_id: Uuid::new_v4(),
                public_path: None,
                published_at: now,
            },
        )
        .await
        .unwrap();
        message
    }

    async fn post(&self, message: &Message) {
        board_client()
            .await
            .insert_electoral_log_messages(
                &self.board,
                &vec![ElectoralLogMessage::try_from(message).unwrap()],
            )
            .await
            .unwrap();
    }

    async fn count(&self, tx: &Transaction<'_>) -> anyhow::Result<SealedBoxBallots> {
        let (tenant, event, election, area) = (
            self.tenant.to_string(),
            self.event.to_string(),
            self.election.to_string(),
            self.area.to_string(),
        );
        sealed_box_ballots(
            tx,
            &self.log(),
            &SealedBox {
                tenant_id: &tenant,
                election_event_id: &event,
                election_id: &election,
                area_id: &area,
                election_name: ELECTION_NAME,
                area_name: AREA_NAME,
                tally_session_id: SESSION,
            },
            WeightedVotingPolicy::default(),
        )
        .await
    }

    /// The tally's entries of `kind` for this box.
    async fn entries(&self, kind: StatementType) -> Vec<Message> {
        let filter: WhereClauseBTreeMap = [
            (
                ElectoralLogVarCharColumn::StatementKind,
                (SqlCompOperators::Equal, kind.to_string()),
            ),
            (
                ElectoralLogVarCharColumn::AreaId,
                (SqlCompOperators::Equal, self.area.to_string()),
            ),
        ]
        .into_iter()
        .collect();
        board_client()
            .await
            .get_electoral_log_messages_filtered::<String, String>(
                &self.board,
                Some(filter),
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap()
            .iter()
            .map(|row| Message::strand_deserialize(&row.message).unwrap())
            .collect()
    }

    async fn rejections(&self) -> Vec<String> {
        self.entries(StatementType::TallyBallotBoxRejected)
            .await
            .into_iter()
            .map(|message| message.statement.head.description)
            .collect()
    }
}

/// A stored ballot.
struct Stored {
    voter: String,
    status: &'static str,
    content: String,
    channel: &'static str,
    ballot_id_override: Option<String>,
}

impl Stored {
    fn new(voter: &str, status: &'static str, content: &str, channel: &'static str) -> Stored {
        Stored {
            voter: voter.into(),
            status,
            content: content.into(),
            channel,
            ballot_id_override: None,
        }
    }

    fn ballot_id(&self) -> String {
        self.ballot_id_override
            .clone()
            .unwrap_or_else(|| format!("id-{}", self.content))
    }

    fn entry(&self, disposition: SealDisposition, weight: u64) -> SealEntry {
        SealEntry {
            ballot_hash: ballot_hash(&self.content).unwrap(),
            ballot_id: self.ballot_id(),
            disposition,
            weight,
            channel: self.channel.into(),
        }
    }
}

/// A box with every disposition: alice voted twice, bob is not in the
/// census, carol's ballot was discarded.
fn ballots() -> Vec<(Stored, SealDisposition, u64)> {
    vec![
        (
            Stored::new("alice", "valid", "alice-old", "ONLINE"),
            SealDisposition::Replaced,
            0,
        ),
        (
            Stored::new("alice", "valid", "alice-new", "KIOSK"),
            SealDisposition::Counted,
            1,
        ),
        (
            Stored::new("bob", "valid", "bob", "ONLINE"),
            SealDisposition::NotEligible,
            0,
        ),
        (
            Stored::new("carol", "discarded", "carol", "ONLINE"),
            SealDisposition::Discarded,
            0,
        ),
        (
            Stored::new("dave", "valid", "dave", "ONLINE"),
            SealDisposition::Counted,
            1,
        ),
    ]
}

/// Casts every ballot and returns the manifest entries that match them.
async fn cast_all(w: &World, tx: &Transaction<'_>) -> Vec<SealEntry> {
    let mut entries = Vec::new();
    for (stored, disposition, weight) in ballots() {
        w.cast(tx, &stored).await;
        entries.push(stored.entry(disposition, weight));
    }
    entries
}

#[tokio::test]
async fn a_box_that_matches_its_seal_is_counted_from_the_manifest_and_verified_once() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    let message = w.seal(&tx, w.manifest(entries, 7), &w.key).await;
    w.post(&message).await;

    let counted = w.count(&tx).await.unwrap();
    let mut contents = counted.merge_result.ballot_contents.clone();
    contents.sort();
    assert_eq!(
        contents,
        vec![("alice-new".to_string(), 1), ("dave".to_string(), 1)]
    );
    // Eligible voters come from the manifest: no census is read.
    assert_eq!(counted.merge_result.eligible_voters, 7);
    assert_eq!(counted.merge_result.ballots_without_voter, 1);
    assert_eq!(counted.merge_result.casted_ballots, 3);
    assert_eq!(
        counted.merge_result.casted_ballots_by_channel,
        [
            (ParticipationChannel::from("KIOSK"), 1),
            (ParticipationChannel::from("ONLINE"), 2),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(counted.seal_hash, seal_hash_of(&message));

    // A second contest of the same box (SINGLE_CONTEST) verifies it again
    // but posts nothing new.
    w.count(&tx).await.unwrap();
    let verified = w.entries(StatementType::TallyBallotBoxVerified).await;
    assert_eq!(verified.len(), 1);
    assert_eq!(
        verified[0].statement.head.description,
        "Ballot box of Mayor, North matches its seal: 2 ballots counted."
    );
    assert!(w.rejections().await.is_empty());
    w.finish().await;
}

/// The hex seal hash a `BallotBoxSealed` message signs.
fn seal_hash_of(message: &Message) -> String {
    match &message.statement.body {
        electoral_log::messages::statement::StatementBody::BallotBoxSealed(_, _, hash, ..) => {
            hash.to_hex()
        }
        other => panic!("not a seal: {other:?}"),
    }
}

/// Seals `entries`, posts the seal, and expects the count to be refused
/// with one rejection whose description contains `differs`.
async fn assert_rejected(w: &World, tx: &Transaction<'_>, differs: &str) {
    let error = w.count(tx).await.unwrap_err();
    assert_eq!(error.to_string(), REFUSAL);
    let rejections = w.rejections().await;
    assert_eq!(rejections.len(), 1, "{rejections:?}");
    assert!(rejections[0].contains(differs), "{}", rejections[0]);
    assert!(w
        .entries(StatementType::TallyBallotBoxVerified)
        .await
        .is_empty());
    // Running the tally again finds the same difference and posts nothing.
    assert!(w.count(tx).await.is_err());
    assert_eq!(w.rejections().await.len(), 1);
}

#[tokio::test]
async fn a_tampered_ballot_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let mut entries = Vec::new();
    for (mut stored, disposition, weight) in ballots() {
        entries.push(stored.entry(disposition, weight));
        if stored.content == "dave" {
            // The stored content changes; its Ballot ID stays.
            stored.content = "dave-tampered".into();
            stored.ballot_id_override = Some("id-dave".into());
        }
        w.cast(&tx, &stored).await;
    }
    let message = w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    w.post(&message).await;
    assert_rejected(
        &w,
        &tx,
        "1 sealed ballot is missing (Ballot IDs id-dave); 1 ballot is not in the seal (Ballot IDs id-dave)",
    )
    .await;
    w.finish().await;
}

#[tokio::test]
async fn a_duplicated_manifest_entry_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let mut entries = cast_all(&w, &tx).await;
    entries.push(entries[1].clone());
    let message = w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    w.post(&message).await;
    assert_rejected(
        &w,
        &tx,
        "1 sealed ballot is missing (Ballot IDs id-alice-new)",
    )
    .await;
    w.finish().await;
}

#[tokio::test]
async fn a_ballot_outside_the_seal_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    w.cast(&tx, &Stored::new("eve", "valid", "eve", "ONLINE"))
        .await;
    let message = w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    w.post(&message).await;
    assert_rejected(&w, &tx, "1 ballot is not in the seal (Ballot IDs id-eve)").await;
    w.finish().await;
}

#[tokio::test]
async fn a_box_without_its_seal_entry_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    assert_rejected(
        &w,
        &tx,
        "the bulletin board has no BallotBoxSealed entry signed by the event's key",
    )
    .await;
    w.finish().await;
}

#[tokio::test]
async fn copies_of_the_seal_entry_and_unsigned_entries_do_not_block_the_tally() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    let manifest = w.manifest(entries, 4);
    let message = w.seal(&tx, manifest.clone(), &w.key).await;
    // Anyone who can write the log could add a byte-identical copy, or an
    // entry for this box signed by another key.
    w.post(&message).await;
    w.post(&message).await;
    let other = StrandSignatureSk::generate().unwrap();
    w.post(&w.signed(manifest, &other)).await;
    let counted = w.count(&tx).await.unwrap();
    assert_eq!(counted.seal_hash, seal_hash_of(&message));
    assert!(w.rejections().await.is_empty());
    w.finish().await;
}

#[tokio::test]
async fn two_different_signed_seal_entries_are_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    let message = w.seal(&tx, w.manifest(entries.clone(), 4), &w.key).await;
    w.post(&message).await;
    // A second seal of the same box, signed with the event's key: a re-seal
    // or a key compromise.
    w.post(&w.signed(w.manifest(entries, 5), &w.key)).await;
    assert_rejected(
        &w,
        &tx,
        "the bulletin board has 2 different BallotBoxSealed entries signed by the event's key",
    )
    .await;
    w.finish().await;
}

#[tokio::test]
async fn a_counted_ballot_with_a_weight_of_zero_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let mut entries = cast_all(&w, &tx).await;
    for entry in entries.iter_mut() {
        if entry.ballot_id == "id-dave" {
            entry.weight = 0;
        }
    }
    let message = w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    w.post(&message).await;
    assert_rejected(&w, &tx, "weight").await;
    w.finish().await;
}

/// A tally session of the world's election, without contests; returns its
/// id.
async fn create_session(tx: &Transaction<'_>, w: &World) -> String {
    let session = Uuid::new_v4();
    let keys_ceremony = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&keys_ceremony, &w.tenant, &w.event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tally_session
             (id, tenant_id, election_event_id, election_ids, keys_ceremony_id, threshold)
         VALUES ($1, $2, $3, $4, $5, 1)",
        &[
            &session,
            &w.tenant,
            &w.event,
            &vec![w.election],
            &keys_ceremony,
        ],
    )
    .await
    .unwrap();
    session.to_string()
}

#[tokio::test]
async fn an_execution_whose_contest_was_removed_after_creation_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    // The execution signs with the event's protocol manager key.
    tx.execute(
        "UPDATE sequent_backend.election_event SET bulletin_board_reference = $2 WHERE id = $1",
        &[
            &w.event,
            &json!({"id": 1, "database_name": w.board, "is_archived": false}),
        ],
    )
    .await
    .unwrap();
    master_secret();
    create_protocol_manager_keys(&tx, &tenant, &event, &w.board)
        .await
        .unwrap();
    let session = create_session(&tx, &w).await;
    let contest = insert_tally_session_contest(
        &tx,
        &tenant,
        &event,
        &w.area.to_string(),
        None,
        1,
        &session,
        &w.election.to_string(),
    )
    .await
    .unwrap();
    let elections = get_election_event_elections(&tx, &tenant, &event)
        .await
        .unwrap();
    let tally_session = get_tally_session_by_id(&tx, &tenant, &event, &session)
        .await
        .unwrap();
    // At creation the box is covered.
    assert!(
        sealed_boxes_for_execution(&tx, &tenant, &event, &tally_session, &elections)
            .await
            .unwrap()
            .is_some()
    );
    // The contest goes away before the execution: the execution is refused
    // instead of leaving the box's ballots out.
    tx.execute(
        "DELETE FROM sequent_backend.tally_session_contest WHERE id = $1",
        &[&Uuid::parse_str(&contest.id).unwrap()],
    )
    .await
    .unwrap();
    let error = sealed_boxes_for_execution(&tx, &tenant, &event, &tally_session, &elections)
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.starts_with("The sealed ballot box of ")
            && error.ends_with(
                ", North is not in this tally: its area no longer has the election's contests."
            ),
        "{error}"
    );
    // An initialization report never uses the seals.
    let mut report = tally_session.clone();
    report.tally_type = Some("INITIALIZATION_REPORT".into());
    assert!(
        sealed_boxes_for_execution(&tx, &tenant, &event, &report, &elections)
            .await
            .unwrap()
            .is_none()
    );
    w.finish().await;
}

#[tokio::test]
async fn a_sealed_box_with_ballots_outside_the_tally_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    let session = create_session(&tx, &w).await;
    let (tenant, event, election, area) = (
        w.tenant.to_string(),
        w.event.to_string(),
        w.election.to_string(),
        w.area.to_string(),
    );
    let elections = HashSet::from([election.clone()]);
    let election_names = HashMap::from([(election.clone(), ELECTION_NAME.to_string())]);
    let area_names = HashMap::from([(area.clone(), AREA_NAME.to_string())]);
    let check = || {
        check_sealed_boxes_tallied(
            &tx,
            &tenant,
            &event,
            &elections,
            &session,
            &election_names,
            &area_names,
        )
    };
    // The area of the sealed box has no contest of the session (unlinked
    // after the seal): refused, naming the box.
    assert_eq!(
        check().await.unwrap_err().to_string(),
        "The sealed ballot box of Mayor, North is not in this tally: its area no longer has the \
         election's contests."
    );
    insert_tally_session_contest(&tx, &tenant, &event, &area, None, 1, &session, &election)
        .await
        .unwrap();
    check().await.unwrap();
    w.finish().await;
}

#[tokio::test]
async fn a_seal_signed_with_another_key_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    let other = StrandSignatureSk::generate().unwrap();
    let message = w.seal(&tx, w.manifest(entries, 4), &other).await;
    w.post(&message).await;
    assert_rejected(
        &w,
        &tx,
        "the bulletin board has no BallotBoxSealed entry signed by the event's key",
    )
    .await;
    w.finish().await;
}

#[tokio::test]
async fn a_box_without_a_seal_row_is_an_error_not_a_rejection() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    cast_all(&w, &tx).await;
    let error = w.count(&tx).await.unwrap_err();
    assert_eq!(error.to_string(), "The ballot box of North has no seal");
    assert!(w.rejections().await.is_empty());
    w.finish().await;
}

#[tokio::test]
async fn a_seal_on_the_board_without_its_stored_seal_is_rejected() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "seal-at-close").await;
    let entries = cast_all(&w, &tx).await;
    // The seal row was deleted (or never restored) after the seal was posted.
    w.post(&w.signed(w.manifest(entries, 4), &w.key)).await;
    assert_rejected(
        &w,
        &tx,
        "the bulletin board holds the seal of this ballot box, but the stored seal is missing",
    )
    .await;
    w.finish().await;
}

#[tokio::test]
async fn sealed_elections_follow_the_board_when_the_policy_is_off() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "do-not-seal").await;
    let elections = HashSet::from([w.election.to_string()]);
    let off = w.event("do-not-seal");
    // Nothing sealed: the old path.
    assert!(
        sealed_elections(&off, &TallyType::ELECTORAL_RESULTS, &elections)
            .await
            .unwrap()
            .is_empty()
    );
    // The policy was turned off after the seal: the board still says sealed.
    let entries = cast_all(&w, &tx).await;
    let message = w.seal(&tx, w.manifest(entries, 4), &w.key).await;
    w.post(&message).await;
    assert_eq!(
        sealed_elections(&off, &TallyType::ELECTORAL_RESULTS, &elections)
            .await
            .unwrap(),
        elections
    );
    // An initialization report never uses the seals.
    assert!(
        sealed_elections(&off, &TallyType::INITIALIZATION_REPORT, &elections)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(sealed_elections(
        &w.event("seal-at-close"),
        &TallyType::INITIALIZATION_REPORT,
        &elections
    )
    .await
    .unwrap()
    .is_empty());
    w.finish().await;
}
