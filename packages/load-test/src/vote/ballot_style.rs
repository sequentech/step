// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Fetches the voter's ballot styles (`GetBallotStyles`, ported from
//! `packages/voting-portal/src/queries/GetBallotStyles.ts:6-23`) and
//! decodes the one for `election_id`. Scoping is entirely by the voter's
//! JWT via Hasura row-level permissions — the query takes no arguments.

use anyhow::{Context, Result};
use graphql_client::GraphQLQuery;
use sequent_core::ballot::BallotStyle;

use crate::hasura::HasuraClient;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_styles.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetBallotStyles;

pub async fn fetch_ballot_style(client: &HasuraClient, election_id: &str) -> Result<BallotStyle> {
    let data = client
        .data_or_bail::<GetBallotStyles>(get_ballot_styles::Variables {})
        .await
        .context("failed to fetch ballot styles")?;

    let row = data
        .sequent_backend_ballot_style
        .into_iter()
        .find(|row| row.election_id == election_id)
        .ok_or_else(|| {
            anyhow::anyhow!("no ballot style visible to this voter for election {election_id}")
        })?;

    let ballot_eml = row.ballot_eml.ok_or_else(|| {
        anyhow::anyhow!("ballot style for election {election_id} has no ballot_eml")
    })?;

    serde_json::from_str(&ballot_eml)
        .with_context(|| format!("failed to decode ballot style for election {election_id}"))
}
