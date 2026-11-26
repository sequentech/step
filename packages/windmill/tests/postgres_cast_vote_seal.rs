// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! HSM outer encryption of stored cast votes, end to end on a private
//! migrated Hasura database and each event's own electoral-log database,
//! through the real receive, cast, sequencer and tally-dump services:
//!
//! - a database without the state function reads events as legacy;
//! - requiring sealing and opening voting wait for each other;
//! - the publish function's role checks and its refusals (another key, an
//!   event that opened voting without requiring sealing);
//! - a sealed event stores every copy sealed (received and cast), with the
//!   inner Ballot ID; a legacy event stores plaintext exactly as before;
//! - a sealed event without its key, or whose key is not pinned in its
//!   electoral log, refuses the cast;
//! - the voter's own reads never return a sealed ballot;
//! - the tally dump refuses without an opener and, with one (a software
//!   stand-in for the HSM), gives the voters' ballots as the legacy dump
//!   does.
//!
//! Needs `HASURA_DB__*`, `ELECTORAL_LOG_PG_*`, `PGMQ_TEST_DATABASE_URL` and
//! `ENV_SLUG`; sets its own `MASTER_SECRET` and points the services' pools at
//! the private database. One test: the opener is installed once per process.

#![recursion_limit = "256"]
#[path = "support/cast_vote_opener.rs"]
mod cast_vote_opener;
#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use cast_vote_opener::{Openers, SoftwareOpener};
use chrono::Utc;
use deadpool_postgres::Pool;
use electoral_log::adapters::ballot_box_reads::BallotIdMatch;
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::cast_vote_seal::SealedBallot;
use electoral_log::ports::ElectoralLogStore;
use sequent_core::ballot::{
    BallotStyle, HashableBallot, SignedHashableBallot, VotingStatus, VotingStatusChannel,
};
use sequent_core::ballot_receipt::sign_cast_statement;
use sequent_core::fixtures::ballot_codec::{
    get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use windmill::services::ballot_box_reads::{get_voter_cast_votes, VoterCastVotes, VoterScope};
use windmill::services::cast_vote_seal::{self, SealKey};
use windmill::services::election_event_status::{
    change_election_voting_status, get_election_status, VotingTransitionError,
};
use windmill::services::insert_cast_vote::{
    try_insert_cast_vote, CastVoteError, InsertCastVoteInput, InsertCastVoteResult,
};
use windmill::services::{
    ballot_box, ballot_box_key, cast_ballot, cast_votes, database::get_hasura_pool,
    electoral_log_board, receive_ballot,
};

/// How an event stores its cast votes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sealing {
    Legacy,
    /// Sealed to a key published and pinned by `publish_key`.
    Published,
    /// The flag only: no key.
    RequiredOnly,
    /// The key in the table, never pinned in the event's log.
    Unpinned,
}

struct Fixture {
    tenant: String,
    event: String,
    election: String,
    area: String,
    style: BallotStyle,
    store: PostgresStore,
    key: Option<SealKey>,
}

impl Fixture {
    async fn create(
        pool: &Pool,
        receipts: bool,
        sealing: Sealing,
        openers: &Openers,
    ) -> Result<Self> {
        let tenant = Uuid::new_v4().to_string();
        let event = Uuid::new_v4().to_string();
        let election = Uuid::new_v4().to_string();
        let area = Uuid::new_v4().to_string();
        let board =
            electoral_log_board::get_event_board(&tenant, &event, &std::env::var("ENV_SLUG")?);
        let store = electoral_log_board::get_event_databases()
            .await?
            .create_event(&tenant, &event)
            .await?;
        store.create_board(&board).await?;
        store.create_ballot_box(&event).await?;
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        windmill::postgres::trusted_write(&tx).await?;
        tx.execute(
            "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1::text::uuid,$1)",
            &[&tenant],
        )
        .await?;
        let receipts_policy = if receipts {
            "signed-by-ballot-box"
        } else {
            "disabled"
        };
        let presentation = json!({"ballot_identity": "voter-id", "network_data_policy": "do-not-store", "voter_signing_policy": "with-signature", "receipts": {"policy": receipts_policy}});
        tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation,bulletin_board_reference,voting_channels) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3,$4,$5)", &[&event,&tenant,&presentation,&json!({"id":0,"database_name":board,"is_archived":false}),&json!({"online":true,"telephone":false})]).await?;
        // Voting has not started: sealing is required before it does.
        tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,voting_channels,num_allowed_revotes) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4,0)", &[&election,&tenant,&event,&json!({"online":true,"telephone":false})]).await?;
        tx.execute("INSERT INTO sequent_backend.area(id,tenant_id,election_event_id,name) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,'Sealed cast votes fixture')", &[&area,&tenant,&event]).await?;
        electoral_log_board::create_protocol_manager_keys(&tx, &tenant, &event, &board).await?;
        let event_row =
            windmill::postgres::election_event::get_election_event_by_id(&tx, &tenant, &event)
                .await?;
        // Only an event with signed receipts has a ballot box key.
        let box_key = ballot_box_key::ballot_box_key_for_publication(&tx, &event_row).await?;
        assert_eq!(box_key.is_some(), receipts);
        let style = BallotStyle {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant.clone(),
            election_event_id: event.clone(),
            election_id: election.clone(),
            area_id: area.clone(),
            contests: vec![get_test_contest()],
            ballot_box_key: box_key,
            ..get_writein_ballot_style()
        };
        let publication = Uuid::new_v4();
        tx.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids,is_generated,published_at) VALUES($1,$2::text::uuid,$3::text::uuid,$4,true,now())", &[&publication,&tenant,&event,&vec![election.parse::<Uuid>()?]]).await?;
        tx.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,$5::text::uuid,$6,$7)", &[&style.id,&tenant,&event,&election,&area,&publication,&serde_json::to_string(&style)?]).await?;
        tx.commit().await?;

        let key = match sealing {
            Sealing::Legacy => None,
            Sealing::RequiredOnly => {
                let db = pool.get().await?;
                let required: bool = db
                    .query_one(
                        "SELECT sequent_backend.cast_vote_seal_require($1::text::uuid,$2::text::uuid)",
                        &[&tenant, &event],
                    )
                    .await?
                    .get(0);
                assert!(required);
                None
            }
            Sealing::Published | Sealing::Unpinned => {
                let (opener, key) = SoftwareOpener::generate(&tenant, &event);
                openers.add(Arc::new(opener));
                let mut db = pool.get().await?;
                let tx = db.transaction().await?;
                if sealing == Sealing::Published {
                    cast_vote_seal::publish_key(&tx, &tenant, &event, &key).await?;
                    // Publishing again is idempotent.
                    cast_vote_seal::publish_key(&tx, &tenant, &event, &key).await?;
                } else {
                    let published: bool = tx
                        .query_one(
                            "SELECT sequent_backend.cast_vote_seal_publish($1::text::uuid,$2::text::uuid,$3,$4,$5)",
                            &[&tenant, &event, &key.key_id, &1i32, &key.public_key],
                        )
                        .await?
                        .get(0);
                    assert!(published);
                }
                tx.commit().await?;
                Some(key)
            }
        };

        // Open voting.
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        windmill::postgres::trusted_write(&tx).await?;
        tx.execute(OPEN_VOTING, &[&election, &json!({"voting_status": "OPEN"})])
            .await?;
        tx.commit().await?;
        Ok(Self {
            tenant,
            event,
            election,
            area,
            style,
            store,
            key,
        })
    }

    fn ballot(&self) -> Result<(InsertCastVoteInput, SigningKey)> {
        let voter_key = SigningKey::generate()?;
        let encrypted = sequent_core::encrypt::encrypt_decoded_contest(
            &voting_crypto::RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &self.style,
        )?;
        let mut signed = SignedHashableBallot::try_from(&encrypted)?;
        let hashable = HashableBallot::try_from(&signed)?;
        let hash = sequent_core::encrypt::hash_ballot(&hashable)?;
        let signature = sequent_core::ballot::sign_hashable_ballot_with_voter_signing_key(
            &voter_key,
            &hash,
            &self.election,
            &hashable,
        )
        .map_err(anyhow::Error::msg)?;
        signed.voter_signing_pk = Some(signature.public_key);
        signed.voter_ballot_signature = Some(signature.signature);
        Ok((
            InsertCastVoteInput {
                election_id: self.election.parse()?,
                ballot_id: hash,
                content: serde_json::to_string(&signed)?,
            },
            voter_key,
        ))
    }

    /// A direct cast (no receipts).
    async fn cast(
        &self,
        voter: &str,
        input: InsertCastVoteInput,
    ) -> Result<InsertCastVoteResult, CastVoteError> {
        try_insert_cast_vote(
            input,
            &self.tenant,
            voter,
            &self.area,
            VotingStatusChannel::ONLINE,
            &Some(Utc::now().timestamp()),
            &None,
            &None,
            &None,
        )
        .await
    }

    /// Receive, then cast by the signed Ballot ID (receipts).
    async fn receive_and_cast(
        &self,
        voter: &str,
        input: InsertCastVoteInput,
        key: &SigningKey,
    ) -> Result<()> {
        let auth = Some(Utc::now().timestamp());
        let received = receive_ballot::try_receive_ballot_with_network(
            input,
            &self.tenant,
            voter,
            &self.area,
            VotingStatusChannel::ONLINE,
            &auth,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("receive: {e:?}"))?;
        let cast = cast_ballot::CastBallotInput {
            election_id: self.election.parse()?,
            ballot_id: received.ballot_id.clone(),
            cast_signature: sign_cast_statement(key, &self.election, &received.ballot_id)?,
        };
        match cast_ballot::try_cast_ballot_with_network(
            cast,
            &self.tenant,
            voter,
            &self.area,
            VotingStatusChannel::ONLINE,
            &auth,
            &None,
            &None,
            &None,
            &None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("cast: {e:?}"))?
        {
            cast_ballot::CastBallotResult::Cast(_) => Ok(()),
            _ => anyhow::bail!("expected a stored cast"),
        }
    }

    /// (format, content, ballot_id) of each stored cast vote, in order.
    async fn stored(&self, table: &str) -> Result<Vec<(String, String, String)>> {
        let sql = match table {
            "ballot_box_ballot" => "SELECT format, content, ballot_id FROM ballot_box_ballot WHERE election_event_id=$1::text::uuid ORDER BY seq",
            _ => "SELECT 'received', content, ballot_hash FROM ballot_box_received WHERE election_event_id=$1::text::uuid ORDER BY received_at",
        };
        Ok(self
            .store
            .client()
            .await?
            .query(sql, &[&self.event])
            .await?
            .iter()
            .map(|row| (row.get(0), row.get(1), row.get(2)))
            .collect())
    }

    /// The voter's own read of a cast vote by its Ballot ID.
    async fn own_content(
        &self,
        pool: &Pool,
        voter: &str,
        ballot_id: &str,
    ) -> Result<Option<String>> {
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        let elections = [self.election.clone()];
        let votes = get_voter_cast_votes(
            &tx,
            &VoterScope {
                tenant_id: &self.tenant,
                election_event_id: &self.event,
                voter_id: voter,
                area_id: &self.area,
                election_ids: &elections,
            },
            VoterCastVotes::Matching {
                election_id: &self.election,
                ballot_id: BallotIdMatch::Exact(ballot_id),
            },
        )
        .await?;
        assert_eq!(votes.len(), 1);
        Ok(votes[0].content.clone())
    }

    /// The tally dump of the area, as parsed CSV records.
    async fn dump(&self, pool: &Pool) -> Result<Vec<Vec<String>>> {
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        let file = tempfile::NamedTempFile::new()?;
        cast_votes::find_area_ballots(
            &tx,
            &self.tenant,
            &self.event,
            &self.area,
            &self.election,
            &file.path().to_path_buf(),
        )
        .await?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(file.path())?;
        Ok(reader
            .records()
            .map(|record| record.map(|r| r.iter().map(str::to_string).collect()))
            .collect::<Result<_, _>>()?)
    }
}

fn assert_sealed(row: &(String, String, String), inner: &InsertCastVoteInput, key: &SealKey) {
    let (format, content, ballot_id) = row;
    assert_eq!(ballot_id, &inner.ballot_id);
    let envelope = SealedBallot::from_content(content).unwrap();
    assert_eq!(envelope.ballot_id, inner.ballot_id);
    assert_eq!(envelope.key_id, key.key_id);
    assert!(
        !content.contains(&inner.content),
        "the stored copy is the voter's ballot"
    );
    if format != "received" {
        assert_eq!(format, "hsm-seal-v1+hashable-ballot");
    }
}

/// An event with one election whose voting has not started, in Hasura only.
async fn unopened_event(pool: &Pool) -> Result<(String, String, String)> {
    let tenant = Uuid::new_v4().to_string();
    let event = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    windmill::postgres::trusted_write(&tx).await?;
    tx.execute(
        "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1::text::uuid,$1)",
        &[&tenant],
    )
    .await?;
    let channels = json!({"online": true, "telephone": false});
    tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,voting_channels) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3)", &[&event,&tenant,&channels]).await?;
    tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,voting_channels,num_allowed_revotes) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4,0)", &[&election,&tenant,&event,&channels]).await?;
    tx.commit().await?;
    Ok((tenant, event, election))
}

const REQUIRE: &str =
    "SELECT sequent_backend.cast_vote_seal_require($1::text::uuid,$2::text::uuid)";
const OPEN_VOTING: &str =
    "UPDATE sequent_backend.election SET status = $2 WHERE id = $1::text::uuid";

fn assert_lock_timeout(result: Result<impl std::fmt::Debug, tokio_postgres::Error>) {
    let error = result.unwrap_err();
    assert_eq!(
        error.code(),
        Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE),
        "{error:?}"
    );
}

/// Exercise the verified receiver's real Rust status transition and signed
/// board append. The receipt inputs stand in for an already admitted package;
/// the public point is synthetic, not from a physical device.
async fn verified_receiver_needs_the_signed_pin(pool: &Pool) -> Result<()> {
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let election = Uuid::new_v4();
    let (tenant_str, event_str, election_str) =
        (tenant.to_string(), event.to_string(), election.to_string());
    let board =
        electoral_log_board::get_event_board(&tenant_str, &event_str, &std::env::var("ENV_SLUG")?);
    let databases = electoral_log_board::get_event_databases().await?;
    let store = databases.create_event(&tenant_str, &event_str).await?;
    store.create_board(&board).await?;
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    windmill::postgres::trusted_write(&tx).await?;
    tx.execute(
        "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1,$2)",
        &[&tenant, &tenant_str],
    )
    .await?;
    tx.execute(
        "INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,bulletin_board_reference,voting_channels)
         VALUES($1,$2,'RSA256',$3,$4)",
        &[&event, &tenant, &json!({"id":0,"database_name":board,"is_archived":false}), &json!({"online":true})],
    ).await?;
    tx.execute(
        "INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,voting_channels)
         VALUES($1,$2,$3,$4)",
        &[&election, &tenant, &event, &json!({"online":true})],
    )
    .await?;
    electoral_log_board::create_protocol_manager_keys(&tx, &tenant_str, &event_str, &board).await?;

    // This synthetic login is scoped to the test transaction. Rollback removes
    // it and the provisioned receiver; nothing is installed on another DB.
    let publisher = format!("synthetic_cast_{}", event.simple());
    tx.batch_execute(&format!(
        "CREATE ROLE {publisher} LOGIN;
         GRANT sequent_os_verified_cast_key_publisher TO {publisher}"
    ))
    .await?;
    tx.execute(
        "INSERT INTO sequent_backend.verified_cast_key_site(tenant_id,publisher_login) VALUES($1,$2::text::name)",
        &[&tenant, &publisher],
    ).await?;
    let (manifest, roster) = ("c".repeat(64), "a".repeat(64));
    tx.execute(
        "INSERT INTO sequent_backend.managed_webauthn_roster(tenant_id,roster_json,sha256,generation,activated_by)
         VALUES($1,'{}',$2,1,'synthetic')",
        &[&tenant, &roster],
    ).await?;
    tx.execute(
        "INSERT INTO sequent_backend.configuration_package
         (tenant_id,election_event_id,external_id,revision,manifest_sha256,manifest,signer_subject,signer_serial,signer_fingerprint,approvers)
         VALUES($1,$2,$3,1,$4,'{}','synthetic','1','synthetic','[]')",
        &[&tenant, &event, &event_str, &manifest],
    ).await?;
    assert!(tx
        .query_one(
            "SELECT sequent_backend.verified_cast_key_authorize($1,$2,1,$3,1,$4)",
            &[&tenant, &event, &manifest],
        )
        .await?
        .get::<_, bool>(0));
    tx.batch_execute(
        "SET CONSTRAINTS sequent_backend.verified_cast_key_registered_import_guard IMMEDIATE",
    )
    .await?;

    // This test binary has exactly one test; its process-wide settings are
    // not shared with other Windmill tests.
    std::env::set_var("WINDMILL_VERIFIED_CAST_VOTE_TENANT", &tenant_str);
    std::env::set_var("SUPER_ADMIN_TENANT_ID", &tenant_str);
    let status_open = VotingStatus::OPEN;
    let open = || {
        change_election_voting_status(
            &tenant_str,
            None,
            None,
            &event_str,
            &election_str,
            &status_open,
            VotingStatusChannel::ONLINE,
            &tx,
        )
    };
    let system = cast_vote_seal::board_public_key(&tx, &tenant_str, &event_str, &board).await?;
    let pending = open().await.unwrap_err();
    assert!(pending.is::<VotingTransitionError>());
    assert!(pending
        .to_string()
        .contains("waiting for its cast vote key"));
    assert!(cast_vote_seal::read_pin(&store, &board, &system)
        .await?
        .is_none());

    let (_, key) = SoftwareOpener::generate(&tenant_str, &event_str);
    tx.batch_execute(&format!("SET LOCAL SESSION AUTHORIZATION {publisher}"))
        .await?;
    let job = tx
        .query_one(
            "SELECT job_id::uuid FROM sequent_backend.verified_cast_key_next()",
            &[],
        )
        .await?
        .get::<_, Uuid>(0);
    assert!(tx
        .query_one(
            "SELECT sequent_backend.verified_cast_key_stage($1,$2,$3,$4,$5)",
            &[&job, &tenant, &event, &manifest, &key.public_key],
        )
        .await?
        .get::<_, bool>(0));
    tx.batch_execute("RESET SESSION AUTHORIZATION").await?;
    // Staging is not publication and may not pin the event on its own.
    assert!(cast_vote_seal::read_pin(&store, &board, &system)
        .await?
        .is_none());
    assert_eq!(
        cast_vote_seal::sealing(&tx, &tenant_str, &event_str).await?,
        cast_vote_seal::Sealing::Required(None)
    );
    assert!(open().await?);
    assert_eq!(
        cast_vote_seal::sealing(&tx, &tenant_str, &event_str).await?,
        cast_vote_seal::Sealing::Required(Some(key.clone()))
    );
    cast_vote_seal::check_pin(
        &key,
        cast_vote_seal::read_pin(&store, &board, &system)
            .await?
            .as_ref(),
        &tenant_str,
        &event_str,
    )?;
    let row = windmill::postgres::election::get_election_by_id(
        &tx,
        &tenant_str,
        &event_str,
        &election_str,
    )
    .await?
    .unwrap();
    let status = get_election_status(row.status).unwrap();
    assert_eq!(
        status.status_by_channel(VotingStatusChannel::ONLINE),
        VotingStatus::OPEN
    );
    assert!(tx
        .query_one(
            "SELECT completed FROM sequent_backend.verified_cast_key_for_open($1,$2)",
            &[&tenant, &event],
        )
        .await?
        .get::<_, bool>(0));
    assert!(!open().await?, "retry must not create a second pin");
    tx.rollback().await?;
    databases.drop_event(&tenant_str, &event_str).await?;
    Ok(())
}

#[tokio::test]
async fn sealed_cast_votes_end_to_end() -> Result<()> {
    std::env::set_var("MASTER_SECRET", "5a".repeat(32));
    let pool = schema::pool().await;
    let database: String = pool
        .get()
        .await?
        .query_one("SELECT current_database()", &[])
        .await?
        .get(0);
    // This binary owns its process-wide pools; no other test changes its environment.
    std::env::set_var("HASURA_DB__DBNAME", database);
    let pool = get_hasura_pool().await.as_ref().clone();
    let openers = Arc::new(Openers::default());

    // A database without the state function reads every event as legacy,
    // and the caller's transaction stays usable. First, while no read has
    // seen the function.
    {
        let (tenant, event, _) = unopened_event(&pool).await?;
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        tx.batch_execute("DROP FUNCTION sequent_backend.cast_vote_seal_state(uuid, uuid)")
            .await?;
        assert_eq!(
            cast_vote_seal::sealing(&tx, &tenant, &event).await?,
            cast_vote_seal::Sealing::Legacy
        );
        let usable: i32 = tx.query_one("SELECT 1", &[]).await?.get(0);
        assert_eq!(usable, 1);
        tx.rollback().await?;
    }

    // Requiring sealing and opening voting are serialized: neither commits
    // past the other while it is in flight.
    {
        let opened = json!({"voting_status": "OPEN"});
        let (tenant, event, election) = unopened_event(&pool).await?;
        let mut opener_db = pool.get().await?;
        let mut requirer_db = pool.get().await?;
        let opening = opener_db.transaction().await?;
        windmill::postgres::trusted_write(&opening).await?;
        opening.execute(OPEN_VOTING, &[&election, &opened]).await?;
        let requiring = requirer_db.transaction().await?;
        requiring
            .batch_execute("SET LOCAL lock_timeout = '200ms'")
            .await?;
        assert_lock_timeout(requiring.query_one(REQUIRE, &[&tenant, &event]).await);
        requiring.rollback().await?;
        opening.commit().await?;
        let required: bool = requirer_db
            .query_one(REQUIRE, &[&tenant, &event])
            .await?
            .get(0);
        assert!(!required, "voting opened first");

        let (tenant, event, election) = unopened_event(&pool).await?;
        let requiring = requirer_db.transaction().await?;
        let required: bool = requiring
            .query_one(REQUIRE, &[&tenant, &event])
            .await?
            .get(0);
        assert!(required);
        let opening = opener_db.transaction().await?;
        windmill::postgres::trusted_write(&opening).await?;
        opening
            .batch_execute("SET LOCAL lock_timeout = '200ms'")
            .await?;
        assert_lock_timeout(opening.execute(OPEN_VOTING, &[&election, &opened]).await);
        opening.rollback().await?;
        requiring.commit().await?;

        // Requiring needs READ COMMITTED, where each statement sees what
        // committed before its lock.
        let (tenant, event, _) = unopened_event(&pool).await?;
        let repeatable = requirer_db
            .build_transaction()
            .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
            .start()
            .await?;
        let refused = repeatable
            .query_one(REQUIRE, &[&tenant, &event])
            .await
            .unwrap_err();
        assert_eq!(
            refused.code(),
            Some(&tokio_postgres::error::SqlState::INVALID_TRANSACTION_STATE),
            "{refused:?}"
        );
        repeatable.rollback().await?;
    }

    // The publish function: only the publisher's role calls it.
    {
        let fixture = Fixture::create(&pool, false, Sealing::Legacy, &openers).await?;
        let (_, key) = SoftwareOpener::generate(&fixture.tenant, &fixture.event);
        let mut db = pool.get().await?;
        for role in ["sequent_os_approve", "sequent_os_ledger_data_owner"] {
            let tx = db.transaction().await?;
            tx.batch_execute(&format!("SET LOCAL ROLE {role}")).await?;
            let refused = tx
                .query_one(
                    "SELECT sequent_backend.cast_vote_seal_publish($1::text::uuid,$2::text::uuid,$3,1,$4)",
                    &[&fixture.tenant, &fixture.event, &key.key_id, &key.public_key],
                )
                .await
                .unwrap_err();
            assert_eq!(
                refused.code(),
                Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE),
                "{role}: {refused:?}"
            );
            tx.rollback().await?;
        }
        // The publisher may, but not for an event that opened voting
        // without requiring sealing first.
        let tx = db.transaction().await?;
        tx.batch_execute("SET LOCAL ROLE sequent_os_cast_vote_key_publisher")
            .await?;
        let published: bool = tx
            .query_one(
                "SELECT sequent_backend.cast_vote_seal_publish($1::text::uuid,$2::text::uuid,$3,1,$4)",
                &[&fixture.tenant, &fixture.event, &key.key_id, &key.public_key],
            )
            .await?
            .get(0);
        assert!(!published);
        let state: bool = tx
            .query_one(
                "SELECT required FROM sequent_backend.cast_vote_seal_state($1::text::uuid,$2::text::uuid)",
                &[&fixture.tenant, &fixture.event],
            )
            .await?
            .get(0);
        assert!(!state);
        tx.rollback().await?;
        let tx = db.transaction().await?;
        assert!(
            cast_vote_seal::publish_key(&tx, &fixture.tenant, &fixture.event, &key)
                .await
                .is_err()
        );
        tx.rollback().await?;
    }

    // A sealed event with direct casts.
    let direct = Fixture::create(&pool, false, Sealing::Published, &openers).await?;
    let key = direct.key.clone().unwrap();
    // Another key is refused for the event.
    {
        let (_, other) = SoftwareOpener::generate(&direct.tenant, &direct.event);
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        assert!(
            cast_vote_seal::publish_key(&tx, &direct.tenant, &direct.event, &other)
                .await
                .is_err()
        );
        tx.rollback().await?;
    }
    let voters: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
    let mut inputs = Vec::new();
    for voter in &voters {
        let (input, _) = direct.ballot()?;
        let InsertCastVoteResult::Success(cast) = direct
            .cast(voter, input.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?
        else {
            anyhow::bail!("expected a valid cast");
        };
        // The voter's answer is their own ballot, not the sealed copy.
        assert_eq!(cast.content.as_deref(), Some(input.content.as_str()));
        inputs.push(input);
    }
    let stored = direct.stored("ballot_box_ballot").await?;
    assert_eq!(stored.len(), 3);
    for (row, input) in stored.iter().zip(&inputs) {
        assert_sealed(row, input, &key);
    }
    assert_eq!(
        direct
            .own_content(&pool, &voters[0], &inputs[0].ballot_id)
            .await?,
        None
    );

    // The dump: refused without an opener, the voters' ballots with one.
    assert_eq!(
        ballot_box::sequence_event(&direct.tenant, &direct.event).await?,
        Some(3)
    );
    let refused = direct.dump(&pool).await.unwrap_err();
    assert!(format!("{refused:#}").contains("no opener"), "{refused:#}");
    cast_vote_seal::install(openers.clone())?;
    let dumped = direct.dump(&pool).await?;
    let mut expected: Vec<Vec<String>> = voters
        .iter()
        .zip(&inputs)
        .map(|(voter, input)| vec![voter.clone(), input.content.clone(), "ONLINE".to_string()])
        .collect();
    expected.sort();
    assert_eq!(dumped, expected);

    // A sealed event with receipts: the received copy is sealed too, and
    // the cast stores it as it is.
    let received = Fixture::create(&pool, true, Sealing::Published, &openers).await?;
    let received_key = received.key.clone().unwrap();
    let voter = Uuid::new_v4().to_string();
    let (input, voter_key) = received.ballot()?;
    received
        .receive_and_cast(&voter, input.clone(), &voter_key)
        .await?;
    let received_rows = received.stored("ballot_box_received").await?;
    let cast_rows = received.stored("ballot_box_ballot").await?;
    assert_eq!((received_rows.len(), cast_rows.len()), (1, 1));
    assert_sealed(&received_rows[0], &input, &received_key);
    assert_sealed(&cast_rows[0], &input, &received_key);
    assert_eq!(received_rows[0].1, cast_rows[0].1);
    assert!(
        ballot_box::sequence_event(&received.tenant, &received.event)
            .await?
            .is_some()
    );
    assert_eq!(
        received.dump(&pool).await?,
        vec![vec![
            voter.clone(),
            input.content.clone(),
            "ONLINE".to_string()
        ]]
    );

    // A legacy event stores and dumps plaintext exactly as before.
    let legacy = Fixture::create(&pool, false, Sealing::Legacy, &openers).await?;
    let voter = Uuid::new_v4().to_string();
    let (input, _) = legacy.ballot()?;
    legacy
        .cast(&voter, input.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    assert_eq!(
        legacy.stored("ballot_box_ballot").await?,
        vec![(
            "hashable-ballot".to_string(),
            input.content.clone(),
            input.ballot_id.clone()
        )]
    );
    assert_eq!(
        legacy.own_content(&pool, &voter, &input.ballot_id).await?,
        Some(input.content.clone())
    );
    ballot_box::sequence_event(&legacy.tenant, &legacy.event).await?;
    assert_eq!(
        legacy.dump(&pool).await?,
        vec![vec![voter, input.content, "ONLINE".to_string()]]
    );

    // Sealing required without a key, or with a key its log doesn't pin:
    // the cast is refused and nothing is stored.
    for sealing in [Sealing::RequiredOnly, Sealing::Unpinned] {
        let fixture = Fixture::create(&pool, false, sealing, &openers).await?;
        let (input, _) = fixture.ballot()?;
        let refused = fixture.cast(&Uuid::new_v4().to_string(), input).await;
        assert!(
            matches!(refused, Err(CastVoteError::BallotSealFailed(_))),
            "{:?}",
            refused.err()
        );
        assert!(fixture.stored("ballot_box_ballot").await?.is_empty());
    }

    for fixture in [direct, received, legacy] {
        electoral_log_board::get_event_databases()
            .await?
            .drop_event(&fixture.tenant, &fixture.event)
            .await?;
    }
    verified_receiver_needs_the_signed_pin(&pool).await?;
    Ok(())
}
