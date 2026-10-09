// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use electoral_log::{
    adapters::{
        events::EventDatabases,
        postgres::{LogScope, PostgresStore},
    },
    ports::ElectoralLogStore,
    proofs::{Checkpoint, Consistency, JournalError, RecordProof},
};
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::{Deserialize, Serialize};
use windmill::services::protocol_manager::get_event_board;

type ApiResult<T> = Result<Json<T>, (Status, String)>;

/// Proofs use their own connection pools, separate from Windmill's.
pub struct ProofService {
    pub databases: EventDatabases,
}

#[derive(Deserialize)]
pub struct ProofRequest {
    tenant_id: String,
    election_event_id: String,
    /// A log of the event's database: the event's board, by default, or a sealed log
    /// the board continues after an import.
    #[serde(default)]
    log_name: Option<String>,
}

/// The log a request reads and the database that holds it.
struct ProofScope {
    log: String,
    store: PostgresStore,
}

impl ProofRequest {
    /// The requested log, in the database of the tenant's election event. The event's
    /// board, named after the tenant, must be in the database.
    async fn scope(
        &self,
        claims: &JwtClaims,
        databases: &EventDatabases,
    ) -> Result<ProofScope, (Status, String)> {
        authorize(
            claims,
            true,
            Some(self.tenant_id.clone()),
            vec![Permissions::LOGS_READ],
        )?;
        let slug = std::env::var("ENV_SLUG").map_err(internal_error)?;
        let board =
            get_event_board(&self.tenant_id, &self.election_event_id, &slug);
        let not_found = || {
            (
                Status::NotFound,
                "Unknown electoral log or record".to_string(),
            )
        };
        if !databases
            .has_board(&board)
            .await
            .map_err(|error| internal_error(format!("{error:#}")))?
        {
            return Err(not_found());
        }
        let store = databases
            .board_store(&board)
            .await
            .map_err(|error| internal_error(format!("{error:#}")))?;
        Ok(ProofScope {
            log: self.log_name.clone().unwrap_or(board),
            store,
        })
    }
}

#[derive(Deserialize)]
pub struct InclusionRequest {
    #[serde(flatten)]
    scope: ProofRequest,
    record_id: i64,
    /// When present, the response verifies against this checkpoint.
    #[serde(default)]
    trusted_checkpoint: Option<Checkpoint>,
}

#[derive(Deserialize)]
pub struct ConsistencyRequest {
    #[serde(flatten)]
    scope: ProofRequest,
    checkpoint: Checkpoint,
}

#[derive(Serialize)]
pub struct CheckpointResponse {
    checkpoint: Checkpoint,
}

fn internal_error(error: impl std::fmt::Display) -> (Status, String) {
    tracing::error!("Electoral-log proof operation failed: {error}");
    (
        Status::InternalServerError,
        "Electoral-log proof operation failed".into(),
    )
}

/// A checkpoint outside the log's history is a conflict (409): a possible fork or
/// rollback that monitors must investigate, not retry. Stored data that is
/// inconsistent is a server fault that operators must investigate.
fn journal_error(error: JournalError) -> (Status, String) {
    match error {
        JournalError::NotFound(_) => {
            (Status::NotFound, "Unknown electoral log or record".into())
        }
        JournalError::Diverged(_) => (Status::Conflict, error.to_string()),
        JournalError::Corrupt(_) => internal_error(&error),
        JournalError::Failed(error) => internal_error(format!("{error:#}")),
    }
}

#[post("/electoral-log/checkpoint", format = "json", data = "<body>")]
pub async fn checkpoint(
    body: Json<ProofRequest>,
    claims: JwtClaims,
    state: &State<ProofService>,
) -> ApiResult<CheckpointResponse> {
    let scope = body.scope(&claims, &state.databases).await?;
    let checkpoint = scope
        .store
        .journal()
        .checkpoint(&scope.log)
        .await
        .map_err(journal_error)?;
    Ok(Json(CheckpointResponse { checkpoint }))
}

#[post("/electoral-log/inclusion", format = "json", data = "<body>")]
pub async fn inclusion(
    body: Json<InclusionRequest>,
    claims: JwtClaims,
    state: &State<ProofService>,
) -> ApiResult<RecordProof> {
    let scope = body.scope.scope(&claims, &state.databases).await?;
    if body
        .trusted_checkpoint
        .as_ref()
        .is_some_and(|trusted| trusted.log_name != scope.log)
    {
        return Err((
            Status::BadRequest,
            "Checkpoint belongs to another board".into(),
        ));
    }
    let store = &scope.store;
    store
        .record_proof(
            &store.journal(),
            LogScope::Board(&scope.log),
            body.record_id,
            body.trusted_checkpoint.as_ref(),
        )
        .await
        .map(Json)
        .map_err(journal_error)
}

#[post("/electoral-log/consistency", format = "json", data = "<body>")]
pub async fn consistency(
    body: Json<ConsistencyRequest>,
    claims: JwtClaims,
    state: &State<ProofService>,
) -> ApiResult<Consistency> {
    let scope = body.scope.scope(&claims, &state.databases).await?;
    if body.checkpoint.log_name != scope.log {
        return Err((
            Status::BadRequest,
            "Checkpoint belongs to another board".into(),
        ));
    }
    scope
        .store
        .journal()
        .consistency(&body.checkpoint)
        .await
        .map(Json)
        .map_err(journal_error)
}

pub fn fairing() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::try_on_ignite(
        "Trellis electoral-log proofs",
        |rocket| async {
            match EventDatabases::from_env() {
                Ok(databases) => Ok(rocket.manage(ProofService { databases })),
                Err(error) => {
                    tracing::error!(
                        "Cannot configure electoral-log proofs: {error}"
                    );
                    Err(rocket)
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_errors_map_to_statuses_without_leaking_details() {
        let (status, body) = journal_error(JournalError::NotFound(
            "Log 'slug-tenant-event'".into(),
        ));
        assert_eq!(status, Status::NotFound);
        assert!(!body.contains("slug-tenant-event"), "{body}");
        assert_eq!(
            journal_error(JournalError::Diverged("other generation".into())).0,
            Status::Conflict
        );
        let (status, body) =
            journal_error(JournalError::Corrupt("node 3/1 is missing".into()));
        assert_eq!(status, Status::InternalServerError);
        assert!(!body.contains("node 3/1"), "{body}");
        let (status, body) = journal_error(JournalError::Failed(
            anyhow::anyhow!("password=secret"),
        ));
        assert_eq!(status, Status::InternalServerError);
        assert!(!body.contains("secret"), "{body}");
    }
}
