// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use electoral_log::{
    adapters::postgres::PostgresStore,
    proofs::{Checkpoint, Consistency, Journal, RecordProof},
};
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::{Deserialize, Serialize};
use windmill::services::protocol_manager::get_event_board;

type ApiResult<T> = Result<Json<T>, (Status, String)>;

pub struct ProofService {
    pub store: PostgresStore,
    pub journal: Journal,
}

#[derive(Deserialize)]
pub struct ProofRequest {
    tenant_id: String,
    election_event_id: String,
}
impl ProofRequest {
    fn board(&self, claims: &JwtClaims) -> Result<String, (Status, String)> {
        authorize(
            claims,
            true,
            Some(self.tenant_id.clone()),
            vec![Permissions::LOGS_READ],
        )?;
        let slug = std::env::var("ENV_SLUG").map_err(internal_error)?;
        Ok(get_event_board(
            &self.tenant_id,
            &self.election_event_id,
            &slug,
        ))
    }
}

#[derive(Deserialize)]
pub struct InclusionRequest {
    #[serde(flatten)]
    scope: ProofRequest,
    record_id: i64,
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
    committed_size: u64,
}

fn internal_error(error: impl std::fmt::Display) -> (Status, String) {
    tracing::error!("Electoral-log proof operation failed: {error}");
    (
        Status::InternalServerError,
        "Electoral-log proof operation failed".into(),
    )
}

#[post("/electoral-log/checkpoint", format = "json", data = "<body>")]
pub async fn checkpoint(
    body: Json<ProofRequest>,
    claims: JwtClaims,
    state: &State<ProofService>,
) -> ApiResult<CheckpointResponse> {
    let board = body.board(&claims)?;
    let (checkpoint, committed_size) = state
        .journal
        .checkpoint(&board)
        .await
        .map_err(internal_error)?;
    Ok(Json(CheckpointResponse {
        checkpoint,
        committed_size,
    }))
}

#[post("/electoral-log/inclusion", format = "json", data = "<body>")]
pub async fn inclusion(
    body: Json<InclusionRequest>,
    claims: JwtClaims,
    state: &State<ProofService>,
) -> ApiResult<RecordProof> {
    let board = body.scope.board(&claims)?;
    let proof = state
        .store
        .record_proof(&state.journal, &board, body.record_id)
        .await
        .map_err(internal_error)?;
    proof.map(Json).ok_or((
        Status::Accepted,
        "Entry is committed; Merkle proof is pending".into(),
    ))
}

#[post("/electoral-log/consistency", format = "json", data = "<body>")]
pub async fn consistency(
    body: Json<ConsistencyRequest>,
    claims: JwtClaims,
    state: &State<ProofService>,
) -> ApiResult<Consistency> {
    let board = body.scope.board(&claims)?;
    if body.checkpoint.log_name != board {
        return Err((
            Status::BadRequest,
            "Checkpoint belongs to another board".into(),
        ));
    }
    state
        .journal
        .consistency(&body.checkpoint)
        .await
        .map(Json)
        .map_err(|_| {
            (
                Status::BadRequest,
                "Checkpoint is not in the processed log".into(),
            )
        })
}

pub fn fairing() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::try_on_ignite(
        "Trellis electoral-log proofs",
        |rocket| async {
            let store = match PostgresStore::from_env() {
                Ok(store) => store,
                Err(error) => {
                    tracing::error!(
                        "Cannot configure electoral-log proofs: {error}"
                    );
                    return Err(rocket);
                }
            };
            let journal = store.journal();
            Ok(rocket.manage(ProofService { store, journal }).attach(
            rocket::fairing::AdHoc::on_liftoff("Trellis processor", |rocket| Box::pin(async move {
                let Some(state) = rocket.state::<ProofService>() else { return };
                let journal = state.journal.clone();
                let shutdown = rocket.shutdown();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    loop {
                        tokio::select! {
                            _ = shutdown.clone() => break,
                            _ = interval.tick() => {
                                if let Err(error) = journal.process_once().await {
                                    tracing::error!("Trellis catch-up failed; retrying: {error}");
                                }
                            }
                        }
                    }
                });
            }))
        ))
        },
    )
}
