// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;
use strum_macros::{AsRefStr, Display, EnumString};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, EnumString, Display, AsRefStr)]
pub enum PipeName {
    DecodeBallots,
    DecodeMCBallots,
    VoteReceipts,
    BallotImages,
    MCBallotReceipts,
    MCBallotImages,
    DoTally,
    MarkWinners,
    GenerateReports,
    GenerateDatabase,
}

struct PipeNameVisitor;

impl<'de> Visitor<'de> for PipeNameVisitor {
    type Value = PipeName;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string starting with 'Velvet' and followed by a PipeName variant")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        PipeName::from_str(v.trim_start_matches("Velvet")).map_err(E::custom)
    }
}

pub fn deserialize_pipe<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PipeName, D::Error> {
    deserializer.deserialize_str(PipeNameVisitor)
}

#[derive(Debug, AsRefStr)]
pub enum PipeNameOutputDir {
    #[strum(serialize = "velvet-decode-ballots")]
    DecodeBallots,
    #[strum(serialize = "velvet-decode-mcballots")]
    DecodeMCBallots,
    #[strum(serialize = "velvet-vote-receipts")]
    VoteReceipts,
    #[strum(serialize = "velvet-mcballot-receipts")]
    MCBallotReceipts,
    #[strum(serialize = "velvet-do-tally")]
    DoTally,
    #[strum(serialize = "velvet-mark-winners")]
    MarkWinners,
    #[strum(serialize = "velvet-generate-reports")]
    GenerateReports,
    #[strum(serialize = "velvet-generate-database")]
    GenerateDatabase,
    #[strum(serialize = "velvet-ballot-images")]
    BallotImages,
    #[strum(serialize = "velvet-mcballot-images")]
    MCBallotImages,
}
