// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Public protocol evidence from the configured internal board service.
use anyhow::{ensure, Context, Result};
use protocol_board::{verification::PublicTranscript, BoardName};
use std::time::Duration;
use windmill::services::protocol_board::board_handle;

pub const MAX_TRANSCRIPT_BYTES: usize = 64 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 8 * 1024 * 1024;
const MAX_MESSAGES: usize = 10_000;

#[rocket::async_trait]
pub trait TranscriptSource: Send + Sync {
    async fn fetch(
        &self,
        dkg: &BoardName,
        tally: Option<&BoardName>,
    ) -> Result<PublicTranscript>;
}

/// There is no caller-selected address: board_handle reads WBRAID_B4_URL.
#[derive(Default)]
pub struct InternalTranscriptSource;

#[rocket::async_trait]
impl TranscriptSource for InternalTranscriptSource {
    async fn fetch(
        &self,
        dkg: &BoardName,
        tally: Option<&BoardName>,
    ) -> Result<PublicTranscript> {
        let dkg = board_handle(dkg)?
            .export_messages_bounded(
                Duration::from_secs(30),
                MAX_METADATA_BYTES,
                MAX_TRANSCRIPT_BYTES,
                MAX_MESSAGES,
                blob_prefix(dkg)?,
            )
            .await?;
        let used: usize = dkg.iter().map(Vec::len).sum();
        ensure!(used <= MAX_TRANSCRIPT_BYTES, "transcript byte limit");
        let tally = match tally {
            Some(board) => Some(
                board_handle(board)?
                    .export_messages_bounded(
                        Duration::from_secs(30),
                        MAX_METADATA_BYTES,
                        MAX_TRANSCRIPT_BYTES - used,
                        MAX_MESSAGES.saturating_sub(dkg.len()),
                        blob_prefix(board)?,
                    )
                    .await?,
            ),
            None => None,
        };
        Ok(PublicTranscript::new(dkg, tally))
    }
}

fn blob_prefix(board: &BoardName) -> Result<String> {
    let endpoint = std::env::var("AWS_S3_PRIVATE_URI")
        .context("private storage endpoint missing")?;
    let bucket = std::env::var("S3_BUCKET_NAME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "wbraid-messages".to_string());
    ensure!(
        bucket
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.')),
        "invalid board bucket"
    );
    let endpoint = reqwest::Url::parse(endpoint.trim())?;
    ensure!(
        matches!(endpoint.scheme(), "http" | "https")
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none(),
        "invalid storage endpoint"
    );
    Ok(format!(
        "{}/{}/{}/messages/",
        endpoint.as_str().trim_end_matches('/'),
        bucket,
        board.as_str()
    ))
}
