// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The bulletin boards the platform has created on the b4 board service,
//! with the parent/child relation between a tally board and its DKG board.
//! This is what tells trustees which boards to join.

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::ceremonies::{ProtocolBoardKind, ProtocolBoardLifecycle};
use sequent_core::types::hasura::core::ProtocolBoard;
use std::str::FromStr;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct ProtocolBoardWrapper(ProtocolBoard);

impl TryFrom<Row> for ProtocolBoardWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let kind = item.try_get::<_, String>("kind")?;
        let lifecycle = item.try_get::<_, String>("lifecycle")?;
        Ok(ProtocolBoardWrapper(ProtocolBoard {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            name: item.try_get("name")?,
            kind: ProtocolBoardKind::from_str(&kind)
                .map_err(|err| anyhow!("invalid protocol board kind '{kind}': {err:?}"))?,
            lifecycle: ProtocolBoardLifecycle::from_str(&lifecycle).map_err(|err| {
                anyhow!("invalid protocol board lifecycle '{lifecycle}': {err:?}")
            })?,
            parent_id: item
                .try_get::<_, Option<Uuid>>("parent_id")?
                .map(|uuid| uuid.to_string()),
            keys_ceremony_id: item
                .try_get::<_, Option<Uuid>>("keys_ceremony_id")?
                .map(|uuid| uuid.to_string()),
            configuration_hash: item.try_get("configuration_hash")?,
            trustee_ids: item
                .try_get::<_, Vec<Uuid>>("trustee_ids")?
                .iter()
                .map(|uuid| uuid.to_string())
                .collect(),
            trustee_reports: item.try_get("trustee_reports")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            annotations: item.try_get("annotations")?,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct NewProtocolBoard {
    pub tenant_id: String,
    pub election_event_id: String,
    pub name: String,
    pub kind: ProtocolBoardKind,
    pub parent_id: Option<String>,
    pub keys_ceremony_id: Option<String>,
    pub configuration_hash: String,
    pub trustee_ids: Vec<String>,
}

#[instrument(err, skip(hasura_transaction))]
pub async fn insert_protocol_board(
    hasura_transaction: &Transaction<'_>,
    board: &NewProtocolBoard,
) -> Result<ProtocolBoard> {
    let parent_id = board
        .parent_id
        .as_deref()
        .map(parse_uuid_v4)
        .transpose()
        .with_context(|| "Error parsing parent_id as UUID")?;
    let keys_ceremony_id = board
        .keys_ceremony_id
        .as_deref()
        .map(parse_uuid_v4)
        .transpose()
        .with_context(|| "Error parsing keys_ceremony_id as UUID")?;
    let trustee_ids = board
        .trustee_ids
        .iter()
        .map(|id| parse_uuid_v4(id).map_err(|err| anyhow!("{err:?}")))
        .collect::<Result<Vec<Uuid>>>()
        .with_context(|| "Error parsing trustee_ids as UUIDs")?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.protocol_board
                (tenant_id, election_event_id, name, kind, lifecycle, parent_id, keys_ceremony_id, configuration_hash, trustee_ids)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING
                    *;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(&board.tenant_id)?,
                &parse_uuid_v4(&board.election_event_id)?,
                &board.name,
                &board.kind.to_string(),
                &ProtocolBoardLifecycle::ACTIVE.to_string(),
                &parent_id,
                &keys_ceremony_id,
                &board.configuration_hash,
                &trustee_ids,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting protocol board: {err}"))?;

    rows.into_iter()
        .next()
        .ok_or_else(|| anyhow!("Row not inserted"))
        .and_then(|row| row.try_into().map(|res: ProtocolBoardWrapper| res.0))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_dkg_board_by_keys_ceremony(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<Option<ProtocolBoard>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.protocol_board
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    keys_ceremony_id = $3 AND
                    kind = $4
                ORDER BY created_at ASC
                LIMIT 1;
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
                &ProtocolBoardKind::DKG.to_string(),
            ],
        )
        .await?;

    rows.into_iter()
        .next()
        .map(|row| row.try_into().map(|res: ProtocolBoardWrapper| res.0))
        .transpose()
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_protocol_board_by_name(
    hasura_transaction: &Transaction<'_>,
    name: &str,
) -> Result<Option<ProtocolBoard>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.protocol_board
                WHERE
                    name = $1;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction.query(&statement, &[&name]).await?;

    rows.into_iter()
        .next()
        .map(|row| row.try_into().map(|res: ProtocolBoardWrapper| res.0))
        .transpose()
}
