// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the platform tells a trustee out of band, and what it hears back: the
//! boards the trustee has work on, and its reports about them. The trustee is
//! always the caller the token's `trustee` claim names.

use anyhow::{anyhow, bail, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::{
    PlatformEvent, TrusteeBoardsResponse, TrusteeReport, TrusteeReportKind, TrusteeReportResponse,
};
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
use sequent_core::types::hasura::core::{ProtocolBoard, Trustee};
use sequent_core::types::protocol_board::ProtocolBoardKind;
use tracing::{info, instrument, warn};

use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::postgres::protocol_board::{get_trustee_board, get_trustee_boards};
use crate::postgres::trustee::get_trustee_by_name;
use crate::services::protocol_board::apply_event;

/// The caller's name and trustee row.
async fn caller<'a>(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_claim: Option<&'a str>,
) -> Result<(&'a str, Trustee)> {
    let name = trustee_claim.ok_or_else(|| anyhow!("the token carries no trustee claim"))?;
    let trustee = get_trustee_by_name(hasura_transaction, tenant_id, name)
        .await
        .with_context(|| format!("error finding trustee {name} in tenant {tenant_id}"))?;
    Ok((name, trustee))
}

/// The boards the caller has work on.
#[instrument(err, skip(hasura_transaction))]
pub async fn list_boards(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_claim: Option<&str>,
) -> Result<TrusteeBoardsResponse> {
    let (_, trustee) = caller(hasura_transaction, tenant_id, trustee_claim).await?;
    let boards = get_trustee_boards(hasura_transaction, tenant_id, &trustee.id).await?;
    Ok(TrusteeBoardsResponse { boards })
}

/// Apply the caller's report about one of its boards, answering the state the
/// board's ceremony is in afterwards.
#[instrument(err, skip(hasura_transaction))]
pub async fn report(
    hasura_transaction: Transaction<'_>,
    tenant_id: &str,
    trustee_claim: Option<&str>,
    report: TrusteeReport,
) -> Result<TrusteeReportResponse> {
    let (name, trustee) = caller(&hasura_transaction, tenant_id, trustee_claim).await?;
    let board_name = &report.board;
    let board = get_trustee_board(&hasura_transaction, tenant_id, &trustee.id, board_name)
        .await?
        .ok_or_else(|| anyhow!("board {board_name} is not one of trustee {name}'s boards"))?;

    let ceremony = match (report.kind, board.kind()) {
        (TrusteeReportKind::HALTED, ProtocolBoardKind::DKG) => {
            warn!(
                "trustee {name} halted on board {board_name}: {}",
                report.detail
            );
            record_halt(hasura_transaction, tenant_id, name, &board, report.detail).await?
        }
        // What a halt does to a tally session is not defined yet, and it must
        // never reach the keys ceremony of the tally's parent board.
        (TrusteeReportKind::HALTED, ProtocolBoardKind::TALLY) => {
            bail!("tally board {board_name}: its halts are not recorded yet")
        }
    };

    info!(
        "keys ceremony {} of board {board_name} is {ceremony} after the report of trustee {name}",
        board.keys_ceremony_id
    );
    Ok(TrusteeReportResponse { ceremony })
}

/// Apply trustee `name`'s halt on DKG board `board` to the board's keys
/// ceremony.
async fn record_halt(
    hasura_transaction: Transaction<'_>,
    tenant_id: &str,
    name: &str,
    board: &ProtocolBoard,
    detail: String,
) -> Result<KeysCeremonyExecutionStatus> {
    let keys_ceremony_id = &board.keys_ceremony_id;
    let keys_ceremony = get_keys_ceremony_by_id(
        &hasura_transaction,
        tenant_id,
        &board.election_event_id,
        keys_ceremony_id,
    )
    .await?;
    let status = keys_ceremony
        .status()
        .with_context(|| format!("keys ceremony {keys_ceremony_id} has an unreadable status"))?;
    if !status.trustees.iter().any(|listed| listed.name == name) {
        bail!(
            "keys ceremony {keys_ceremony_id} of board {} does not list trustee {name}",
            board.name
        );
    }

    apply_event(
        hasura_transaction,
        tenant_id,
        &board.election_event_id,
        keys_ceremony_id,
        PlatformEvent::TrusteeHalted {
            trustee: name.to_string(),
            detail,
        },
    )
    .await
}
