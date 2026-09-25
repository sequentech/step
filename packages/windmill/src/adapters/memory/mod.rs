// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory adapters for unit tests. They keep state behind a mutex so
//! tests can assert what a service stored, not which calls it made.

pub mod clock;
pub mod datafix_cast_vote;
pub mod publication_files;
pub mod tally_ceremony;
