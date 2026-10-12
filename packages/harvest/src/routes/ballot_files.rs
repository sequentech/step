// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::{http::Status, serde::json::Json};
use sequent_core::services::{
    authorization::authorize_voter_event, jwt::JwtClaims,
};
use sequent_core::types::permissions::VoterPermissions;
use serde::Deserialize;
use serde_json::Value;
use windmill::services::{
    ballot_styles::publication_files::{load_voter_files, presign_voter_files},
    database::get_hasura_pool,
};

#[derive(Deserialize)]
pub struct BallotFilesInput {
    election_event_id: String,
}

#[post("/get-ballot-files-urls", format = "json", data = "<body>")]
pub async fn get_ballot_files_urls(
    body: Json<BallotFilesInput>,
    claims: JwtClaims,
) -> Result<Json<Value>, (Status, String)> {
    let (area, elections) = authorize_voter_event(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &body.election_event_id,
    )?;
    let failure = |error: anyhow::Error| {
        tracing::error!(error = %error, "Unable to load published ballot files");
        (
            Status::InternalServerError,
            "Unable to load published ballot files".to_owned(),
        )
    };
    let result: anyhow::Result<Value> = async {
        let references = {
            let mut client = get_hasura_pool().await.get().await?;
            let tx = client.transaction().await?;
            let references = load_voter_files(
                &tx,
                &claims.hasura_claims.tenant_id,
                &body.election_event_id,
                &area,
                &elections,
            )
            .await?;
            tx.commit().await?;
            references
        };
        presign_voter_files(references).await
    }
    .await;
    result.map(Json).map_err(failure)
}
