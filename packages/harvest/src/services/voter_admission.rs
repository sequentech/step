// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Orbit, Rocket};
use sequent_core::services::connection::{
    VoterAccessFailure, VoterAccessPolicy,
};
use windmill::services::{
    audit_storage, database::get_hasura_pool, election_lifecycle, vault,
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

    async fn check_staff(
        &self,
        _tenant: &str,
    ) -> Result<(), VoterAccessFailure> {
        audit_storage::require_voter_storage().await.map_err(|error| {
            tracing::warn!("Staff channel denied by audit-storage protection: {error:#}");
            VoterAccessFailure::StaffAuditStorageUnavailable
        })
    }
}

/// The deployment's readiness probe tests for this file: a rolling update stops
/// an older process only after its replacement has registered.
const READINESS_FILE: &str = "/tmp/harvest-registered";

/// Removes the readiness file an earlier run of this container may have left, so
/// it cannot mark this process ready before it has registered.
fn clear_readiness_file() -> Result<(), String> {
    match std::fs::remove_file(READINESS_FILE) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => match std::path::Path::new(READINESS_FILE).try_exists() {
            Ok(false) => Ok(()),
            _ => Err(format!(
                "The readiness file {READINESS_FILE} could not be removed: {error}"
            )),
        },
    }
}

/// Registers this process in the election lifecycle and then marks it ready.
pub struct LifecycleRegistration {
    /// Whether the readiness file was removed when the application was built.
    cleared: Result<(), String>,
}

/// Removes the readiness file now, before anything else of the application starts.
pub fn lifecycle_fairing() -> LifecycleRegistration {
    LifecycleRegistration {
        cleared: clear_readiness_file(),
    }
}

#[rocket::async_trait]
impl Fairing for LifecycleRegistration {
    fn info(&self) -> Info {
        Info {
            name: "Election lifecycle registration",
            kind: Kind::Ignite | Kind::Liftoff,
        }
    }

    /// The process does not start without its master secret, nor with a readiness
    /// file left that could mark it ready before it has registered.
    async fn on_ignite(
        &self,
        rocket: Rocket<Build>,
    ) -> rocket::fairing::Result {
        let checked = match &self.cleared {
            Ok(()) => vault::check_master_secret().await.map_err(|error| {
                format!(
                    "Harvest does not start without its master secret: {error:#}"
                )
            }),
            Err(error) => Err(error.clone()),
        };
        match checked {
            Ok(()) => Ok(rocket),
            Err(error) => {
                tracing::error!("{error}");
                Err(rocket)
            }
        }
    }

    /// Every heartbeat uses a short deadline, and startup one that fits its wait
    /// for another running process. Until startup registration succeeds there is
    /// no live lease of its own, so voter operations fail closed.
    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let shutdown = rocket.shutdown();
        tokio::spawn(async move {
            let mut registered = false;
            loop {
                let update = async {
                    if registered {
                        election_lifecycle::heartbeat("harvest").await
                    } else {
                        election_lifecycle::register_startup("harvest").await
                    }
                };
                let deadline = if registered { 5 } else { 30 };
                match tokio::time::timeout(
                    std::time::Duration::from_secs(deadline),
                    update,
                )
                .await
                {
                    Ok(Ok(())) => {
                        if !registered {
                            windmill::services::self_tests::startup_runs();
                            if let Err(error) =
                                std::fs::write(READINESS_FILE, "")
                            {
                                tracing::warn!(
                                    "Readiness file could not be written: {error}"
                                );
                            }
                        }
                        registered = true;
                    }
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
    }
}
