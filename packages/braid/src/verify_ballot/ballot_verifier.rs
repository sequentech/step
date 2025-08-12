#![allow(non_camel_case_types)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use b3::messages::artifact::Ballots;
use b3::messages::message::Message;
use colored::Colorize;
use hex::FromHex;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use sequent_core::multi_ballot::{HashableMultiBallot, HashableMultiBallotContests};
use sequent_core::serialization::base64::Base64Serialize;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use std::io;
use std::io::Write;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandDeserialize;
use strum::Display;

use crate::util::VERIFIABLE_BULLETIN_BOARD_FILE;

#[derive(Debug, Display)]
pub enum BallotVerifierError {
    #[strum(to_string = "Invalid ballot hash: {0}")]
    InvalidHash(String),

    #[strum(to_string = "Could not open data file: {0}")]
    DataFileOpen(String),

    #[strum(to_string = "Data corruption encountered: {0}")]
    DataCorruption(String),

    #[strum(to_string = "Ballot not found for hash: {0}")]
    BallotNotFound(String),

    #[strum(to_string = "Unexpected error: {0}")]
    Unexpected(String),
}

pub struct BallotVerifier<'conn> {
    ballot_hash: String,
    sqlite_connection: &'conn Connection,
}

impl<'conn> BallotVerifier<'conn> {
    pub fn new(ballot_hash: &str, conn: &'conn Connection) -> BallotVerifier<'conn> {
        BallotVerifier {
            ballot_hash: ballot_hash.to_string(),
            sqlite_connection: conn,
        }
    }

    pub async fn run(&mut self) -> Result<(), BallotVerifierError> {
        // 1) Validate hash, capture the bad input in the error
        Self::validate_ballot_hash(&self.ballot_hash)
            .map_err(|_| BallotVerifierError::InvalidHash(self.ballot_hash.clone()))?;

        // 3) Prepare statement
        let mut cv_stmt = self
            .sqlite_connection
            .prepare("SELECT content FROM cast_vote WHERE ballot_id = ?1 LIMIT 1")
            .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;

        // 4) Run query, allow “not found” to be distinct
        let content: String = cv_stmt
            .query_row(params![&self.ballot_hash], |r| r.get(0))
            .optional()
            .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?
            .ok_or_else(|| BallotVerifierError::BallotNotFound(self.ballot_hash.clone()))?;

        // 5) Deserialize the ballot
        let hashable_multi_ballot: HashableMultiBallot = deserialize_str(&content)
            .map_err(|e| BallotVerifierError::DataCorruption(e.to_string()))?;

        let contests: HashableMultiBallotContests<RistrettoCtx> = hashable_multi_ballot
            .deserialize_contests()
            .map_err(|e| BallotVerifierError::DataCorruption(e.to_string()))?;
        let ciphertext_b64 = contests
            .ciphertext
            .clone()
            .serialize()
            .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;

        // 6) Find its batch
        match self.find_batch_for_ciphertext(&ciphertext_b64).await? {
            Some(batch) => {
                let _ = writeln!(
                    io::stderr(),
                    "{}",
                    format!("Found ballot in batch {}", batch).green()
                );
                Ok(())
            }
            None => Err(BallotVerifierError::BallotNotFound(
                self.ballot_hash.clone(),
            )),
        }
    }

    async fn find_batch_for_ciphertext(
        &self,
        target_b64: &str,
    ) -> Result<Option<i32>, BallotVerifierError> {
        let mut stmt = self
            .sqlite_connection
            .prepare(
                "SELECT batch, message
                   FROM bulletin_board
                  WHERE statement_kind = ?1
                  ORDER BY id",
            )
            .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;

        let mut rows = stmt
            .query(params!["Ballots"])
            .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?
        {
            let batch: i32 = row
                .get(0)
                .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;
            let raw: Vec<u8> = row
                .get(1)
                .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;

            let msg: Message = StrandDeserialize::strand_deserialize(&raw)
                .map_err(|e| BallotVerifierError::DataCorruption(e.to_string()))?;
            let ballots_bytes = msg
                .artifact
                .ok_or_else(|| BallotVerifierError::DataCorruption("missing artifact".into()))?;
            let ballots: Ballots<RistrettoCtx> =
                StrandDeserialize::strand_deserialize(&ballots_bytes)
                    .map_err(|e| BallotVerifierError::DataCorruption(e.to_string()))?;

            for ct in &ballots.ciphertexts.0 {
                let ct_b64 = ct
                    .serialize()
                    .map_err(|e| BallotVerifierError::Unexpected(e.to_string()))?;
                if ct_b64 == target_b64 {
                    return Ok(Some(batch));
                }
            }
        }

        Ok(None)
    }

    fn validate_ballot_hash(s: &str) -> Result<String, String> {
        if Vec::from_hex(s).map(|b| b.len()) == Ok(32) {
            Ok(s.to_string())
        } else {
            Err("must be a valid SHA-256 hex digest".into())
        }
    }
}
