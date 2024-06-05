// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::{hasura::core::BallotPublication, keycloak::UserArea};
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct BallotPublicationWrapper(pub BallotPublication);

impl TryFrom<Row> for BallotPublicationWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(BallotPublicationWrapper(BallotPublication {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            created_at: item.get("created_at"),
            deleted_at: item.get("deleted_at"),
            created_by_user_id: item.try_get("created_by_user_id")?,
            is_generated: item.try_get("is_generated")?,
            election_ids: item.get("election_ids"),
            published_at: item.get("published_at"),
            election_id: item.get("election_id"),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_ballot_publication_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> Result<Option<BallotPublication>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_publication
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = $3 AND
                deleted_at IS NULL;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(ballot_publication_id)?,
            ],
        )
        .await?;

    let results: Vec<BallotPublication> = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results
        .get(0)
        .map(|element: &BallotPublication| element.clone()))
}
