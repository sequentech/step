// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::services::connection::{
    VoterAccessFailure, VoterAccessPolicy,
};
use windmill::services::{
    audit_storage, database::get_hasura_pool, election_lifecycle,
};

pub struct ElectionVoterAdmission;

#[rocket::async_trait]
impl VoterAccessPolicy for ElectionVoterAdmission {
    async fn check(
        &self,
        tenant: &str,
        event: &str,
    ) -> Result<(), VoterAccessFailure> {
        audit_storage::require_voter_storage().await.map_err(|error| {
            tracing::warn!("Voter channel denied by audit-storage protection: {error:#}");
            VoterAccessFailure::AuditStorageUnavailable
        })?;
        let allowed = async {
            let mut db = get_hasura_pool().await.get().await?;
            let tx = db.transaction().await?;
            election_lifecycle::require_voter_access(&tx, tenant, event)
                .await?;
            tx.commit().await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        allowed.map_err(|error| {
            tracing::warn!(
                "Voter channel denied by election lifecycle: {error:#}"
            );
            VoterAccessFailure::ElectionUnavailable
        })
    }
}

/// Startup and every heartbeat use short deadlines. Until startup registration
/// succeeds there is no live lease, so voter operations fail closed.
pub fn lifecycle_fairing() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_liftoff(
        "Election lifecycle registration",
        |rocket| {
            Box::pin(async move {
                let shutdown = rocket.shutdown();
                tokio::spawn(async move {
                    let mut registered = false;
                    loop {
                        let update = async {
                            if registered {
                                election_lifecycle::heartbeat("harvest").await
                            } else {
                                election_lifecycle::register_startup("harvest")
                                    .await
                            }
                        };
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            update,
                        )
                        .await
                        {
                            Ok(Ok(())) => registered = true,
                            _ => tracing::warn!(
                                "Harvest lifecycle lease could not be updated"
                            ),
                        }
                        tokio::select! {
                            _ = shutdown.clone() => {
                                let _ = tokio::time::timeout(std::time::Duration::from_secs(5), election_lifecycle::mark_process_stopped("harvest")).await;
                                break;
                            }
                            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {}
                        }
                    }
                });
            })
        },
    )
}
