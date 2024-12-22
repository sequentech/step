// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::results::*;
use serde_json::Value;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::instrument;
use uuid::Uuid;

pub struct ResultsAreaContestWrapper(pub ResultsAreaContest);
impl TryFrom<Row> for ResultsAreaContestWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let documents_value: Option<Value> = item.try_get("documents")?;
        let documents: Option<ResultDocuments> = documents_value
            .map(|value| deserialize_value(value))
            .transpose()?;

        Ok(ResultsAreaContestWrapper(ResultsAreaContest {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
            area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
            results_event_id: item.try_get::<_, Uuid>("results_event_id")?.to_string(),
            elegible_census: item.try_get("elegible_census")?,
            total_valid_votes: item.try_get("total_valid_votes")?,
            explicit_invalid_votes: item.try_get("explicit_invalid_votes")?,
            implicit_invalid_votes: item.try_get("implicit_invalid_votes")?,
            blank_votes: item.try_get("blank_votes")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            total_valid_votes_percent: item
                .try_get::<&str, Option<f64>>("total_valid_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            total_invalid_votes: item.try_get("total_invalid_votes")?,
            total_invalid_votes_percent: item
                .try_get::<&str, Option<f64>>("total_invalid_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            explicit_invalid_votes_percent: item
                .try_get::<&str, Option<f64>>("explicit_invalid_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            blank_votes_percent: item
                .try_get::<&str, Option<f64>>("blank_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            implicit_invalid_votes_percent: item
                .try_get::<&str, Option<f64>>("implicit_invalid_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            total_votes: item.try_get("total_votes")?,
            total_votes_percent: item
                .try_get::<&str, Option<f64>>("total_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
            documents,
            total_auditable_votes: item.try_get("total_auditable_votes")?,
            total_auditable_votes_percent: item
                .try_get::<&str, Option<f64>>("total_auditable_votes_percent")?
                .map(|val| val.try_into())
                .transpose()?,
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_results_area_contest_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    area_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let documents_value = serde_json::to_value(documents.clone())?;
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let results_event_uuid: uuid::Uuid = Uuid::parse_str(&results_event_id)
        .map_err(|err| anyhow!("Error parsing results_event_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let contest_uuid: uuid::Uuid = Uuid::parse_str(&contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(&area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.results_area_contest
                SET
                    documents = $1
                WHERE
                    tenant_id = $2 AND
                    results_event_id = $3 AND
                    election_event_id = $4 AND
                    election_id = $5 AND
                    contest_id = $6 AND
                    area_id = $7
                RETURNING
                    id;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &documents_value,
                &tenant_uuid,
                &results_event_uuid,
                &election_event_uuid,
                &election_uuid,
                &contest_uuid,
                &area_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;

    if 1 == rows.len() {
        Ok(())
    } else if rows.len() > 1 {
        Err(anyhow!(
            "Too many affected rows in table results_area_contest: {}",
            rows.len()
        ))
    } else {
        Err(anyhow!("Rows not found in table results_area_contest"))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_results_area_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: Option<&str>,
    area_id: &str,
) -> Result<Option<ResultsAreaContest>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {err:?}"))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {err:?}"))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {err:?}"))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(&area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {err:?}"))?;

    let (contest_uuid, contest_clause): (Option<uuid::Uuid>, &str) = match contest_id {
        Some(contest_id) => {
            let c_uuid = Uuid::parse_str(&contest_id)
                .map_err(|err| anyhow!("Error parsing contest_id as UUID: {err:?}"))?;
            let clause = " AND contest_id = $5";
            (Some(c_uuid), clause)
        }
        None => (None, ""),
    };

    let statement_str = format!(
        r#"
                SELECT
                    *
                FROM
                    sequent_backend.results_area_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    area_id = $4
                    {contest_clause}
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![
        &tenant_uuid,
        &election_event_uuid,
        &election_uuid,
        &area_uuid,
    ];

    if contest_uuid.is_some() {
        params.push(&contest_uuid);
    }

    let rows = hasura_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("Error running the query: {:?}", err))?;

    match rows.into_iter().next() {
        Some(row) => row
            .try_into()
            .map(|res: ResultsAreaContestWrapper| Some(res.0))
            .map_err(|err| anyhow!("Error converting row into ResultsAreaContest: {:?}", err)),
        None => Ok(None),
    }
}
