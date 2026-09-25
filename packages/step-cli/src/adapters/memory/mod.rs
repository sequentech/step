// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory adapters for unit tests. They keep what a command sent behind a
//! mutex, so tests can assert requests, uploads and downloads.

pub mod documents;
pub mod graphql;
