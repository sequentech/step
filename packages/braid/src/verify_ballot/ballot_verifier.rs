#![allow(non_camel_case_types)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use b3::messages::artifact::Ballots;
use b3::messages::message::Message;
use colored::*;
use reedline_repl_rs::yansi::Paint;
use rusqlite::{params, Connection, OptionalExtension};
use sequent_core::multi_ballot::{HashableMultiBallot, HashableMultiBallotContests};
use sequent_core::serialization::base64::Base64Serialize;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
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

        let content_opt = match content_opt {
            Some(content) => content,
            None => {
                println!(
                    "{}",
                    format!("Ballot ID not found in cast_vote table").red()
                );
                return Ok(());
            }
        };

        let hashable_multi_ballot: HashableMultiBallot = deserialize_str(&content_opt)?;

        let hashable_multi_ballot_contests: HashableMultiBallotContests<RistrettoCtx> =
            hashable_multi_ballot
                .deserialize_contests()
                .map_err(|err| anyhow!("{:?}", err))?;
        let ciphertext: Ciphertext<RistrettoCtx> =
            hashable_multi_ballot_contests.ciphertext.clone();

        let ballot_ciphertext_base64 = ciphertext.serialize()?;

        let ballot_batch = Self::find_batch_for_ciphertext(&conn, &ballot_ciphertext_base64)
            .await
            .context("Failed to find batch for ciphertext")?;

        if let Some(batch) = ballot_batch {
            println!("{}", format!("Found Ballot at batch: {batch}").green());
        }

        Ok(())
    }

    async fn find_batch_for_ciphertext(
        conn: &Connection,
        ballot_ciphertext_base64: &str,
    ) -> anyhow::Result<Option<i32>> {
        let mut stmt = conn.prepare(
            "SELECT
                batch,
                message
             FROM b3_messages
             WHERE statement_kind = ?1
             ORDER BY id",
        )?;

        let mut rows = stmt.query(params!["Ballots"])?;

        while let Some(row) = rows.next()? {
            let batch: i32 = row.get(0)?;
            let raw_message: Vec<u8> = row.get(1)?;

            let msg: Message = StrandDeserialize::strand_deserialize(&raw_message)
                .context("failed to parse Message")?;

            let ballots_bytes = msg.artifact.context("Message.artifact was None")?;

            let ballots: Ballots<RistrettoCtx> =
                StrandDeserialize::strand_deserialize(&ballots_bytes)
                    .context("failed to parse Ballots")?;

            // scan each ciphertext for a match
            for ct in &ballots.ciphertexts.0 {
                let ct_b64 = ct.serialize().unwrap(); // TODO: handle errors
                if ct_b64 == ballot_ciphertext_base64 {
                    return Ok(Some(batch));
                }
            }
        }

        Ok(None)
    }
}
