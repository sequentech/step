// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionPresentation;
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
            permission_label: item.try_get("permission_label")?,
            initialization_report_generated: item.try_get("initialization_report_generated")?,
            keys_ceremony_id: item
                .try_get::<_, Option<Uuid>>("keys_ceremony_id")?
                .map(|val| val.to_string()),
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
                *
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

#[instrument(skip(hasura_transaction), err)]
pub async fn get_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    get_test_elections: Option<bool>,
) -> Result<Vec<Election>> {
    let get_test_elections_clause = match get_test_elections {
        Some(true) => "AND name ILIKE '%Test%'".to_string(),
        Some(false) => "AND name NOT ILIKE '%Test%'".to_string(),
        None => "".to_string(),
    };

    let statement_str = format!(
        r#"
            SELECT
                *
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2
                {get_test_elections_clause}
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
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

    Ok(elections)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_elections_by_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
) -> Result<Vec<Election>> {
    let election_uuids = election_ids
        .clone()
        .into_iter()
        .map(|id| Uuid::parse_str(&id).map_err(|err| anyhow!("{:?}", err)))
        .collect::<Result<Vec<Uuid>>>()?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = ANY($3);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &election_uuids,
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

    Ok(elections)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_elections_by_keys_ceremony_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<Vec<Election>> {
    println!("get_elections_by_keys_ceremony_id: {:?}", &keys_ceremony_id);
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                keys_ceremony_id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(keys_ceremony_id)?,
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

    Ok(elections)
}

#[instrument(skip(hasura_transaction), err)]
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

#[instrument(skip(hasura_transaction), err)]
pub async fn update_election_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    status: Value,
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
                status = $4
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
            &[&tenant_uuid, &election_event_uuid, &election_uuid, &status],
        )
        .await
        .map_err(|err| anyhow!("Error running the update_election_presentation query: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn create_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
    presentation: &ElectionPresentation,
    description: Option<String>,
) -> Result<Election> {
    let presentation_value = serde_json::to_value(presentation)
        .map_err(|err| anyhow!("Error serializing election presentation: {err}"))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.election
                (
                    tenant_id,
                    election_event_id,
                    created_at,
                    last_updated_at,
                    name,
                    description,
					presentation
                )
                VALUES
                (
                    $1,
                    $2,
                    NOW(),
                    NOW(),
                    $3,
                    $4,
					$5
                )
                RETURNING *;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(&tenant_id)?,
                &Uuid::parse_str(&election_event_id)?,
                &name.to_string(),
                &description,
                &presentation_value,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    let elections: Vec<Election> = rows
        .into_iter()
        .map(|row| -> Result<Election> {
            row.try_into()
                .map(|res: ElectionWrapper| -> Election { res.0 })
        })
        .collect::<Result<Vec<Election>>>()?;

    Ok(elections
        .first()
        .cloned()
        .ok_or(anyhow!("Coudln't insert election"))?)
}

#[instrument(err, skip_all)]
pub async fn insert_elections(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for election in &data.elections {
        election.validate()?;
        let keys_ceremony_id_uuid_opt = election
            .keys_ceremony_id
            .clone()
            .map(|val| Uuid::parse_str(&val))
            .transpose()?;

        let statement = hasura_transaction
            .prepare(
                r#"
                INSERT INTO sequent_backend.election
                (
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
                    receipts,
                    permission_label,
                    keys_ceremony_id,
                    initialization_report_generated
                )
                VALUES
                (
                    $1,
                    $2,
                    $3,
                    NOW(),
                    NOW(),
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9,
                    $10,
                    $11,
                    $12,
                    $13,
                    $14,
                    $15,
                    $16,
                    $17,
                    $18,
                    $19,
                    $20,
                    $21,
                    $22
                );
            "#,
            )
            .await?;

        let _rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &Uuid::parse_str(&election.id)?,
                    &Uuid::parse_str(&election.tenant_id)?,
                    &Uuid::parse_str(&election.election_event_id)?,
                    &election.labels,
                    &election.annotations,
                    &election.name,
                    &election.description,
                    &election.presentation,
                    &election.status,
                    &election.eml,
                    &election
                        .num_allowed_revotes
                        .and_then(|val| Some(val as i32)),
                    &election.is_consolidated_ballot_encoding,
                    &election.spoil_ballot_option,
                    &election.alias,
                    &election.voting_channels,
                    &election.is_kiosk,
                    &election.image_document_id,
                    &election.statistics,
                    &election.receipts,
                    &election.permission_label,
                    &keys_ceremony_id_uuid_opt,
                    &election.initialization_report_generated,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn export_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Election>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.election
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

    let elections: Vec<Election> = rows
        .into_iter()
        .map(|row| -> Result<Election> {
            row.try_into()
                .map(|res: ElectionWrapper| -> Election { res.0 })
        })
        .collect::<Result<Vec<Election>>>()?;

    Ok(elections)
}

#[instrument(err, skip(hasura_transaction))]
pub async fn set_election_keys_ceremony(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    keys_ceremony_id: &str,
) -> Result<Vec<Election>> {
    let election_uuid_opt = election_id
        .clone()
        .map(|val| Uuid::parse_str(&val))
        .transpose()?;
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.election
                SET
                    keys_ceremony_id = $1
                WHERE
                    ($2::uuid IS NULL OR id = $2::uuid) AND
                    tenant_id = $3 AND
                    election_event_id = $4
                RETURNING
                    *;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(keys_ceremony_id)?,
                &election_uuid_opt,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the set_election_keys_ceremony query: {err}"))?;

    if 0 == rows.len() {
        return Err(anyhow!("No election found"));
    }

    let elections: Vec<Election> = rows
        .into_iter()
        .map(|row| -> Result<Election> {
            row.try_into()
                .map(|res: ElectionWrapper| -> Election { res.0 })
        })
        .collect::<Result<Vec<Election>>>()?;

    Ok(elections)
}

#[instrument(err, skip(hasura_transaction))]
pub async fn set_election_initialization_report_generated(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    initialization_status: &bool,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.election
                SET
                    initialization_report_generated = $1
                WHERE
                    tenant_id = $2 AND
                    election_event_id = $3 AND
                    id = $4
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                initialization_status,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the set_election_keys_ceremony query: {err}"))?;

    Ok(())
}

#[instrument(err, skip(hasura_transaction))]
pub async fn update_election_status(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    status: bool,
) -> Result<Vec<Election>> {
    let query = r#"
        UPDATE
            sequent_backend.election
        SET
            last_updated_at = NOW(),
            status = jsonb_set(
                COALESCE(status, '{}'::jsonb),   -- start with empty object if NULL
                '{is_published}',                -- path
                to_jsonb($4::bool),              -- new value
                true                             -- create the key if missing
            )
        WHERE
            id = $1 AND
            tenant_id = $2 AND
            election_event_id = $3
        RETURNING *;
    "#;

    // Prepare the statement
    let statement = hasura_transaction
        .prepare(&query)
        .await
        .map_err(|err| anyhow!("Error preparing the update query: {err}"))?;

    // Parse UUIDs
    let parsed_id = Uuid::parse_str(id)?;
    let parsed_tenant_id = Uuid::parse_str(tenant_id)?;
    let parsed_election_event_id = Uuid::parse_str(election_event_id)?;

    // Execute the query
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parsed_id,
                &parsed_tenant_id,
                &parsed_election_event_id,
                &status,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error updating Election: {err}"))?;

    let results: Vec<Election> = rows
        .into_iter()
        .map(|row| -> Result<Election> {
            row.try_into()
                .map(|res: ElectionWrapper| -> Election { res.0 })
        })
        .collect::<Result<Vec<Election>>>()?;

    Ok(results)
}

// #[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
// pub struct ElectionMonitorStatus {
//     pub area_id: String,
//     pub area_name: Option<String>,
//     pub area_description: Option<String>,
//     pub area_annotations: Option<String>,
//     pub election: Election,
//     pub is_generate_re: bool,
// }

// pub struct ElectionMonitorStatusWrapper(pub ElectionMonitorStatus);

// impl TryFrom<Row> for ElectionMonitorStatusWrapper {
//     type Error = anyhow::Error;
//     fn try_from(item: Row) -> Result<Self> {
//         let status = item.try_get("status")?;
//         let election_status = get_election_status(status);

//         let election_annotations = item.try_get("election_annotations")
//         .ok_or_else(|| anyhow!("Missing election  annotations"))?;
//         let annotations: Annotations = deserialize_value(annotations_js)?;

//         Ok(ElectionMonitorStatusWrapper(ElectionMonitorStatus {
//             area_id: item.try_get::<_, Uuid>("id")?.to_string(),
//             area_name: item.try_get("area_name")?,
//             area_description: item.try_get("area_description")?,
//             area_annotations: item.try_get("area_annotations")?,
//             election: serde_json::from_value(item.try_get::<_, serde_json::Value>("election")?)?,
//             is_generate_re: item.try_get::<_, Option<Uuid>>("results_election_id")?.is_some(),
//         }))
//     }
// }

// pub struct ElectionsMonitorFilter {
//     pub area_name: Option<String>,
//     pub precinct_code: Option<String>,
//     pub voting_status: Option<String>,
//     pub is_generate_re: Option<bool>,
//     pub limit: Option<i32>,
//     pub offset: Option<i32>,
//     pub transmitted: Option<bool>,
// }

// /**
//  * Returns a vec of the areas related to all elections with additional election info.
//  */
//  #[instrument(skip(hasura_transaction), err)]
//  pub async fn get_elections_monitior(
//      hasura_transaction: &Transaction<'_>,
//      tenant_id: &str,
//      election_event_id: &str,
//      filter: ElectionsMonitorFilter,
//  ) -> Result<Vec<AreaElection>> {
//     let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
//     let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
//     let query_limit: i64 =
//         std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();
//     let query_offset: i64 = if let Some(offset_val) = filter.offset {
//         offset_val.into()
//     } else {
//         0
//     };

//     let area_name_filter_clause = if let Some(area_name_filter) = filter.area_name {
//         format!("AND a.name ILIKE '{area_name_filter}'")
//     } else {
//         "".to_string()
//     };

//     let precinct_code_filter_clause = if let Some(precinct_code_filter) = filter.precinct_code {
//        format!("AND e.annotations::jsonb ->> 'precinct_code' ILIKE '{precinct_code_filter}'")
//     } else {
//         "".to_string()
//     };

//     let voting_status_filter_clause = if let Some(voting_status_filter) = filter.voting_status {
//          format!("AND e.status::jsonb ->> 'voting_status' ILIKE '{voting_status_filter}'")
//      } else {
//          "".to_string()
//      };

//      let is_generate_re_filter_clause = if let Some(is_generate_re_filter) = filter.is_generate_re {
//         format!("AND ('{is_generate_re}'::boolean IS NULL OR ('{is_generate_re}'::boolean = TRUE AND re.id IS NOT NULL) OR ('{is_generate_re}'::boolean = FALSE AND re.id IS NULL))")
//      } else {
//          "".to_string()
//      };

//      let statement: tokio_postgres::Statement = hasura_transaction
//          .prepare(
//              r#"
//              SELECT DISTINCT ON (a.id)
//                  a.id as area_id,
//                  a.name as area_name,
//                  a.description as area_description,
//                  a.description as area_description,
//                 row_to_json(e.*) AS election,
//                  re.id AS results_election_id
//              FROM sequent_backend.area a
//              JOIN sequent_backend.area_contest ac
//                  ON a.id = ac.area_id
//                  AND a.election_event_id = ac.election_event_id
//                  AND a.tenant_id = ac.tenant_id
//              JOIN sequent_backend.contest c
//                  ON ac.contest_id = c.id
//                  AND ac.election_event_id = c.election_event_id
//                  AND ac.tenant_id = c.tenant_id
//              JOIN sequent_backend.election e
//                  ON c.election_id = e.id
//              LEFT JOIN sequent_backend.results_election re
//                  ON e.id = re.election_id
//              WHERE c.tenant_id = $1
//                AND c.election_event_id = $2;
//                {area_name_filter_clause}
//                {precinct_code_filter_clause}
//                {voting_status_filter_clause}
//                {is_generate_re_filter_clause}
//                LIMIT $3 OFFSET $4;
//              "#,
//          )
//          .await?;

//          let mut params: Vec<&(dyn ToSql + Sync)> =
//          vec![&Uuid::parse_str(tenant_id)?, &Uuid::parse_str(election_event_id)?,&query_limit, &query_offset];

//      let rows: Vec<Row> = hasura_transaction
//          .query(
//              &statement,
//              &params,
//          )
//          .await
//          .map_err(|err| anyhow!("Error running get_areas_by_elections query: {err}"))?;

//      let areas: Vec<ElectionMonitorStatus> = rows
//          .into_iter()
//          .map(|row| -> Result<ElectionMonitorStatus> { row.try_into().map(|res: ElectionMonitorStatusWrapper| res.0) })
//          .collect::<Result<Vec<ElectionMonitorStatus>>>()?;

//      Ok(areas)
//  }
