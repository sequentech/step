// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocuments;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResultsAreaContest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub blank_votes: Option<i64>,
    pub elegible_census: Option<i64>,
    pub explicit_invalid_votes: Option<i64>,
    pub implicit_invalid_votes: Option<i64>,
    pub total_auditable_votes: Option<i64>,
    pub total_invalid_votes: Option<i64>,
    pub total_valid_votes: Option<i64>,
    pub total_votes: Option<i64>,
    pub annotations: Option<Value>,
}
pub struct ResultsAreaContestWrapper(pub ResultsAreaContest);
impl TryFrom<Row> for ResultsAreaContestWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(ResultsAreaContestWrapper(ResultsAreaContest {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            annotations: item.try_get("annotations")?,
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
            blank_votes: item.try_get("blank_votes")?,
            elegible_census: item.try_get("elegible_census")?,
            explicit_invalid_votes: item.try_get("explicit_invalid_votes")?,
            implicit_invalid_votes: item.try_get("implicit_invalid_votes")?,
            total_auditable_votes: item.try_get("total_auditable_votes")?,
            total_invalid_votes: item.try_get("total_invalid_votes")?,
            total_valid_votes: item.try_get("total_valid_votes")?,
            total_votes: item.try_get("total_votes")?,
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_results_area_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
) -> Result<Option<ResultsAreaContest>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {err:?}"))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {err:?}"))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {err:?}"))?;
    let contest_uuid: uuid::Uuid = Uuid::parse_str(&contest_id)
        .map_err(|err| anyhow!("Error parsing contest_id as UUID: {err:?}"))?;
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.results_area_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    contest_id = $4
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the query: {err:?}"))?;
    let row: Option<Row> = hasura_transaction
        .query_opt(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &election_uuid,
                &contest_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the query: {err:?}"))?;

    match row {
        Some(row) => {
            let results_contest: ResultsAreaContest = row
                .try_into()
                .map(|res: ResultsAreaContestWrapper| -> ResultsAreaContest { res.0 })
                .map_err(|err| anyhow!("Error preparing the query: {err:?}"))?;
            Ok(Some(results_contest))
        }
        None => Ok(None),
    }
}
