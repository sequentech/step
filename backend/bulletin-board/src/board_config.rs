// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::proto::Board;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents the private+public configuration of a bulletin board
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Debug,
    Default,
    Clone,
)]
pub struct BoardConfig {
    // Private key of the board
    pub private_key: String,

    // Sequence id of the entry containing the latest Public Board Config
    pub board_config_sequence_id: u64,

    // Last sequence id. We don't serialize it since this can/should be
    // obtained by other means (i.e. reading the board checkpoint), and
    // also this allows not having to write to disk to update each time
    // sequence id is updated
    #[borsh_skip]
    #[serde(skip)]
    pub last_sequence_id: u64,

    // Latest Public Board Config. This is read directly from the entry pointed
    // by `board_config_sequence_id` on startup, thus avoiding duplication.
    #[borsh_skip]
    #[serde(skip)]
    pub board: Board,
}

// SeqPath builds the directory path and relative filename for the entry at the
// given sequence number.
// As a reference see https://github.com/google/trillian-examples/blob/19b536fc88386f7a061d240aad3016cfe926b152/serverless/api/layout/paths.go#L36
#[allow(clippy::ptr_arg)]
pub fn to_seq_path(log_root_path: &PathBuf, sequence_id: u64) -> PathBuf {
    return [
        log_root_path.to_str().unwrap(),
        "seq",
        format!("{:02x}", (sequence_id >> 32)).as_str(),
        format!("{:02x}", (sequence_id >> 24) & 0xff).as_str(),
        format!("{:02x}", (sequence_id >> 16) & 0xff).as_str(),
        format!("{:02x}", (sequence_id >> 8) & 0xff).as_str(),
        format!("{:02x}", sequence_id & 0xff).as_str(),
    ]
    .iter()
    .collect();
}
