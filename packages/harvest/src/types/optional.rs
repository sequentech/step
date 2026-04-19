// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Serialize, Deserialize, Debug, Default)]
/// Response body for optional ID.
pub struct OptionalId {
    /// The ID.
    pub id: Option<String>,
}
