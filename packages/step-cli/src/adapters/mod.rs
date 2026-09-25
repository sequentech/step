// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Implementations of `crate::ports`. Production adapters wrap the existing
//! Hasura and document helpers, which use the stored CLI configuration;
//! `memory` holds the in-memory adapters used by unit tests.

pub mod documents;
pub mod graphql;
#[cfg(test)]
pub mod memory;
