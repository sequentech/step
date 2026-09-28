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
use crate::sessions::{BoardAccess, SessionSet};

/// The pause between two cycles.
const CYCLE_PAUSE: Duration = Duration::from_secs(1);

/// Run cycles until SIGTERM or SIGINT, which end the loop between two cycles.
pub(crate) async fn run<A: BoardAccess>(
    mut platform: PlatformClient,
    mut sessions: SessionSet<A>,
) -> Result<()> {
    let mut terminate =
        signal(SignalKind::terminate()).context("listening for SIGTERM")?;
    let mut interrupt =
        signal(SignalKind::interrupt()).context("listening for SIGINT")?;
    loop {
        poll_once(&mut platform, &mut sessions).await;
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
) {
    for report in sessions.pending_reports() {
        match platform.report(&report).await {
            Ok(answer) => {
                info!(
                    board = %report.board,
                    ceremony = %answer.ceremony,
                    "the platform recorded the halt"
                );
                sessions.settle(&report.board);
            }
            Err(err) => {
                error!(board = %report.board, "reporting the halt: {err:#}");
            }
        }
    }
    match platform.list().await {
        Ok(boards) => {
            sessions.reconcile(&boards).await;
            sessions.advance().await;
        }
        Err(err) => error!("listing this trustee's boards: {err:#}"),
    }
}
