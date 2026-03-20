// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use async_trait::async_trait;
use b3::messages::newtypes::BatchNumber;
use std::path::PathBuf;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;

/// Port for reading area ballots from storage into a CSV file.
#[async_trait]
pub trait BallotRepository: Send + Sync {
    async fn find_area_ballots(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        area_id: &str,
        election_id: &str,
        output_file: &PathBuf,
    ) -> Result<()>;
}

/// Port for reading eligible voters from identity storage into a CSV file.
#[async_trait]
pub trait VoterRepository: Send + Sync {
    async fn find_eligible_voters(
        &self,
        realm: &str,
        area_id: &str,
        election_alias: &str,
        output_file: &PathBuf,
        delegated_voting_enabled: bool,
    ) -> Result<()>;
}

/// Port for submitting an encrypted ballot batch to the tamper-evident bulletin board.
#[async_trait]
pub trait BulletinBoardPort: Send + Sync {
    async fn add_ballots(
        &self,
        board_name: &str,
        ciphertexts: Vec<Ciphertext<RistrettoCtx>>,
        batch: BatchNumber,
    ) -> Result<()>;
}
