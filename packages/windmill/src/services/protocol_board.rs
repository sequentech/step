// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The crypto core's boards, as windmill handles it: loading the messages
//! their rows store, and writing back what the `protocol_board` crate has
//! decided.

use anyhow::{anyhow, Context as _, Result};
use deadpool_postgres::Transaction;
use protocol_board::{
    encode_manager_key, parse_manager_key, tally_transition, transition, BoardHandle, BoardManager,
    BoardName, CeremonyState, DkgBoard, PlatformEvent, SignedBallots, SignedConfiguration,
    TallyBoard, TallyEvent, TallySessionState, Timestamps,
};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::{get_now_utc_unix_ms, ISO8601};
use sequent_core::types::ceremonies::{TallyExecutionStatus, TallySessionDocuments};
use sequent_core::types::hasura::core::ProtocolBoard;
use tracing::instrument;
use uuid::Uuid;

use crate::postgres::keys_ceremony::{
    get_keys_ceremony_by_id_for_update, update_keys_ceremony_status,
};
use crate::postgres::protocol_board::{
    get_dkg_board_by_keys_ceremony, get_tally_boards_by_session, insert_protocol_board,
    NewProtocolBoard,
};
use crate::postgres::tally_session::{
    get_tally_session_by_id_for_update, update_tally_session_status,
};
use crate::postgres::tally_session_execution::{
    get_last_tally_session_execution, insert_tally_session_execution,
};
use crate::services::ceremonies::tally_ceremony::get_tally_ceremony_status;
use crate::services::vault;

pub fn board_handle(board: &BoardName) -> Result<BoardHandle> {
    let base_url = std::env::var("WBRAID_B4_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
        // An unset address would otherwise look like an outage.
        .ok_or_else(|| anyhow!("WBRAID_B4_URL must be set to the board service URL"))?;
    Ok(BoardHandle::new(base_url.trim(), board.clone()))
}

/// The current time, in the two forms a ceremony records it.
pub fn ceremony_now() -> Timestamps {
    Timestamps {
        iso8601: ISO8601::to_string(&ISO8601::now()),
        unix_ms: get_now_utc_unix_ms().to_string(),
    }
}

#[instrument(err, skip(hasura_transaction))]
pub async fn load_dkg_board(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<DkgBoard> {
    let (_, dkg) = load_dkg_board_row(
        hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    )
    .await?;
    Ok(dkg)
}

/// The keys ceremony's DKG board together with the row that records it, which
/// the ceremony's tally boards name as their parent.
#[instrument(err, skip(hasura_transaction))]
pub async fn load_dkg_board_row(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<(ProtocolBoard, DkgBoard)> {
    let row = get_dkg_board_by_keys_ceremony(
        hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    )
    .await?
    .ok_or_else(|| anyhow!("keys ceremony {keys_ceremony_id} has no DKG board recorded"))?;

    let keys_ceremony_id = Uuid::parse_str(&row.keys_ceremony_id)
        .with_context(|| format!("board {} names an unreadable ceremony id", row.name))?;
    let configuration = SignedConfiguration::parse(&row.manager_message)
        .with_context(|| format!("board {} stores an unreadable configuration", row.name))?;

    let dkg = DkgBoard::from_signed_configuration(&keys_ceremony_id, configuration);
    Ok((row, dkg))
}

/// Apply one event to a ceremony and commit what it decided.
///
/// The ceremony is loaded again under a row lock: windmill's beat and a
/// trustee's report both write it, and which event prevails is a rule of the
/// transition, not of write order, so every transition must run on the latest
/// committed state.
///
/// A no-op transition commits nothing, this lets a task run on every
/// beat without touching the database when the board has not
/// moved.
#[instrument(err, skip(hasura_transaction, event))]
pub async fn apply_event(
    hasura_transaction: Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
    event: PlatformEvent,
) -> Result<()> {
    let keys_ceremony = get_keys_ceremony_by_id_for_update(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    )
    .await?;
    let unreadable = || format!("keys ceremony {keys_ceremony_id} has an unreadable status");
    let state = CeremonyState::new(
        keys_ceremony.execution_status().with_context(unreadable)?,
        keys_ceremony.status().with_context(unreadable)?,
    );

    let next = transition(&state, keys_ceremony.policy(), event, &ceremony_now());
    if next == state {
        return Ok(());
    }

    update_keys_ceremony_status(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
        &serde_json::to_value(&next.status)?,
        &next.execution.to_string(),
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")?;
    Ok(())
}

pub fn manager_key_vault_path(board: &BoardName) -> String {
    format!("protocol-boards/{board}/manager")
}

/// Keep the board's protocol manager key in the vault.
///
/// The caller should commit `hasura_transaction` together with the board row
/// that stores the Configuration this key signed.
#[instrument(err, skip(hasura_transaction, manager))]
pub async fn save_manager_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &BoardName,
    manager: &BoardManager,
) -> Result<()> {
    tracing::info!("storing the protocol manager key of board {board}");
    vault::save_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &manager_key_vault_path(board),
        &encode_manager_key(manager)?,
    )
    .await
}

/// The protocol manager key of `board`, which signs every message the
/// platform posts on the board and on its tally boards.
#[instrument(err, skip(hasura_transaction))]
pub async fn load_manager(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &BoardName,
) -> Result<BoardManager> {
    let encoded = vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &manager_key_vault_path(board),
    )
    .await?
    .ok_or_else(|| anyhow!("the vault holds no protocol manager key for board {board}"))?;
    parse_manager_key(&encoded)
        .with_context(|| format!("the protocol manager key of board {board}"))
}

/// The tally boards of a session, by batch, each with the `Ballots` message
/// its row stores.
#[instrument(err, skip(hasura_transaction, configuration))]
pub async fn load_tally_boards(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    configuration: &SignedConfiguration,
) -> Result<Vec<(ProtocolBoard, SignedBallots)>> {
    get_tally_boards_by_session(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?
    .into_iter()
    .map(|row| {
        let ballots = SignedBallots::parse(&row.manager_message, configuration)
            .with_context(|| format!("board {} stores an unreadable Ballots message", row.name))?;
        Ok((row, ballots))
    })
    .collect()
}

/// Record a tally board of `parent`, the DKG board of the session's keys
/// ceremony, in row `row_id`, the id its `Ballots` message was signed with.
///
/// The caller should commit `hasura_transaction` before the board is
/// published: what is published is what the row stores.
#[instrument(err, skip(hasura_transaction, parent, board), fields(board = %board.name))]
pub async fn insert_tally_board(
    hasura_transaction: &Transaction<'_>,
    parent: &ProtocolBoard,
    row_id: &Uuid,
    tally_session_id: &str,
    batch: i64,
    board: &TallyBoard,
) -> Result<()> {
    insert_protocol_board(
        hasura_transaction,
        &NewProtocolBoard {
            id: Some(row_id.to_string()),
            tenant_id: parent.tenant_id.clone(),
            election_event_id: parent.election_event_id.clone(),
            parent_id: Some(parent.id.clone()),
            keys_ceremony_id: parent.keys_ceremony_id.clone(),
            tally_session_id: Some(tally_session_id.to_string()),
            batch: Some(batch),
            name: board.name.as_str().to_string(),
            // TODO(tally follow-up): the stored Ballots grow with the electorate
            // and likely should be nulled once the board serves the message.
            manager_message: board.input.to_bytes(),
        },
    )
    .await
    .with_context(|| format!("couldn't record tally board {}", board.name))
}

/// Apply one event to a tally session, inside `hasura_transaction`.
///
/// The session is loaded again under a row lock, and its status from the
/// newest execution row read after it: windmill's task and a trustee's report
/// both write the session, and which event prevails is a rule of the
/// transition, not of write order. The lock is held until the caller commits,
/// and the caller acts on the state returned, never on one it loaded before.
///
/// A change is recorded as a new execution row that carries every other column
/// of the newest one forward; a no-op transition writes nothing.
#[instrument(err, skip(hasura_transaction, event))]
pub async fn apply_tally_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    event: TallyEvent,
) -> Result<TallySessionState> {
    let tally_session = get_tally_session_by_id_for_update(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    let head = get_last_tally_session_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?
    .ok_or_else(|| anyhow!("tally session {tally_session_id} has no execution"))?;
    let unreadable = || format!("tally session {tally_session_id} has an unreadable status");
    let execution = tally_session
        .execution_status
        .as_deref()
        .ok_or_else(|| anyhow!("tally session {tally_session_id} has no execution status"))?
        .parse::<TallyExecutionStatus>()
        .with_context(unreadable)?;
    let state = TallySessionState::new(
        execution,
        get_tally_ceremony_status(head.status.clone()).with_context(unreadable)?,
    );

    let next = tally_transition(&state, event, &ceremony_now());
    if next == state {
        return Ok(state);
    }

    let documents = head
        .documents
        .clone()
        .map(deserialize_value::<TallySessionDocuments>)
        .transpose()
        .with_context(|| format!("tally session {tally_session_id} has unreadable documents"))?;
    insert_tally_session_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        head.current_message_id,
        tally_session_id,
        Some(next.status.clone()),
        head.results_event_id.clone(),
        head.session_ids.clone(),
        documents,
        head.run_reason(),
    )
    .await?;

    if next.execution != state.execution {
        let is_execution_completed = match next.execution {
            TallyExecutionStatus::SUCCESS | TallyExecutionStatus::FAILED => true,
            TallyExecutionStatus::STARTED
            | TallyExecutionStatus::CONNECTED
            | TallyExecutionStatus::IN_PROGRESS
            | TallyExecutionStatus::AWAITING_INPUT
            | TallyExecutionStatus::CANCELLED => false,
        };
        update_tally_session_status(
            hasura_transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
            next.execution.clone(),
            is_execution_completed,
        )
        .await?;
    }

    Ok(next)
}
