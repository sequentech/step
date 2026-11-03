// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Context as _, Result};
use clap::Args;
use protocol_board::verification::{
    verify_transcript, PublicTranscript, VerificationRequest,
    VerificationStatus,
};
use protocol_board::{BoardHandle, BoardName, HashHex};
use std::path::PathBuf;

/// Independently verify public election evidence. No trustee keys or login.
#[derive(Args)]
pub struct Verify {
    /// Offline JSON public transcript, instead of fetching boards.
    #[arg(long, conflicts_with_all = ["board_url", "dkg_board", "tally_board"])]
    transcript: Option<PathBuf>,
    #[arg(long, requires = "dkg_board")]
    board_url: Option<String>,
    #[arg(long, requires = "board_url")]
    dkg_board: Option<String>,
    #[arg(long, requires_all = ["dkg_board", "tally_id"])]
    tally_board: Option<String>,
    /// Trusted configuration hash, obtained independently of this export.
    #[arg(long)]
    configuration_hash: String,
    /// Trusted protocol_board UUID, or its decimal unsigned u128.
    #[arg(long)]
    tally_id: Option<String>,
    /// Independently supplied hash of the signed Ballots body.
    #[arg(long)]
    ballots_hash: Option<String>,
    /// A stored cast-vote ciphertext to check for inclusion; repeat as needed.
    #[arg(long)]
    ballot_ciphertext: Vec<String>,
    /// Save fetched evidence for offline or browser verification.
    #[arg(long, conflicts_with = "transcript")]
    export: Option<PathBuf>,
}

impl Verify {
    pub fn run(self) -> Result<()> {
        let transcript = if let Some(path) = self.transcript {
            serde_json::from_str::<PublicTranscript>(&std::fs::read_to_string(
                path,
            )?)?
        } else {
            let url = self.board_url.context(
                "supply --transcript or --board-url and --dkg-board",
            )?;
            let dkg = BoardName::parse(
                &self.dkg_board.context("missing DKG board")?,
            )?;
            let tally =
                self.tally_board.map(|b| BoardName::parse(&b)).transpose()?;
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let dkg = BoardHandle::new(&url, dkg).export_messages().await?;
                let tally = match tally {
                    Some(board) => Some(
                        BoardHandle::new(&url, board).export_messages().await?,
                    ),
                    None => None,
                };
                Ok::<_, anyhow::Error>(PublicTranscript::new(dkg, tally))
            })?
        };
        if let Some(path) = self.export {
            // Avoid replacing an operator's existing evidence file.
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            serde_json::to_writer_pretty(&mut output, &transcript)?;
        }
        let tally_id = self
            .tally_id
            .map(|id| -> Result<String> {
                // A UUID's u128 decimal form is the platform's tally proof domain.
                if id.contains('-') {
                    ensure!(id.len() == 36 && id.bytes().enumerate().all(|(position, byte)| {
                        if [8, 13, 18, 23].contains(&position) { byte == b'-' }
                        else { byte.is_ascii_hexdigit() }
                    }), "tally identifier must be a UUID or an unsigned decimal integer");
                    Ok(u128::from_str_radix(&id.replace('-', ""), 16)?.to_string())
                } else {
                    Ok(id.parse::<u128>()?.to_string())
                }
            })
            .transpose()?;
        let report = verify_transcript(
            &transcript,
            &VerificationRequest {
                configuration_hash: HashHex::parse(&self.configuration_hash)?,
                tally_id,
                ballots_hash: self
                    .ballots_hash
                    .map(|h| HashHex::parse(&h))
                    .transpose()?,
                ballot_ciphertexts: self.ballot_ciphertext,
            },
        );
        println!("{}", serde_json::to_string_pretty(&report)?);
        ensure!(
            report.status == VerificationStatus::Verified,
            "public verification {}: {}",
            if report.status == VerificationStatus::Incomplete {
                "incomplete"
            } else {
                "failed"
            },
            report.detail
        );
        Ok(())
    }
}
