// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The bulletin boards the platform has created on the b4 board service,
//! with the parent/child relation between a tally board and its DKG board.
//! This is what tells trustees which boards to join.

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::ProtocolBoard;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct ProtocolBoardWrapper(ProtocolBoard);

impl TryFrom<Row> for ProtocolBoardWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(ProtocolBoardWrapper(ProtocolBoard {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            parent_id: item
                .try_get::<_, Option<Uuid>>("parent_id")?
                .map(|i| i.to_string()),
            keys_ceremony_id: item.try_get::<_, Uuid>("keys_ceremony_id")?.to_string(),
            name: item.try_get("name")?,
            manager_message: item.try_get("manager_message")?,
            created_at: item.try_get("created_at")?,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct NewProtocolBoard {
    pub tenant_id: String,
    pub election_event_id: String,
    pub parent_id: Option<String>,
    pub keys_ceremony_id: String,
    pub name: String,
    pub manager_message: Vec<u8>,
}

#[instrument(err, skip(hasura_transaction))]
pub async fn insert_protocol_board(
    hasura_transaction: &Transaction<'_>,
    board: &NewProtocolBoard,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.protocol_board
                (tenant_id, election_event_id, parent_id, keys_ceremony_id, name, manager_message)
                VALUES($1, $2, $3, $4, $5, $6);
            "#,
        )
        .await?;
    hasura_transaction
        .execute(
            &statement,
            &[
                &parse_uuid_v4(&board.tenant_id)?,
                &parse_uuid_v4(&board.election_event_id)?,
                &board.parent_id.as_deref().map(parse_uuid_v4).transpose()?,
                &parse_uuid_v4(&board.keys_ceremony_id)?,
                &board.name,
                &board.manager_message,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting protocol board: {err}"))?;
    Ok(())
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
                    parent_id = NULL AND
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

    rows.into_iter()
        .next()
        .map(|row| row.try_into().map(|res: ProtocolBoardWrapper| res.0))
        .transpose()
}
