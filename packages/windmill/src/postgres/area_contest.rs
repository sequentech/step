// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::AreaContest;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct AreaContestWrapper(pub AreaContest);

impl TryFrom<Row> for AreaContestWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(AreaContestWrapper(AreaContest {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_area_to_area_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    contest_ids: &[Uuid],
) -> Result<()> {
    let area_contests: Vec<AreaContest> = contest_ids
        .iter()
        .map(|contest_id| AreaContest {
            id: Uuid::new_v4().to_string(),
            area_id: area_id.to_string(),
            contest_id: contest_id.to_string(),
        })
        .collect();

    insert_area_contests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &area_contests,
    )
    .await?;
    Ok(())
}

#[instrument(err, skip_all)]
pub async fn insert_area_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_contests: &Vec<AreaContest>,
) -> Result<()> {
    for area_contest in area_contests {
        let statement = hasura_transaction
            .prepare(
                r#"
                INSERT INTO sequent_backend.area_contest
                (id, tenant_id, election_event_id, contest_id, area_id, created_at, last_updated_at)
                VALUES
                ($1, $2, $3, $4, $5, NOW(), NOW());
            "#,
            )
            .await?;

        let _rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &Uuid::parse_str(&area_contest.id)?,
                    &Uuid::parse_str(tenant_id)?,
                    &Uuid::parse_str(election_event_id)?,
                    &Uuid::parse_str(&area_contest.contest_id)?,
                    &Uuid::parse_str(&area_contest.area_id)?,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn export_area_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<AreaContest>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, area_id, contest_id
                FROM
                    sequent_backend.area_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let area_contests: Vec<AreaContest> = rows
        .into_iter()
        .map(|row| -> Result<AreaContest> {
            row.try_into()
                .map(|res: AreaContestWrapper| -> AreaContest { res.0 })
        })
        .collect::<Result<Vec<AreaContest>>>()?;

    Ok(area_contests)
}

#[instrument(err, skip_all)]
pub async fn get_areas_by_contest_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    contest_id: &str,
) -> Result<Vec<String>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    area_id
                FROM
                    sequent_backend.area_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    contest_id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(contest_id)?,
            ],
        )
        .await?;

    // Map each row to the area_id column and collect into a Vec<String>
    let area_ids: Vec<String> = rows.into_iter().map(|row| row.get("area_id")).collect();

    Ok(area_ids)
}

#[instrument(err, skip_all)]
pub async fn get_area_contests_by_area_contest_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_ids: &Vec<String>,
    contest_ids: &Vec<String>,
) -> Result<Vec<AreaContest>> {
    let uuid_tenant_id = Uuid::parse_str(tenant_id)?;
    let uuid_election_event_id = Uuid::parse_str(election_event_id)?;
    let uuid_area_ids: Vec<Uuid> = area_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<_, _>>()?;
    let uuid_contest_ids: Vec<Uuid> = contest_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<_, _>>()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, area_id, contest_id
                FROM
                    sequent_backend.area_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    area_id = ANY($3) AND
                    contest_id = ANY($4);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &uuid_tenant_id,
                &uuid_election_event_id,
                &uuid_area_ids,
                &uuid_contest_ids,
            ],
        )
        .await?;

    let area_contests: Vec<AreaContest> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: AreaContestWrapper| res.0))
        .collect::<Result<Vec<AreaContest>>>()?;

    Ok(area_contests)
}
