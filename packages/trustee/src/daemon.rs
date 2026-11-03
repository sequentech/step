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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tests::{graphql_body, platform, TOKEN};
    use protocol_board::trustee_sessions::{B4Boards, SessionStores};
    use protocol_board::CustodyRequest;
    use serde_json::json;

    const ACKNOWLEDGED: &str =
        r#"{"data":{"submit_trustee_action_result":{"accepted":true}}}"#;

    fn request(secrets: &TrusteeSecrets) -> CustodyRequest {
        let keys = secrets.public_keys().unwrap();
        CustodyRequest {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            keys_ceremony_id: "ceremony".into(),
            tally_session_id: Some("tally".into()),
            trustee_name: "trustee1".into(),
            signing_public_key: keys.signing_public_key,
            share_encryption_public_key: keys.share_encryption_public_key,
        }
    }

    fn updates(request: &CustodyRequest) -> String {
        json!({"data": {"trustee_event_updates": {
        "boards": [],
        "actions": [
            {"id": "future", "kind": "FUTURE_ACTION"},
            {"id": "custody", "kind": "CREATE_CUSTODY_PROOF", "request": request}
        ]
    }}}).to_string()
    }

    fn sessions(
        secrets: &TrusteeSecrets,
        directory: &tempfile::TempDir,
    ) -> SessionSet<B4Boards> {
        SessionSet::new(
            "trustee1".into(),
            secrets.clone(),
            B4Boards::new(
                "http://127.0.0.1:1".into(),
                SessionStores::new(directory.path().to_path_buf()).unwrap(),
            ),
        )
    }

    #[tokio::test]
    async fn polling_signs_the_scoped_request_and_skips_unknown_actions() {
        let secrets = TrusteeSecrets::generate();
        let request = request(&secrets);
        let response = updates(&request);
        let (mut client, server) =
            platform(vec![(200, TOKEN), (200, &response), (200, ACKNOWLEDGED)]);
        let directory = tempfile::tempdir().unwrap();
        let mut sessions = sessions(&secrets, &directory);
        poll_once(&mut client, &mut sessions, "trustee1", &secrets).await;
        let calls = server.join().unwrap();
        assert_eq!(calls.len(), 3, "Unknown actions must not submit a result");
        let result = &graphql_body(&calls[2])["variables"]["result"];
        assert_eq!(result["id"], "custody");
        assert_eq!(result["request"], serde_json::to_value(&request).unwrap());
        assert_eq!(result["status"], "READY");
        assert!(request
            .verifies(result["custody_proof_json"].as_str().unwrap())
            .unwrap());
        assert!(sessions.pending_reports().is_empty());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn polling_reports_wrong_trustees_and_mismatched_local_keys_without_a_proof(
    ) {
        let secrets = TrusteeSecrets::generate();
        let valid = request(&secrets);
        let other_keys = TrusteeSecrets::generate().public_keys().unwrap();
        for (field, value, detail) in [
            (
                "trustee_name",
                "another-trustee",
                "The request names another trustee",
            ),
            (
                "signing_public_key",
                other_keys.signing_public_key.as_str(),
                "local trustee keys do not match the custody request",
            ),
            (
                "share_encryption_public_key",
                other_keys.share_encryption_public_key.as_str(),
                "local trustee keys do not match the custody request",
            ),
        ] {
            let mut changed = serde_json::to_value(&valid).unwrap();
            changed[field] = json!(value);
            let request: CustodyRequest =
                serde_json::from_value(changed.clone()).unwrap();
            let response = updates(&request);
            let (mut client, server) = platform(vec![
                (200, TOKEN),
                (200, &response),
                (200, ACKNOWLEDGED),
            ]);
            let directory = tempfile::tempdir().unwrap();
            let mut sessions = sessions(&secrets, &directory);
            poll_once(&mut client, &mut sessions, "trustee1", &secrets).await;
            let calls = server.join().unwrap();
            assert_eq!(
                graphql_body(&calls[2])["variables"]["result"],
                json!({
                    "id": "custody", "request": changed, "status": "FAILED", "detail": detail
                })
            );
        }
    }

    #[tokio::test]
    async fn polling_resubmits_an_unacknowledged_action_on_the_next_cycle() {
        let secrets = TrusteeSecrets::generate();
        let request = request(&secrets);
        let response = updates(&request);
        let (mut client, server) = platform(vec![
            (200, TOKEN),
            (200, &response),
            (503, "temporarily unavailable"),
            (200, &response),
            (200, ACKNOWLEDGED),
        ]);
        let directory = tempfile::tempdir().unwrap();
        let mut sessions = sessions(&secrets, &directory);
        for _ in 0..2 {
            poll_once(&mut client, &mut sessions, "trustee1", &secrets).await;
        }
        let calls = server.join().unwrap();
        for index in [2, 4] {
            let result = &graphql_body(&calls[index])["variables"]["result"];
            assert_eq!(result["id"], "custody");
            assert_eq!(result["status"], "READY");
            assert!(request
                .verifies(result["custody_proof_json"].as_str().unwrap())
                .unwrap());
        }
    }

    #[tokio::test]
    async fn a_failed_update_does_not_submit_action_results() {
        let secrets = TrusteeSecrets::generate();
        let (mut client, server) =
            platform(vec![(200, TOKEN), (503, "unavailable")]);
        let directory = tempfile::tempdir().unwrap();
        let mut sessions = sessions(&secrets, &directory);
        poll_once(&mut client, &mut sessions, "trustee1", &secrets).await;
        assert_eq!(server.join().unwrap().len(), 2);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
