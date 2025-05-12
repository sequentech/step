#![allow(non_camel_case_types)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
// use b3::client::pgsql::B3MessageRow;
use colored::*;
use rusqlite::{params, Connection};
use rusqlite::{OptionalExtension, Row};
use sequent_core::multi_ballot::{HashableMultiBallot, HashableMultiBallotContests};
use sequent_core::serialization::deserialize_with_path::deserialize_str;

use b3::messages::artifact::Configuration;
use b3::messages::message::Message;
use b3::messages::message::VerifiedMessage;
use b3::messages::newtypes::*;
use b3::messages::statement::StatementType;
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use tracing::info;

pub const VERIFIABLE_BULLETIN_BOARD_FILE: &str = "export_verifiable_bulletin_board.db";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct B3MessageRow {
    pub id: i64,
    pub created: String,
    pub sender_pk: String,
    pub statement_timestamp: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
    pub message: Vec<u8>,
    pub version: String,
}

pub struct BallotVerifier {
    ballot_hash: String,
}

impl BallotVerifier {
    pub fn new(ballot_hash: &str) -> BallotVerifier {
        BallotVerifier {
            ballot_hash: ballot_hash.to_string(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let ballot_id = &self.ballot_hash;
        let conn = Connection::open(VERIFIABLE_BULLETIN_BOARD_FILE)
            .context("Failed to open verifiable_bulletin_board.db")?;

        // 2) Pull the `content` for this ballot_id
        let mut cv_stmt = conn
            .prepare(
                "SELECT content
           FROM cast_vote
          WHERE ballot_id = ?1
          LIMIT 1",
            )
            .context("Failed to prepare cast_vote lookup")?;

        let content_opt = cv_stmt
            .query_row(params![ballot_id], |row| row.get::<_, String>(0))
            .optional()
            .context("Error querying cast_vote")?;

        println!("content_opt: {:?}", content_opt);
        let content_opt = match content_opt {
            Some(content) => content,
            None => {
                println!("Ballot ID not found in cast_vote table");
                return Ok(());
            }
        };

        let hashable_multi_ballot: HashableMultiBallot = deserialize_str(&content_opt)?;

        let hashable_multi_ballot_contests: HashableMultiBallotContests<RistrettoCtx> =
            hashable_multi_ballot
                .deserialize_contests()
                .map_err(|err| anyhow!("{:?}", err))?;
        let ciphertext = hashable_multi_ballot_contests.ciphertext.clone();
        let ciphertext_base64 = ciphertext.serialize()?;

        print!("ciphertext_base64: {:?}", ciphertext_base64);

        let mut stmt = conn.prepare(
            "SELECT
                id,
                created,
                sender_pk,
                statement_timestamp,
                statement_kind,
                batch,
                mix_number,
                message,
                version
             FROM b3_messages
             WHERE statement_kind = ?1
             ORDER BY id",
        )?;

        // 3) Execute and map each row into your struct
        let rows = stmt
            .query_map(params!["Ballots"], |row| {
                Ok(B3MessageRow {
                    id: row.get(0)?,
                    created: row.get(1)?,
                    sender_pk: row.get(2)?,
                    statement_timestamp: row.get(3)?,
                    statement_kind: row.get(4)?,
                    batch: row.get(5)?,
                    mix_number: row.get(6)?,
                    message: row.get(7)?,
                    version: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for b3_message in rows {
            println!("-------batch: {:?}------", b3_message.batch);
        }

        Ok(())
    }
}
