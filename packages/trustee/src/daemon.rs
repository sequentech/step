// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The poll loop: every second, report the halts the platform has not
//! recorded, ask for this trustee's boards, follow the list and advance the
//! sessions.

use std::time::Duration;

use anyhow::{Context as _, Result};
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::sleep;
use tracing::{error, info};

use crate::platform::PlatformClient;
use protocol_board::trustee_sessions::{BoardAccess, SessionSet};
use protocol_board::{
    TrusteeActionKind, TrusteeActionOutcome, TrusteeActionResult,
    TrusteeSecrets,
};

/// The pause between two cycles.
const CYCLE_PAUSE: Duration = Duration::from_secs(1);

/// Run cycles until SIGTERM or SIGINT, which end the loop between two cycles.
pub(crate) async fn run<A: BoardAccess>(
    mut platform: PlatformClient,
    mut sessions: SessionSet<A>,
    trustee_name: String,
    secrets: TrusteeSecrets,
) -> Result<()> {
    let mut terminate =
        signal(SignalKind::terminate()).context("listening for SIGTERM")?;
    let mut interrupt =
        signal(SignalKind::interrupt()).context("listening for SIGINT")?;
    loop {
        poll_once(&mut platform, &mut sessions, &trustee_name, &secrets).await;
        tokio::select! {
            () = sleep(CYCLE_PAUSE) => {}
            _ = terminate.recv() => {
                info!("SIGTERM received: stopping");
                return Ok(());
            }
            _ = interrupt.recv() => {
                info!("SIGINT received: stopping");
                return Ok(());
            }
        }
    }
}

/// One cycle. A failed report is tried again next cycle; a failed list ends
/// the cycle with no session opened, closed or advanced.
async fn poll_once<A: BoardAccess>(
    platform: &mut PlatformClient,
    sessions: &mut SessionSet<A>,
    trustee_name: &str,
    secrets: &TrusteeSecrets,
) {
    for report in sessions.pending_reports() {
        match platform.report(&report).await {
            Ok(()) => {
                info!(board = %report.board, "the platform recorded the halt");
                sessions.settle(&report.board);
            }
            Err(err) => {
                error!(board = %report.board, "reporting the halt: {err:#}");
            }
        }
    }
    match platform.updates().await {
        Ok(updates) => {
            for action in updates.actions {
                match action.action {
                    TrusteeActionKind::CreateCustodyProof { request } => {
                        let outcome = if request.trustee_name != trustee_name {
                            TrusteeActionOutcome::Failed {
                                detail: "The request names another trustee"
                                    .into(),
                            }
                        } else {
                            match request.attest(secrets) {
                                Ok(custody_proof_json) => {
                                    TrusteeActionOutcome::Ready {
                                        custody_proof_json,
                                    }
                                }
                                Err(error) => TrusteeActionOutcome::Failed {
                                    detail: format!("{error:#}"),
                                },
                            }
                        };
                        let result = TrusteeActionResult {
                            id: action.id,
                            request,
                            outcome,
                        };
                        if let Err(error) =
                            platform.action_result(&result).await
                        {
                            error!(action = %result.id, "Submitting trustee action result: {error:#}");
                        }
                    }
                    TrusteeActionKind::Unknown => {
                        tracing::debug!(action = %action.id, "Skipping unsupported trustee action")
                    }
                }
            }
            sessions.reconcile(&updates.boards).await;
            sessions.advance().await;
        }
        Err(err) => error!("fetching this trustee's event updates: {err:#}"),
    }
}
