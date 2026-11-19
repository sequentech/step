// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The result statement: what the results tally stored for one election and
//! area, with the parts it is made of, so that the election verifier can
//! check it against the decrypted votes of each tally board.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const RESULT_STATEMENT_VERSION: u32 = 1;

/// The only counting algorithm the election verifier checks.
pub const CHECKED_COUNTING_ALGORITHM: &str = "plurality-at-large";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResultStatement {
    pub version: u32,
    /// The hash of the Configuration of the board's key generation.
    pub configuration_hash: String,
    pub election_id: String,
    pub area_id: String,
    /// The published ballot style the votes were decoded with.
    pub ballot_style: Value,
    pub contests: Vec<ContestStatement>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContestStatement {
    pub contest_id: String,
    pub counting_algorithm: String,
    /// The result the results tally stored for the area and contest.
    pub result: ResultCounts,
    pub batches: Vec<BatchCounts>,
    pub tally_sheets: Vec<TallySheetCounts>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BatchCounts {
    /// The protocol board's UUID as an unsigned decimal 128-bit integer.
    pub tally_id: String,
    pub multiplier: u64,
    pub counts: ResultCounts,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TallySheetCounts {
    pub tally_sheet_id: String,
    pub counts: ResultCounts,
}

/// Explicit-blank candidates are not listed: their selections are blank
/// votes. A candidate that is not listed has no votes.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct ResultCounts {
    pub total_valid_votes: u64,
    pub explicit_invalid_votes: u64,
    pub implicit_invalid_votes: u64,
    pub blank_votes: u64,
    pub candidates: BTreeMap<String, u64>,
}

impl ResultCounts {
    /// Each count with its name, candidates as `candidate <id>`, over the
    /// candidates of both sides, so that two counts compare field by field.
    pub fn fields(&self, other: &ResultCounts) -> Vec<(String, u64, u64)> {
        let mut fields = vec![
            (
                "total_valid_votes".to_string(),
                self.total_valid_votes,
                other.total_valid_votes,
            ),
            (
                "explicit_invalid_votes".to_string(),
                self.explicit_invalid_votes,
                other.explicit_invalid_votes,
            ),
            (
                "implicit_invalid_votes".to_string(),
                self.implicit_invalid_votes,
                other.implicit_invalid_votes,
            ),
            (
                "blank_votes".to_string(),
                self.blank_votes,
                other.blank_votes,
            ),
        ];
        let ids: std::collections::BTreeSet<&String> = self
            .candidates
            .keys()
            .chain(other.candidates.keys())
            .collect();
        for id in ids {
            fields.push((
                format!("candidate {id}"),
                self.candidates.get(id).copied().unwrap_or(0),
                other.candidates.get(id).copied().unwrap_or(0),
            ));
        }
        fields
    }

    /// `self + other * multiplier`, refusing an overflow.
    pub fn add_scaled(
        &mut self,
        other: &ResultCounts,
        multiplier: u64,
    ) -> Option<()> {
        let add = |a: u64, b: u64| b.checked_mul(multiplier)?.checked_add(a);
        self.total_valid_votes =
            add(self.total_valid_votes, other.total_valid_votes)?;
        self.explicit_invalid_votes =
            add(self.explicit_invalid_votes, other.explicit_invalid_votes)?;
        self.implicit_invalid_votes =
            add(self.implicit_invalid_votes, other.implicit_invalid_votes)?;
        self.blank_votes = add(self.blank_votes, other.blank_votes)?;
        for (id, votes) in &other.candidates {
            let entry = self.candidates.entry(id.clone()).or_insert(0);
            *entry = add(*entry, *votes)?;
        }
        Some(())
    }
}
