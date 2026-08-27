// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Custom scalar aliases `graphql_client` resolves by name from the scope
//! enclosing each `#[derive(GraphQLQuery)]`. Mirrors
//! `packages/step-cli/src/types/hasura_types.rs`.
#![allow(non_camel_case_types)]

use serde_json::Value;

pub type uuid = String;
pub type jsonb = Value;
pub type timestamptz = String;
pub type bytea = String;
pub type text = String;
pub type varchar = String;
pub type numeric = f64;
pub type json = Value;
