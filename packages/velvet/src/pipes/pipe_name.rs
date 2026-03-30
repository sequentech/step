// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;
use strum_macros::{AsRefStr, Display, EnumString};

/// Names of the different election processing pipelines.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, EnumString, Display, AsRefStr)]
pub enum PipeName {
    /// Decode standard ballots pipeline.
    DecodeBallots,
    /// Decode multi-contest ballots pipeline.
    DecodeMCBallots,
    /// Generate ballot images pipeline.
    BallotImages,
    /// Generate multi-contest ballot receipts pipeline.
    MCBallotReceipts,
    /// Generate multi-contest ballot images pipeline.
    MCBallotImages,
    /// Tally votes pipeline.
    DoTally,
    /// Mark election winners pipeline.
    MarkWinners,
    /// Generate election reports pipeline.
    GenerateReports,
    /// Generate election database pipeline.
    GenerateDatabase,
}

/// Visitor for deserializing `PipeName` from strings.
struct PipeNameVisitor;

impl Visitor<'_> for PipeNameVisitor {
    type Value = PipeName;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string starting with 'Velvet' and followed by a PipeName variant")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        PipeName::from_str(v.trim_start_matches("Velvet")).map_err(E::custom)
    }
}

/// Deserializes a `PipeName` from a string.
///
/// # Errors
/// Returns a deserialization error if the string is not a valid `PipeName` variant.
pub fn deserialize_pipe<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PipeName, D::Error> {
    deserializer.deserialize_str(PipeNameVisitor)
}

#[derive(Debug, AsRefStr)]
/// Output directory names for each pipeline's results.
pub enum PipeNameOutputDir {
    /// Output directory for decoded ballots pipeline.
    #[strum(serialize = "velvet-decode-ballots")]
    DecodeBallots,
    /// Output directory for decoded multi-contest ballots pipeline.
    #[strum(serialize = "velvet-decode-mcballots")]
    DecodeMCBallots,
    /// Output directory for multi-contest ballot receipts pipeline.
    #[strum(serialize = "velvet-mcballot-receipts")]
    MCBallotReceipts,
    /// Output directory for tally pipeline.
    #[strum(serialize = "velvet-do-tally")]
    DoTally,
    /// Output directory for mark winners pipeline.
    #[strum(serialize = "velvet-mark-winners")]
    MarkWinners,
    /// Output directory for generate reports pipeline.
    #[strum(serialize = "velvet-generate-reports")]
    GenerateReports,
    /// Output directory for generate database pipeline.
    #[strum(serialize = "velvet-generate-database")]
    GenerateDatabase,
    /// Output directory for ballot images pipeline.
    #[strum(serialize = "velvet-ballot-images")]
    BallotImages,
    /// Output directory for multi-contest ballot images pipeline.
    #[strum(serialize = "velvet-mcballot-images")]
    MCBallotImages,
}
