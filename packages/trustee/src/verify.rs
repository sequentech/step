// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Context as _, Result};
use clap::Args;
use protocol_board::export_signature::{ExportKind, SignedExport};
use protocol_board::verification::{
    verify_transcript_with_anchors, PublicTranscript, VerificationRequest,
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
    /// The board's signed result statement, to check against the votes.
    #[arg(long)]
    result: Option<PathBuf>,
    /// Save fetched evidence for offline or browser verification.
    #[arg(long, conflicts_with = "transcript")]
    export: Option<PathBuf>,
}

impl Verify {
    pub fn run(self) -> Result<()> {
        // Downloaded verification data must carry the export signature; read
        // without one, they are reported as not authenticated.
        let mut signed_transcript = None;
        let from_file = self.transcript.is_some();
        let transcript = if let Some(path) = self.transcript {
            let bytes = std::fs::read(path)?;
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            if value.get("format").is_some() {
                let envelope = SignedExport::parse(&bytes)?;
                let transcript = serde_json::from_str::<PublicTranscript>(
                    envelope.content.get(),
                )?;
                signed_transcript = Some(envelope);
                transcript
            } else {
                serde_json::from_slice::<PublicTranscript>(&bytes)?
            }
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
        let statement = self
            .result
            .map(|path| -> Result<_> {
                let envelope = SignedExport::parse(&std::fs::read(path)?)
                    .context("the result statement is not a signed export")?;
                let statement = crate::result_check::parse_statement(
                    envelope.content.get(),
                )?;
                Ok((envelope, statement))
            })
            .transpose()?;
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
        let configuration_hash = self.configuration_hash.clone();
        let (mut report, anchors) = verify_transcript_with_anchors(
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
        let fail = |report: &mut protocol_board::verification::VerificationReport,
                        detail: String| {
            report.status = VerificationStatus::Invalid;
            report.detail = detail;
            report.plaintexts.clear();
            report.ballot_inclusions.clear();
        };
        match (&anchors, &signed_transcript) {
            (Some(anchors), Some(envelope)) => {
                match envelope
                    .verify(ExportKind::VerificationData, &anchors.protocol_manager)
                {
                    Ok(_) => report.transcript_signature = Some("verified".into()),
                    Err(error) => {
                        report.transcript_signature = Some("invalid".into());
                        fail(&mut report, error.to_string());
                    }
                }
            }
            (_, None) if from_file => {
                // FCS_COP.1/EXP: exported data are authentic only when signed.
                // The proofs' outcome stays in the report; only `verified`,
                // which would claim authentic data, is withheld.
                report.transcript_signature = Some("absent".into());
                if report.status == VerificationStatus::Verified {
                    report.status = VerificationStatus::Invalid;
                    report.detail = "verification data are not signed".into();
                }
            }
            (_, None) => {
                // Read from the boards directly: there is no export to sign.
                report.transcript_signature = Some("not applicable".into());
            }
            (None, Some(_)) => {
                report.transcript_signature = Some("not checked".into());
            }
        }
        report.result_check = Some("not checked".into());
        if let (Some((envelope, statement)), Some(anchors)) = (&statement, &anchors) {
            if report.status == VerificationStatus::Verified {
                let checked = envelope
                    .verify(ExportKind::ResultStatement, &anchors.protocol_manager)
                    .and_then(|_| {
                        crate::result_check::check_result(
                            statement,
                            &configuration_hash,
                            report.tally_id.as_deref().unwrap_or_default(),
                            anchors,
                            &report.plaintexts,
                        )
                    });
                match checked {
                    Ok(check) => {
                        report.ballot_style_hash = Some(check.ballot_style_hash);
                        report.result_check = Some(check.report);
                    }
                    Err(error) => fail(&mut report, error.to_string()),
                }
            }
        }
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
