// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::{http::Status, serde::json::Json};
use sequent_core::services::{authorization::VoterClient, jwt::JwtClaims};
use sequent_core::types::permissions::VoterPermissions;
use serde::Deserialize;
use serde_json::Value;
use windmill::services::{
    ballot_styles::publication_files::voter_files, database::get_hasura_pool,
};

#[derive(Deserialize)]
pub struct BallotFilesInput {
    election_event_id: String,
}

fn voter_scope(
    claims: &JwtClaims,
    event: &str,
) -> Result<(String, Vec<String>), (Status, String)> {
    let denied = || (Status::Forbidden, "Voter not authorized".to_owned());
    if claims.hasura_claims.election_event_id.as_deref() != Some(event)
        || !claims
            .hasura_claims
            .allowed_roles
            .iter()
            .any(|r| r == &VoterPermissions::CAST_VOTE.to_string())
        || claims.azp.parse::<VoterClient>().is_err()
    {
        return Err(denied());
    }
    let area = claims.hasura_claims.area_id.clone().ok_or_else(denied)?;
    let elections = claims
        .hasura_claims
        .authorized_election_ids
        .clone()
        .ok_or_else(denied)?;
    Ok((area, elections))
}

#[post("/get-ballot-files-urls", format = "json", data = "<body>")]
pub async fn get_ballot_files_urls(
    body: Json<BallotFilesInput>,
    claims: JwtClaims,
) -> Result<Json<Value>, (Status, String)> {
    let (area, elections) = voter_scope(&claims, &body.election_event_id)?;
    let failure = |error: anyhow::Error| {
        tracing::error!(error = %error, "Unable to load published ballot files");
        (
            Status::InternalServerError,
            "Unable to load published ballot files".to_owned(),
        )
    };
    let result: anyhow::Result<Value> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let tx = client.transaction().await?;
        let files = voter_files(
            &tx,
            &claims.hasura_claims.tenant_id,
            &body.election_event_id,
            &area,
            &elections,
        )
        .await?;
        tx.commit().await?;
        Ok(files)
    }
    .await;
    result.map(Json).map_err(failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn claims() -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test", "sub": "voter", "typ": "Bearer", "azp": "voting-portal", "acr": "1", "allowed-origins": [], "scope": "openid", "email_verified": false,
            "https://hasura.io/jwt/claims": {"x-hasura-default-role":"user", "x-hasura-tenant-id":"tenant", "x-hasura-user-id":"voter", "x-hasura-area-id":"area", "x-hasura-election-event-id":"event", "authorized-election-ids":["election"], "x-hasura-allowed-roles":["user"]}
        })).unwrap()
    }
    #[test]
    fn publication_urls_require_event_area_role_and_voter_client() {
        let valid = claims();
        for client in [
            "voting-portal",
            "voting-portal-kiosk",
            "onsite-voting-portal",
            "ivr-voting",
        ] {
            let mut voter = valid.clone();
            voter.azp = client.into();
            assert!(voter_scope(&voter, "event").is_ok());
        }
        assert!(voter_scope(&valid, "other").is_err());
        let mut missing = valid.clone();
        missing.hasura_claims.area_id = None;
        assert!(voter_scope(&missing, "event").is_err());
        let mut missing = valid.clone();
        missing.hasura_claims.election_event_id = None;
        assert!(voter_scope(&missing, "event").is_err());
        let mut missing = valid.clone();
        missing.hasura_claims.authorized_election_ids = None;
        assert!(voter_scope(&missing, "event").is_err());
        let mut missing = valid.clone();
        missing.hasura_claims.allowed_roles.clear();
        assert!(voter_scope(&missing, "event").is_err());
        let mut missing = valid.clone();
        missing.azp = "admin-portal".into();
        assert!(voter_scope(&missing, "event").is_err());
        let mut empty = valid;
        empty.hasura_claims.authorized_election_ids = Some(vec![]);
        assert_eq!(voter_scope(&empty, "event").unwrap().1.len(), 0);
    }
}
