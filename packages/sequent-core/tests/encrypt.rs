// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the encrypt module

use sequent_core::ballot::*;
use sequent_core::ballot_codec::multi_ballot::*;
use sequent_core::encrypt::*;
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use serde_json::json;

