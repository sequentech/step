// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Traits for what commands need from outside the CLI: the Hasura GraphQL API
//! and election event documents. Production implementations live in
//! `crate::adapters`.

pub mod documents;
pub mod graphql;
pub(crate) mod load_setup;
