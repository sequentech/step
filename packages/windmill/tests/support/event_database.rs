// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! An election event's electoral-log database, on the PostgreSQL server that
//! `ELECTORAL_LOG_PG_*` names, created through the same process-wide event
//! databases the services use, with the event's board and ballot box. Votes
//! go in through `accept_ballot` and reach the board through the sequencer's
//! steps. Include it with `#[path = "support/event_database.rs"] mod
//! event_database;`.
//!
//! Those event databases keep their connections in one pool per process, and
//! a connection dies with the runtime that opened it, so every test that
//! uses them runs on the one runtime of [`run`], not a `#[tokio::test]`
//! runtime of its own.

#![allow(dead_code)]

use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::ports::ElectoralLogStore;
use electoral_log::seal::ballot_hash;
use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use windmill::services::ballot_box::ballot_record;
use windmill::services::electoral_log_board::{get_event_board, get_event_databases};

/// The slug of the boards the tests create.
const BOARD_SLUG: &str = "sealtest";
/// The ballots one sequencer pass appends.
const SEQUENCER_BATCH: i64 = 1000;
/// The format of the ballots the tests cast.
pub const FORMAT: &str = "hashable-ballot";

/// Runs a test on the runtime the tests of this binary share.
pub fn run<F: Future>(test: F) -> F::Output {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("the tests' runtime")
        })
        .block_on(test)
}

/// The board of an event, named as the services name it.
pub fn board_name(tenant: Uuid, event: Uuid) -> String {
    get_event_board(&tenant.to_string(), &event.to_string(), BOARD_SLUG)
}

/// A ballot to cast.
pub struct Vote<'a> {
    pub election: Uuid,
    pub area: Uuid,
    pub voter: &'a str,
    pub content: &'a str,
    pub ballot_id: &'a str,
    pub channel: &'a str,
}

pub struct EventDatabase {
    pub store: PostgresStore,
    pub board: String,
    tenant: Uuid,
    event: Uuid,
}

impl EventDatabase {
    /// Creates the event's database with its board and ballot box.
    pub async fn create(tenant: Uuid, event: Uuid) -> EventDatabase {
        let databases = get_event_databases()
            .await
            .expect("ELECTORAL_LOG_PG_* must name the test PostgreSQL server");
        let store = databases
            .create_event(&tenant.to_string(), &event.to_string())
            .await
            .expect("the event's electoral-log database");
        let board = board_name(tenant, event);
        store.create_board(&board).await.unwrap();
        store.create_ballot_box(&event.to_string()).await.unwrap();
        EventDatabase {
            store,
            board,
            tenant,
            event,
        }
    }

    /// Accepts a vote with no revote limit; returns how the ballot box
    /// answered.
    pub async fn try_cast(&self, vote: &Vote<'_>, status: BallotStatus) -> AcceptOutcome {
        let (event, election, area) = (
            self.event.to_string(),
            vote.election.to_string(),
            vote.area.to_string(),
        );
        self.store
            .accept_ballot(&AcceptBallot {
                election_event_id: &event,
                election_id: &election,
                area_id: &area,
                voter_id: vote.voter,
                ballot_id: vote.ballot_id,
                format: FORMAT,
                content: vote.content,
                voter_signature: None,
                admission_record: &format!(r#"{{"payload":{{"accepted_at":"2026-10-27T12:00:00.000Z","sequence":{}}},"signature":""}}"#, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64).into_bytes(),
                pseudonym_hash: &ballot_hash(vote.voter).unwrap(),
                ballot_hash: &ballot_hash(vote.content).unwrap(),
                voting_channel: vote.channel,
                status,
                voter_ip: None,
                voter_country: None,
                username: Some(vote.voter),
                allowed_votes: 0,
            })
            .await
            .unwrap()
    }

    /// Accepts a valid vote; returns the stored ballot's ID.
    pub async fn cast(&self, vote: &Vote<'_>) -> String {
        match self.try_cast(vote, BallotStatus::Valid).await {
            AcceptOutcome::Accepted { id, .. } => id,
            other => panic!("the vote of {} was not accepted: {other:?}", vote.voter),
        }
    }

    /// Accepts a vote and rejects it, as a discard does.
    pub async fn cast_rejected(&self, vote: &Vote<'_>) {
        let id = match self.try_cast(vote, BallotStatus::Pending).await {
            AcceptOutcome::Accepted { id, .. } => id,
            other => panic!("the vote of {} was not accepted: {other:?}", vote.voter),
        };
        assert!(self
            .store
            .set_ballot_status(
                &self.event.to_string(),
                vote.voter,
                &id,
                BallotStatus::Pending,
                BallotStatus::Rejected,
            )
            .await
            .unwrap());
    }

    /// Appends the records of every accepted ballot to the board, signed
    /// with `key`, as the sequencer does; returns how many.
    pub async fn sequence(&self, key: &SigningKey) -> usize {
        let event = self.event.to_string();
        let mut appended = 0;
        loop {
            let pending = self
                .store
                .pending_ballots(&event, SEQUENCER_BATCH)
                .await
                .unwrap();
            if pending.is_empty() {
                return appended;
            }
            let entries: Vec<_> = pending
                .iter()
                .map(|ballot| ballot_record(&self.board, &event, ballot, key).unwrap())
                .collect();
            self.store
                .append(&self.board, &mut entries.into_iter().map(anyhow::Ok))
                .await
                .unwrap();
            let seqs: Vec<i64> = pending.iter().map(|ballot| ballot.seq).collect();
            self.store.remove_pending(&event, &seqs).await.unwrap();
            appended += seqs.len();
        }
    }

    /// Drops the event's database.
    pub async fn drop(self) {
        if let Err(error) = get_event_databases()
            .await
            .unwrap()
            .drop_event(&self.tenant.to_string(), &self.event.to_string())
            .await
        {
            eprintln!("dropping the database of event {}: {error}", self.event);
        }
    }
}
