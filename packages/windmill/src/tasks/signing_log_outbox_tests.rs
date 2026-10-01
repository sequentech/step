// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The kick never fails its caller.

use super::*;

#[tokio::test]
async fn a_kick_that_fails_panics_or_hangs_only_logs() {
    assert!(spawn_kick(async { Ok(()) }, KICK_TIMEOUT).await.is_ok());
    assert!(
        spawn_kick(async { Err(anyhow!("broker down")) }, KICK_TIMEOUT)
            .await
            .is_ok()
    );
    assert!(spawn_kick(async { panic!("no celery app") }, KICK_TIMEOUT)
        .await
        .is_ok());
    assert!(
        spawn_kick(std::future::pending(), Duration::from_millis(10))
            .await
            .is_ok()
    );
}
