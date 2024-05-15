// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Election;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct ElectionWrapper(pub Election);

impl TryFrom<Row> for ElectionWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let num_allowed_revotes: Option<i32> = item.try_get("num_allowed_revotes")?;

        Ok(ElectionWrapper(Election {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            name: item.try_get("name")?,
            description: item.try_get("description")?,
            presentation: item.try_get("presentation")?,
            dates: item.try_get("dates")?,
            status: item.try_get("status")?,
            eml: item.try_get("eml")?,
            num_allowed_revotes: num_allowed_revotes.map(|val| val as i64),
            is_consolidated_ballot_encoding: item.try_get("is_consolidated_ballot_encoding")?,
            spoil_ballot_option: item.try_get("spoil_ballot_option")?,
            is_kiosk: item.try_get("is_kiosk")?,
            alias: item.try_get("alias")?,
            voting_channels: item.try_get("voting_channels")?,
            image_document_id: item.try_get("image_document_id")?,
            statistics: item.try_get("statistics")?,
            receipts: item.try_get("receipts")?,
        }))
    }
}

/**
 * Returns a vector of areas per election event, with the posibility of
 * filtering by area_id
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_max_revotes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<usize> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id, num_allowed_revotes
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await?;

    event!(Level::INFO, "rows: {:?}", rows);

    let revotes: Vec<usize> = rows
        .iter()
        .map(|row| {
            let num_allowed_revotes: Option<i32> = row.try_get("num_allowed_revotes")?;

            Ok(num_allowed_revotes.unwrap_or(1) as usize)
        })
        .collect::<Result<Vec<usize>>>()?;

    let data = revotes.get(0).unwrap_or(&1).clone();

    Ok(data)
}

/* Returns election */

#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Option<Election>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id,
                tenant_id,
                election_event_id,
                created_at,
                last_updated_at,
                labels,
                annotations,
                name,
                description,
                presentation,
                dates,
                status,
                eml,
                num_allowed_revotes,
                is_consolidated_ballot_encoding,
                spoil_ballot_option,
                alias,
                voting_channels,
                is_kiosk,
                image_document_id,
                statistics,
                receipts
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await?;

    let elections: Vec<Election> = rows
        .into_iter()
        .map(|row| -> Result<Election> {
            row.try_into()
                .map(|res: ElectionWrapper| -> Election { res.0 })
        })
        .collect::<Result<Vec<Election>>>()?;

    Ok(elections.get(0).map(|election| election.clone()))
}

pub async fn update_election_presentation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    presentation: Value,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let election_uuid: uuid::Uuid =
        Uuid::parse_str(election_id).with_context(|| "Error parsing election_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".election
            SET
                presentation = $4
            WHERE
                tenant_id = $1
                AND election_event_id = $2
                AND id = $3
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &election_uuid,
                &presentation,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the update_election_presentation query: {err}"))?;

    Ok(())
}
