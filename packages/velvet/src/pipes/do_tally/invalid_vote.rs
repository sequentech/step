// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::Serialize;
use strum_macros::AsRefStr;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidVote {
    Implicit,
    Explicit,
    MarkedAsInvalid,
}
