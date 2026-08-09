// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use futures::pin_mut;
use sequent_core::ballot::ElectionPresentation;
use sequent_core::ballot::ElectionStatus;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::{Election, VotingChannels};
use serde_json::Value;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::row::Row;
use tokio_postgres::types::{ToSql, Type};
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// Election column for insert operation
const ELECTION_COPY_COLUMNS: &str = "id, tenant_id, election_event_id, created_at, last_updated_at,
labels, annotations, description, presentation, status, eml, num_allowed_revotes, is_consolidated_ballot_encoding,
spoil_ballot_option, voting_channels, is_kiosk, image_document_id, statistics, receipts, permission_label,
keys_ceremony_id, initialization_report_generated, external_id";

/// Election columns types for insert operation (same order as the columns)
const ELECTION_COPY_TYPES: &[Type] = &[
    Type::UUID,
    Type::UUID,
    Type::UUID,
    Type::TIMESTAMPTZ,
    Type::TIMESTAMPTZ,
    Type::JSONB,
    Type::JSONB,
    Type::TEXT,
    Type::JSONB,
    Type::JSONB,
    Type::TEXT,
    Type::INT4,
    Type::BOOL,
    Type::BOOL,
    Type::JSONB,
    Type::BOOL,
    Type::TEXT,
    Type::JSONB,
    Type::JSONB,
    Type::TEXT,
    Type::UUID,
    Type::BOOL,
    Type::TEXT,
];

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
            description: item.try_get("description")?,
            presentation: item.try_get("presentation")?,
            status: item.try_get("status")?,
            eml: item.try_get("eml")?,
            external_id: item.try_get("external_id")?,
            num_allowed_revotes: num_allowed_revotes.map(|val| val as i64),
            is_consolidated_ballot_encoding: item.try_get("is_consolidated_ballot_encoding")?,
            spoil_ballot_option: item.try_get("spoil_ballot_option")?,
            is_kiosk: item.try_get("is_kiosk")?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
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
) -> Result<Vec<Election>> {
    let statement_str = format!(
        r#"
            SELECT
                *
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
        .map(|id| parse_uuid_v4(&id).map_err(|err| anyhow!("{:?}", err)))
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(keys_ceremony_id)?,
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
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let election_uuid: uuid::Uuid =
        parse_uuid_v4(election_id).with_context(|| "Error parsing election_id as UUID")?;

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
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let election_uuid: uuid::Uuid =
        parse_uuid_v4(election_id).with_context(|| "Error parsing election_id as UUID")?;

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
    presentation: &ElectionPresentation,
    description: Option<String>,
    external_id: &str,
) -> Result<Election> {
    let presentation_value = serde_json::to_value(presentation)
        .map_err(|err| anyhow!("Error serializing election presentation: {err}"))?;
    let voting_channels_value = serde_json::to_value(&VotingChannels::default())
        .map_err(|err| anyhow!("Error serializing voting_channels: {err}"))?;
    let status = serde_json::to_value(ElectionStatus::default())
        .map_err(|err| anyhow!("Error serializing election status: {err}"))?;
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.election
                (
                    tenant_id,
                    election_event_id,
                    created_at,
                    last_updated_at,
                    description,
                    presentation,
                    voting_channels,
                    status,
                    external_id
                )
                VALUES
                (
                    $1,
                    $2,
                    NOW(),
                    NOW(),
                    $3,
                    $4,
                    $5,
                    $6,
                    $7
                )
                RETURNING *;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(&tenant_id)?,
                &parse_uuid_v4(&election_event_id)?,
                &description,
                &presentation_value,
                &voting_channels_value,
                &status,
                &external_id,
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
    if data.elections.is_empty() {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let copy_sql =
        format!("COPY sequent_backend.election ({ELECTION_COPY_COLUMNS}) FROM STDIN BINARY");

    let sink = hasura_transaction
        .copy_in(&copy_sql)
        .await
        .with_context(|| format!("Error preparing election COPY IN: {copy_sql}"))?;
    let writer = BinaryCopyInWriter::new(sink, ELECTION_COPY_TYPES);
    pin_mut!(writer);

    for election in &data.elections {
        election.validate()?;

        let id = parse_uuid_v4(&election.id)?;
        let tenant_id = parse_uuid_v4(&election.tenant_id)?;
        let election_event_id = parse_uuid_v4(&election.election_event_id)?;
        let keys_ceremony_id = election
            .keys_ceremony_id
            .as_ref()
            .map(|val| parse_uuid_v4(val))
            .transpose()?;
        let num_allowed_revotes = election.num_allowed_revotes.map(|val| val as i32);

        let row: [&(dyn ToSql + Sync); 23] = [
            &id,
            &tenant_id,
            &election_event_id,
            &now,
            &now,
            &election.labels,
            &election.annotations,
            &election.description,
            &election.presentation,
            &election.status,
            &election.eml,
            &num_allowed_revotes,
            &election.is_consolidated_ballot_encoding,
            &election.spoil_ballot_option,
            &election.voting_channels,
            &election.is_kiosk,
            &election.image_document_id,
            &election.statistics,
            &election.receipts,
            &election.permission_label,
            &keys_ceremony_id,
            &election.initialization_report_generated,
            &election.external_id,
        ];

        writer
            .as_mut()
            .write(&row)
            .await
            .map_err(|err| anyhow!("Error writing election COPY row: {err}"))?;
    }

    writer
        .finish()
        .await
        .context("Error finishing election COPY IN transaction")?;

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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
        .map(|val| parse_uuid_v4(&val))
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
                &parse_uuid_v4(keys_ceremony_id)?,
                &election_uuid_opt,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
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
    let parsed_id = parse_uuid_v4(id)?;
    let parsed_tenant_id = parse_uuid_v4(tenant_id)?;
    let parsed_election_event_id = parse_uuid_v4(election_event_id)?;

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

#[instrument(skip(hasura_transaction), err)]
pub async fn get_elections_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<String>> {
    let statement_str = format!(
        r#"
            SELECT
                id
            FROM
                sequent_backend.election
            WHERE
                tenant_id = $1 AND
                election_event_id = $2
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;

    let elections: Vec<String> = rows
        .into_iter()
        .map(|row| -> Result<String> {
            let id: Uuid = row.try_get("id")?;
            Ok(id.to_string())
        })
        .collect::<Result<Vec<String>>>()?;

    Ok(elections)
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_election_permission_label(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
) -> Result<Vec<String>> {
    let election_uuid_opt = election_id
        .clone()
        .map(|val| parse_uuid_v4(&val))
        .transpose()?;
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                permission_label
                FROM
                    sequent_backend.election
                WHERE
                    ($1::uuid IS NULL OR id = $1::uuid) AND
                    tenant_id = $2 AND
                    election_event_id = $3
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &election_uuid_opt,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the set_election_keys_ceremony query: {err}"))?;

    if 0 == rows.len() {
        return Err(anyhow!("No election found"));
    }

    let perms: Vec<Option<String>> = rows
        .into_iter()
        .map(|row: Row| -> Result<Option<String>> {
            let permission_label: Option<String> = row.try_get(0)?;
            Ok(permission_label)
        })
        .collect::<Result<Vec<Option<String>>>>()?;

    Ok(perms.into_iter().flatten().collect())
}
