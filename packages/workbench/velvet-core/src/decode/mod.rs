// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot decoder.
//!
//! Pure-computation helpers that turn raw plaintext-ballot lines
//! (decimal-encoded `BigUint`) into `DecodedVoteContest` values. The
//! file-handling pipe in velvet (`pipes::decode_ballots`) wraps these to
//! open the on-disk ballots file and write JSON output; client-side use
//! in the workbench passes the ballot lines directly.

pub mod error;

pub use error::{Error, Result};

use num_bigint::BigUint;
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;
use std::io::BufRead;
use std::str::FromStr;
use tracing::instrument;

/// Decode a single ballot line.
///
/// An empty line yields `Ok(None)` (matches the velvet pipe's behaviour
/// of silently skipping empty lines in a ballot file). Any other parse
/// failure or decode failure is surfaced as `Error::InvalidBallot`.
#[instrument(skip(contest))]
pub fn decode_ballot_line(line: &str, contest: &Contest) -> Result<Option<DecodedVoteContest>> {
    // Preserve velvet's original behaviour: only an *empty* line is
    // skipped. Whitespace-only lines fall through to `from_str` and
    // produce an `InvalidBallot` error, just as they did in the pipe.
    if line.is_empty() {
        return Ok(None);
    }

    let plaintext = BigUint::from_str(line)
        .map_err(|_| Error::InvalidBallot("Wrong ballot format".into()))?;

    let decoded = contest
        .decode_plaintext_contest_bigint(&plaintext)
        .map_err(|_| Error::InvalidBallot("Wrong ballot format".into()))?;

    Ok(Some(decoded))
}

/// Decode every line of a `BufRead` source into `DecodedVoteContest`
/// values, skipping empty lines. I/O errors from the underlying reader
/// are surfaced as `Error::Io`.
#[instrument(skip_all)]
pub fn decode_ballots_from_reader<R: BufRead>(
    reader: R,
    contest: &Contest,
) -> Result<Vec<DecodedVoteContest>> {
    let mut decoded = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if let Some(vote) = decode_ballot_line(&line, contest)? {
            decoded.push(vote);
        }
    }
    Ok(decoded)
}

/// Decode an iterator of already-split ballot lines. Convenient for
/// callers that have ballots in memory (e.g. the workbench feeding
/// strings produced by the JS frontend).
#[instrument(skip_all)]
pub fn decode_ballots_from_lines<I, S>(
    lines: I,
    contest: &Contest,
) -> Result<Vec<DecodedVoteContest>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut decoded = Vec::new();
    for line in lines {
        if let Some(vote) = decode_ballot_line(line.as_ref(), contest)? {
            decoded.push(vote);
        }
    }
    Ok(decoded)
}
