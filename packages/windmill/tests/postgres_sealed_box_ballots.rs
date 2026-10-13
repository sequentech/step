// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally of a sealed ballot box (VOTE-FREEZE) against a private migrated
//! Hasura database and the event's own electoral-log database: a box that
//! matches its seal is counted from the manifest and verified once; any
//! difference is rejected on the log and refused.
//!
//! Needs `HASURA_DB__*` and `ELECTORAL_LOG_PG_*` (the stack's
//! `.devcontainer/.env`, or the PostgreSQL service of the CI jobs) and sets
//! its own `MASTER_SECRET`.

#![recursion_limit = "256"]

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use chrono::Utc;
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_seal::{
    insert_seal, BallotBoxId, NewSeal, SealOutcome as StoredOutcome,
};
use electoral_log::adapters::postgres::append_in;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::statement::StatementType;
use electoral_log::ports::ElectoralLogStore;
use electoral_log::seal::{
    ballot_hash, build, BallotBoxSealManifest, BuiltSeal, SealDisposition, SealEntry,
    SEAL_FORMAT_V1,
};
use electoral_log::{Filter, LogEntry, LogQuery, SqlCompOperators, TextColumn};
use event_database::{EventDatabase, Vote};
use sequent_core::ballot::WeightedVotingPolicy;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::participation::ParticipationChannel;
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Once;
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use voting_crypto::{RecordDeserialize, RecordSerialize};
use windmill::postgres::ballot_box_seal::{
    insert_pending, list_for_elections, mark_published, mark_sealed, ClosedBy, NewPendingSeal,
    PublishedFields, SealLogRecord, SealedFields,
};
use windmill::postgres::tally_session::get_tally_session_by_id;
use windmill::postgres::tally_session_contest::insert_tally_session_contest;
use windmill::services::ballot_box_seal::is_ballot_box_sealed;
use windmill::services::ballot_box_seal::seal::seal_delivery_id;
use windmill::services::ceremonies::sealed_box_ballots::{
    check_sealed_boxes_tallied, sealed_box_ballots, sealed_elections, SealedBox, SealedBoxBallots,
    SealedBoxLog,
};
use windmill::services::ceremonies::tally_boards::sealed_boxes_for_execution;
use windmill::services::election::get_election_event_elections;
use windmill::services::electoral_log_board::create_protocol_manager_keys;

const SESSION: &str = "tally-session";
const ELECTION_NAME: &str = "Mayor";
const AREA_NAME: &str = "North";
const REFUSAL: &str = "The ballot box of Mayor, North does not match its seal";

/// The secrets of an event are encrypted with the deployment's master secret.
fn master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

/// One ballot box of a seal-at-close event, with its own electoral-log
/// database.
struct World {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
    database: EventDatabase,
    key: SigningKey,
}

impl World {
    async fn new(tx: &Transaction<'_>, policy: &str) -> World {
        let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
        let w = World {
            tenant,
            event,
            election: Uuid::new_v4(),
            area: Uuid::new_v4(),
            database: EventDatabase::create(tenant, event).await,
            key: SigningKey::generate().unwrap(),
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
        w
    }

    fn board(&self) -> &str {
        &self.database.board
    }

    fn event(&self, policy: &str) -> ElectionEvent {
        serde_json::from_value(json!({
            "id": self.event.to_string(), "tenant_id": self.tenant.to_string(),
            "is_archived": false, "encryption_protocol": "RSA256",
            "presentation": {"ballot_box_seal_policy": policy},
            "bulletin_board_reference":
                {"id": 1, "database_name": self.board(), "is_archived": false},
        }))
        .unwrap()
    }

    /// Drops the test's electoral-log database.
    async fn finish(self) {
        self.database.drop().await;
    }

    fn log(&self) -> SealedBoxLog {
        SealedBoxLog::with_signing_key(
            self.board().to_string(),
            self.database.store.clone(),
            self.key.clone(),
        )
        .unwrap()
    }

    /// Casts `ballot` and appends its record to the board, as the sequencer
    /// does.
    async fn cast(&self, ballot: &Stored) {
        self.accept(ballot).await;
        self.database.sequence(&self.key).await;
    }

    /// Casts `ballot`, leaving it to the sequencer.
    async fn accept(&self, ballot: &Stored) {
        let ballot_id = ballot.ballot_id();
        let vote = Vote {
            election: self.election,
            area: self.area,
            voter: &ballot.voter,
            content: &ballot.content,
            ballot_id: &ballot_id,
            channel: ballot.channel,
        };
        if ballot.discarded {
            self.database.cast_rejected(&vote).await;
        } else {
            self.database.cast(&vote).await;
        }
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
    fn signed(&self, manifest: BallotBoxSealManifest, signer: &SigningKey) -> Message {
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

    /// Seals the box as the sealer does: the seal, with its record on the
    /// board, in the event's database, and the published seal row; returns
    /// its signed message.
    async fn seal(
        &self,
        tx: &Transaction<'_>,
        manifest: BallotBoxSealManifest,
        signer: &SigningKey,
    ) -> Message {
        self.store_seal(tx, manifest, signer, self.board()).await
    }

    /// [`World::seal`], with the seal's record on another board of the
    /// event's database instead of the event's.
    async fn seal_without_record(
        &self,
        tx: &Transaction<'_>,
        manifest: BallotBoxSealManifest,
        signer: &SigningKey,
    ) -> Message {
        let elsewhere = format!("{}elsewhere", self.board());
        self.database.store.create_board(&elsewhere).await.unwrap();
        self.store_seal(tx, manifest, signer, &elsewhere).await
    }

    async fn store_seal(
        &self,
        tx: &Transaction<'_>,
        manifest: BallotBoxSealManifest,
        signer: &SigningKey,
        board: &str,
    ) -> Message {
        let built = build(manifest).unwrap();
        let message = self.signed(built.manifest.clone(), signer);
        let signed_message = message.record_serialize().unwrap();
        let (event, election, area) = (
            self.event.to_string(),
            self.election.to_string(),
            self.area.to_string(),
        );
        let ballot_box = BallotBoxId {
            election_event_id: &event,
            election_id: &election,
            area_id: &area,
        };
        let mut client = self.database.store.client().await.unwrap();
        let event_tx = client.transaction().await.unwrap();
        let stored = append_in(
            &event_tx,
            board,
            &[LogEntry {
                delivery_id: seal_delivery_id(ballot_box),
                message: (&message).try_into().unwrap(),
            }],
        )
        .await
        .unwrap();
        assert!(insert_seal(
            &event_tx,
            &NewSeal {
                ballot_box,
                outcome: &StoredOutcome::Sealed {
                    seal_hash: built.hash.to_vec(),
                    manifest: built.bytes.clone(),
                    ballots_in_box: built.manifest.ballots_in_box() as i64,
                    ballots_counted: built.manifest.ballots_counted() as i64,
                },
                signed_message: &signed_message,
                record: &stored[0],
            },
        )
        .await
        .unwrap());
        event_tx.commit().await.unwrap();
        self.publish_row(tx, &built, signed_message, SealLogRecord::from(&stored[0]))
            .await;
        message
    }

    /// The box's seal row, published with the seal `built`.
    async fn publish_row(
        &self,
        tx: &Transaction<'_>,
        built: &BuiltSeal,
        signed_message: Vec<u8>,
        log_record: SealLogRecord,
    ) {
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
                signed_message,
                log_record,
            },
        )
        .await
        .unwrap();
        mark_published(
            tx,
            &seal.id,
            &PublishedFields {
                public_document_id: Uuid::new_v4(),
                public_path: None,
                published_at: now,
            },
        )
        .await
        .unwrap();
    }

    /// Appends `message` to the board under a delivery ID of its own: a
    /// copy, or an entry anyone who can write the log could add.
    async fn post(&self, message: &Message) {
        self.database
            .store
            .append(
                self.board(),
                &mut std::iter::once(Ok(LogEntry {
                    delivery_id: Uuid::new_v4().to_string(),
                    message: message.try_into().unwrap(),
                })),
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
        let query = LogQuery {
            filters: vec![
                Filter::Text(
                    TextColumn::StatementKind,
                    SqlCompOperators::Equal,
                    kind.to_string(),
                ),
                Filter::Text(
                    TextColumn::AreaId,
                    SqlCompOperators::Equal,
                    self.area.to_string(),
                ),
            ],
            ..LogQuery::default()
        };
        self.database
            .store
            .query(self.board(), &query)
            .await
            .unwrap()
            .iter()
            .map(|row| Message::record_deserialize(&row.message).unwrap())
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
    discarded: bool,
    content: String,
    channel: &'static str,
    ballot_id_override: Option<String>,
}

impl Stored {
    fn new(voter: &str, status: &'static str, content: &str, channel: &'static str) -> Stored {
        Stored {
            voter: voter.into(),
            discarded: status == "discarded",
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
async fn cast_all(w: &World) -> Vec<SealEntry> {
    let mut entries = Vec::new();
    for (stored, disposition, weight) in ballots() {
        w.cast(&stored).await;
        entries.push(stored.entry(disposition, weight));
    }
    entries
}

#[test]
fn a_box_that_matches_its_seal_is_counted_from_the_manifest_and_verified_once() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        let message = w.seal(&tx, w.manifest(entries, 7), &w.key).await;

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
    })
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

#[test]
fn a_tampered_ballot_is_rejected() {
    event_database::run(async {
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
            w.cast(&stored).await;
        }
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        assert_rejected(
        &w,
        &tx,
        "1 sealed ballot is missing (Ballot IDs id-dave); 1 ballot is not in the seal (Ballot IDs id-dave)",
    )
    .await;
        w.finish().await;
    })
}

#[test]
fn a_duplicated_manifest_entry_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let mut entries = cast_all(&w).await;
        entries.push(entries[1].clone());
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        assert_rejected(
            &w,
            &tx,
            "1 sealed ballot is missing (Ballot IDs id-alice-new)",
        )
        .await;
        w.finish().await;
    })
}

#[test]
fn a_ballot_outside_the_seal_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        w.cast(&Stored::new("eve", "valid", "eve", "ONLINE")).await;
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        assert_rejected(&w, &tx, "1 ballot is not in the seal (Ballot IDs id-eve)").await;
        w.finish().await;
    })
}

#[test]
fn a_box_without_its_seal_entry_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        w.seal_without_record(&tx, w.manifest(entries, 4), &w.key)
            .await;
        assert_rejected(
            &w,
            &tx,
            "the bulletin board has no BallotBoxSealed entry signed by the event's key",
        )
        .await;
        w.finish().await;
    })
}

#[test]
fn copies_of_the_seal_entry_and_unsigned_entries_do_not_block_the_tally() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        let manifest = w.manifest(entries, 4);
        let message = w.seal(&tx, manifest.clone(), &w.key).await;
        // Anyone who can write the log could add a byte-identical copy, or an
        // entry for this box signed by another key.
        w.post(&message).await;
        let other = SigningKey::generate().unwrap();
        w.post(&w.signed(manifest, &other)).await;
        let counted = w.count(&tx).await.unwrap();
        assert_eq!(counted.seal_hash, seal_hash_of(&message));
        assert!(w.rejections().await.is_empty());
        w.finish().await;
    })
}

#[test]
fn two_different_signed_seal_entries_are_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        w.seal(&tx, w.manifest(entries.clone(), 4), &w.key).await;
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
    })
}

#[test]
fn a_counted_ballot_with_a_weight_of_zero_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let mut entries = cast_all(&w).await;
        for entry in entries.iter_mut() {
            if entry.ballot_id == "id-dave" {
                entry.weight = 0;
            }
        }
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        assert_rejected(&w, &tx, "weight").await;
        w.finish().await;
    })
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

#[test]
fn an_execution_whose_contest_was_removed_after_creation_is_refused() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
        // The execution signs with the event's protocol manager key.
        tx.execute(
            "UPDATE sequent_backend.election_event SET bulletin_board_reference = $2 WHERE id = $1",
            &[
                &w.event,
                &json!({"id": 1, "database_name": w.board(), "is_archived": false}),
            ],
        )
        .await
        .unwrap();
        master_secret();
        create_protocol_manager_keys(&tx, &tenant, &event, w.board())
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
    })
}

#[test]
fn a_sealed_box_with_ballots_outside_the_tally_is_refused() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
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
    })
}

#[test]
fn a_seal_signed_with_another_key_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        let other = SigningKey::generate().unwrap();
        w.seal(&tx, w.manifest(entries, 4), &other).await;
        assert_rejected(
            &w,
            &tx,
            "the bulletin board has no BallotBoxSealed entry signed by the event's key",
        )
        .await;
        w.finish().await;
    })
}

#[test]
fn a_box_without_a_seal_row_is_an_error_not_a_rejection() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        cast_all(&w).await;
        let error = w.count(&tx).await.unwrap_err();
        assert_eq!(error.to_string(), "The ballot box of North has no seal");
        assert!(w.rejections().await.is_empty());
        w.finish().await;
    })
}

#[test]
fn a_seal_on_the_board_without_its_stored_seal_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        // The seal was deleted from the event's database after it was stored.
        w.post(&w.signed(w.manifest(entries, 4), &w.key)).await;
        assert_rejected(
            &w,
            &tx,
            "the bulletin board holds the seal of this ballot box, but the stored seal is missing",
        )
        .await;
        w.finish().await;
    })
}

#[test]
fn sealed_elections_follow_the_board_when_the_policy_is_off() {
    event_database::run(async {
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
        let entries = cast_all(&w).await;
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
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
    })
}

/// The services' role owns the event's database and can switch its guard
/// off: the tally still finds a ballot changed after the seal.
#[test]
fn a_ballot_changed_past_the_guard_after_the_seal_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        let change = "UPDATE ballot_box_ballot SET content = 'dave-changed'
         WHERE election_event_id = $1::text::uuid AND ballot_id = 'id-dave'";
        let event = w.event.to_string();
        let event_client = w.database.store.client().await.unwrap();
        // The guard refuses the change.
        let refusal = event_client.execute(change, &[&event]).await.unwrap_err();
        assert!(is_ballot_box_sealed(&anyhow::Error::new(refusal)));
        // Past the guard, it is stored.
        event_client
            .batch_execute("SET session_replication_role = replica")
            .await
            .unwrap();
        assert_eq!(event_client.execute(change, &[&event]).await.unwrap(), 1);
        event_client
            .batch_execute("SET session_replication_role = DEFAULT")
            .await
            .unwrap();
        assert_rejected(
        &w,
        &tx,
        "1 sealed ballot is missing (Ballot IDs id-dave); 1 ballot is not in the seal (Ballot IDs id-dave)",
    )
    .await;
        w.finish().await;
    })
}

/// Accepted ballots the sequencer hasn't appended stop the count, as for
/// any box, without a rejection: nothing was found to differ.
#[test]
fn a_box_with_ballots_not_on_the_log_yet_is_an_error_not_a_rejection() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let mut entries = Vec::new();
        for (stored, disposition, weight) in ballots() {
            w.accept(&stored).await;
            entries.push(stored.entry(disposition, weight));
        }
        w.seal(&tx, w.manifest(entries, 4), &w.key).await;
        let error = w.count(&tx).await.unwrap_err().to_string();
        assert!(
            error.ends_with("accepted ballot(s) are not in the electoral log yet"),
            "{error}"
        );
        assert!(w.rejections().await.is_empty());
        w.database.sequence(&w.key).await;
        w.count(&tx).await.unwrap();
        w.finish().await;
    })
}

/// A seal row that says the box is sealed while the event's database has no
/// seal (for example, that database restored to before the seal) is an
/// incident.
#[test]
fn a_seal_row_without_the_stored_seal_is_rejected() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World::new(&tx, "seal-at-close").await;
        let entries = cast_all(&w).await;
        let built = build(w.manifest(entries, 4)).unwrap();
        let message = w.signed(built.manifest.clone(), &w.key);
        w.publish_row(
            &tx,
            &built,
            message.record_serialize().unwrap(),
            SealLogRecord {
                delivery_id: "ballot-box-seal:restored".into(),
                log_uid: Uuid::new_v4(),
                leaf_index: 1,
            },
        )
        .await;
        assert_rejected(
            &w,
            &tx,
            "the ballot box's seal row says it is sealed, but the stored seal is missing",
        )
        .await;
        w.finish().await;
    })
}
