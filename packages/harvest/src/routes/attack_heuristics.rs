// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::Deserialize;
use serde_json::{json, Value};
use windmill::services::{
    attack_heuristics,
    database::get_hasura_pool,
    electoral_log_audit_deadline::AuditDeadline,
    signing::{board_gate::require_board, SigningCaller},
};
fn error(e: impl std::fmt::Debug) -> (Status, String) {
    tracing::error!("Attack detection request failed: {e:?}");
    (
        Status::BadRequest,
        "Attack detection request refused; check settings and election state"
            .into(),
    )
}
fn administrator(claims: &JwtClaims) -> Result<(), (Status, String)> {
    authorize(
        claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_USER, Permissions::TENANT_WRITE],
    )?;
    if SigningCaller::from_claims(claims)
        .has(Permissions::SIGNING_REQUESTS_CANCEL)
    {
        return Err((Status::Forbidden,"Administrator and election-board capabilities must remain separate".into()));
    }
    Ok(())
}
#[post("/attack-detection/settings/get")]
pub async fn get_attack_detection_settings(
    claims: JwtClaims,
) -> Result<Json<Value>, (Status, String)> {
    administrator(&claims)?;
    let settings = AuditDeadline::database()
        .run(attack_heuristics::settings(&claims.hasura_claims.tenant_id))
        .await
        .map_err(error)?
        .map_err(error)?;
    Ok(Json(
        json!({"tenant_id":claims.hasura_claims.tenant_id,"settings":settings}),
    ))
}
#[derive(Deserialize)]
pub struct SettingsInput {
    settings: Value,
}
#[post("/attack-detection/settings/update", format = "json", data = "<body>")]
pub async fn update_attack_detection_settings(
    claims: JwtClaims,
    body: Json<SettingsInput>,
) -> Result<Json<Value>, (Status, String)> {
    administrator(&claims)?;
    let settings = body.into_inner().settings;
    AuditDeadline::database()
        .run(attack_heuristics::update_settings(
            &claims.hasura_claims.tenant_id,
            &claims.sub,
            &settings,
        ))
        .await
        .map_err(error)?
        .map_err(error)?;
    Ok(Json(
        json!({"tenant_id":claims.hasura_claims.tenant_id,"settings":settings}),
    ))
}
#[derive(Deserialize)]
pub struct FeedInput {
    election_event_id: String,
}
#[post("/attack-detection/get", format = "json", data = "<body>")]
pub async fn get_attack_detections(
    claims: JwtClaims,
    body: Json<FeedInput>,
) -> Result<Json<Value>, (Status, String)> {
    authorize(
        &claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MONITORING_VIEW],
    )?;
    require_board(&SigningCaller::from_claims(&claims)).map_err(error)?;
    let event = body.into_inner().election_event_id;
    uuid::Uuid::parse_str(&event).map_err(error)?;
    let mut db = get_hasura_pool().await.get().await.map_err(error)?;
    let tx = db.transaction().await.map_err(error)?;
    windmill::postgres::election_event::get_election_event_by_id(
        &tx,
        &claims.hasura_claims.tenant_id,
        &event,
    )
    .await
    .map_err(error)?;
    authorize_election_permission_labels(&tx, &claims, &event, None).await?;
    tx.commit().await.map_err(error)?;
    drop(db);
    Ok(Json(
        AuditDeadline::database()
            .run(attack_heuristics::detections(
                &claims.hasura_claims.tenant_id,
                &event,
            ))
            .await
            .map_err(error)?
            .map_err(error)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID,
    };
    #[test]
    fn attack_settings_require_separate_administrator_and_tenant_write_permissions(
    ) {
        assert!(administrator(&admin_claims(
            CALLER_TENANT_ID,
            &["admin-user", "tenant-write"]
        ))
        .is_ok());
        for roles in [
            vec!["tenant-write"],
            vec!["admin-user"],
            vec!["admin-user", "tenant-write", "signing-requests-cancel"],
            vec!["monitoring-view"],
        ] {
            assert!(
                administrator(&admin_claims(CALLER_TENANT_ID, &roles)).is_err()
            );
        }
    }
}
