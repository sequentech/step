// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot publication: `generate_ballot_publication` -> poll
//! `get_ballot_publication_status` -> `publish_ballot`. Interval/timeout
//! match `step-cli`'s own loop
//! (`packages/step-cli/src/commands/publish_changes.rs:59-131`).

use std::time::Duration;

use anyhow::{bail, Context, Result};
use graphql_client::GraphQLQuery;
use tokio::time::sleep;

use crate::hasura::HasuraClient;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/generate_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GenerateBallotPublication;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_publication_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetBallotPublicationStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/publish_ballot.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct PublishBallot;

const PUBLICATION_TIMEOUT: Duration = Duration::from_secs(60);
const PUBLICATION_POLL_INTERVAL: Duration = Duration::from_secs(3);

pub async fn publish(client: &HasuraClient, election_event_id: &str) -> Result<()> {
    let generate_variables = generate_ballot_publication::Variables {
        election_event_id: election_event_id.to_string(),
        election_id: None,
    };
    let generated = client
        .data_or_bail::<GenerateBallotPublication>(generate_variables)
        .await
        .context("failed to generate a ballot publication")?;
    let ballot_publication_id = generated
        .generate_ballot_publication
        .ok_or_else(|| anyhow::anyhow!("generate_ballot_publication returned no data"))?
        .ballot_publication_id;

    let start = tokio::time::Instant::now();
    loop {
        let status_variables = get_ballot_publication_status::Variables {
            id: ballot_publication_id.clone(),
        };
        let status_data = client
            .data_or_bail::<GetBallotPublicationStatus>(status_variables)
            .await
            .context("failed to check ballot publication status")?;
        let is_generated = status_data
            .sequent_backend_ballot_publication
            .first()
            .map(|row| row.is_generated)
            .unwrap_or(false);
        if is_generated {
            break;
        }
        if start.elapsed() >= PUBLICATION_TIMEOUT {
            bail!(
                "timed out waiting for ballot publication {ballot_publication_id} \
                 to be generated"
            );
        }
        sleep(PUBLICATION_POLL_INTERVAL).await;
    }

    let publish_variables = publish_ballot::Variables {
        election_event_id: election_event_id.to_string(),
        ballot_publication_id,
    };
    client
        .data_or_bail::<PublishBallot>(publish_variables)
        .await
        .context("failed to publish the ballot")?;
    Ok(())
}
