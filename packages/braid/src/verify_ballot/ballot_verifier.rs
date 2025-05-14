#![allow(non_camel_case_types)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use b3::messages::artifact::Ballots;
use b3::messages::message::Message;
use reedline_repl_rs::yansi::Paint;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use sequent_core::multi_ballot::{HashableMultiBallot, HashableMultiBallotContests};
use sequent_core::serialization::base64::Base64Serialize;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandDeserialize;
use thiserror::Error;

pub const VERIFIABLE_BULLETIN_BOARD_FILE: &str = "export_verifiable_bulletin_board.db";

#[derive(Error, Debug)]
pub enum BallotVerifierError {
    #[error("Could not open the data file. Please make sure it exists and is readable.")]
    DataFileOpen,

    #[error("Internal data error. Please try again.")]
    DataCorruption,

    #[error("Ballot not found. Please double-check the ballot ID and try again.")]
    BallotNotFound,

    #[error("An unexpected error occurred. Please contact support.")]
    Unexpected,

    #[error("The ballot hash you provided isn’t valid. Please double-check and try again.")]
    InvalidHash,
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

    pub async fn run(&mut self) -> Result<(), BallotVerifierError> {
        let conn = Connection::open_with_flags(
            VERIFIABLE_BULLETIN_BOARD_FILE,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|_| BallotVerifierError::DataFileOpen)?;

        let mut cv_stmt = conn
            .prepare("SELECT content FROM cast_vote WHERE ballot_id = ?1 LIMIT 1")
            .map_err(|_| BallotVerifierError::Unexpected)?;

        let content_opt: Option<String> = cv_stmt
            .query_row(params![&self.ballot_hash], |r| r.get(0))
            .optional()
            .map_err(|_| BallotVerifierError::Unexpected)?;

        let content = content_opt.ok_or(BallotVerifierError::BallotNotFound)?;

        let ballot: HashableMultiBallot =
            deserialize_str(&content).map_err(|_| BallotVerifierError::DataCorruption)?;

        let contests: HashableMultiBallotContests<RistrettoCtx> = ballot
            .deserialize_contests()
            .map_err(|_| BallotVerifierError::DataCorruption)?;
        let ciphertext = contests.ciphertext.clone();
        let b64 = ciphertext
            .serialize()
            .map_err(|_| BallotVerifierError::Unexpected)?;

        let batch_opt = Self::find_batch_for_ciphertext(&conn, &b64).await?;

        if let Some(batch) = batch_opt {
            println!("{}", format!("Found ballot in batch {}", batch).green());
        } else {
            return Err(BallotVerifierError::BallotNotFound);
        }

        Ok(())
    }

    async fn find_batch_for_ciphertext(
        conn: &Connection,
        target_b64: &str,
    ) -> Result<Option<i32>, BallotVerifierError> {
        let mut stmt = conn
            .prepare(
                "SELECT batch, message FROM b3_messages
                 WHERE statement_kind = ?1
                 ORDER BY id",
            )
            .map_err(|_| BallotVerifierError::Unexpected)?;
        let mut rows = stmt
            .query(params!["Ballots"])
            .map_err(|_| BallotVerifierError::Unexpected)?;

        while let Some(row) = rows.next().map_err(|_| BallotVerifierError::Unexpected)? {
            let batch: i32 = row.get(0).map_err(|_| BallotVerifierError::Unexpected)?;
            let raw: Vec<u8> = row.get(1).map_err(|_| BallotVerifierError::Unexpected)?;

            let msg: Message = StrandDeserialize::strand_deserialize(&raw)
                .map_err(|_| BallotVerifierError::DataCorruption)?;
            let ballots_bytes = msg.artifact.ok_or(BallotVerifierError::DataCorruption)?;
            let ballots: Ballots<RistrettoCtx> =
                StrandDeserialize::strand_deserialize(&ballots_bytes)
                    .map_err(|_| BallotVerifierError::DataCorruption)?;

            for ct in &ballots.ciphertexts.0 {
                let ct_b64 = ct
                    .serialize()
                    .map_err(|_| BallotVerifierError::Unexpected)?;
                if ct_b64 == target_b64 {
                    return Ok(Some(batch));
                }
            }
        }

        Ok(None)
    }
}
