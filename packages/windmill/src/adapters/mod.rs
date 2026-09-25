// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Implementations of `crate::ports`. Production adapters wrap the existing
//! `crate::postgres` queries and service clients; `memory` holds the
//! in-memory adapters used by unit tests.

pub mod keys_ceremony;
#[cfg(test)]
pub mod memory;
pub mod system;
