// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The bulletin boards the platform has created on the b4 board service,
//! with the parent/child relation between a tally board and its DKG board.
//! This is what tells trustees which boards to join.

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use protocol_board::{BoardName, TrusteeBoard};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
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
                    parent_id IS NULL AND
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

/// The boards trustee `trustee_id` has work on, oldest first: the DKG boards
/// of the keys ceremonies it is part of that are in progress.
#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustee_boards(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_id: &str,
) -> Result<Vec<TrusteeBoard>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    board.*,
                    parent.name AS parent_name
                FROM
                    sequent_backend.protocol_board AS board
                JOIN
                    sequent_backend.keys_ceremony AS keys_ceremony
                ON
                    keys_ceremony.id = board.keys_ceremony_id AND
                    keys_ceremony.tenant_id = board.tenant_id AND
                    keys_ceremony.election_event_id = board.election_event_id
                LEFT JOIN
                    sequent_backend.protocol_board AS parent
                ON
                    parent.id = board.parent_id
                WHERE
                    board.tenant_id = $1 AND
                    board.parent_id IS NULL AND
                    keys_ceremony.execution_status = $2 AND
                    $3 = ANY(keys_ceremony.trustee_ids)
                ORDER BY
                    board.created_at,
                    board.name;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &KeysCeremonyExecutionStatus::IN_PROGRESS.to_string(),
                &parse_uuid_v4(trustee_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error listing the boards of trustee {trustee_id}: {err}"))?;

    rows.into_iter()
        .map(|row| -> Result<TrusteeBoard> {
            let parent_name: Option<String> = row.try_get("parent_name")?;
            let board = row.try_into().map(|res: ProtocolBoardWrapper| res.0)?;
            TrusteeBoard::of(&board, parent_name.as_deref())
        })
        .collect()
}

/// Board `board`, if it belongs to a keys ceremony trustee `trustee_id` is
/// part of, whatever the ceremony's state.
#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustee_board(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_id: &str,
    board: &BoardName,
) -> Result<Option<ProtocolBoard>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    board.*
                FROM
                    sequent_backend.protocol_board AS board
                JOIN
                    sequent_backend.keys_ceremony AS keys_ceremony
                ON
                    keys_ceremony.id = board.keys_ceremony_id AND
                    keys_ceremony.tenant_id = board.tenant_id AND
                    keys_ceremony.election_event_id = board.election_event_id
                WHERE
                    board.tenant_id = $1 AND
                    board.name = $2 AND
                    $3 = ANY(keys_ceremony.trustee_ids);
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &board.as_str(),
                &parse_uuid_v4(trustee_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error finding board {board} of trustee {trustee_id}: {err}"))?;

    rows.into_iter()
        .next()
        .map(|row| row.try_into().map(|res: ProtocolBoardWrapper| res.0))
        .transpose()
}
