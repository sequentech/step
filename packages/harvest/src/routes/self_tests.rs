// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::{
    self_tests::{SelfTestSettings, SelfTestTrigger},
    services::{jwt::JwtClaims, uuid_validation::parse_uuid_v4},
    types::permissions::Permissions,
};
use serde::Deserialize;
use serde_json::{json, Value};
use windmill::services::{
    database::get_hasura_pool, electoral_log_audit_deadline::AuditDeadline,
    self_tests,
};
#[derive(Debug, Deserialize)]
pub struct Input {
    election_event_id: String,
    trigger: Option<SelfTestTrigger>,
    settings: Option<SelfTestSettings>,
}
fn error(e: impl std::fmt::Debug) -> (Status, String) {
    tracing::error!("Self-test request failed: {e:?}");
    (
        Status::BadRequest,
        "Self-test request failed; review the event state and server log"
            .into(),
    )
}
async fn authorize_event(
    claims: &JwtClaims,
    event: &str,
    permission: Permissions,
) -> Result<(), (Status, String)> {
    parse_uuid_v4(event).map_err(|_| {
        (Status::BadRequest, "Invalid election event ID".into())
    })?;
    authorize(
        claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![permission],
    )?;
    let mut c = get_hasura_pool().await.get().await.map_err(error)?;
    let tx = c.transaction().await.map_err(error)?;
    windmill::postgres::election_event::get_election_event_by_id(
        &tx,
        &claims.hasura_claims.tenant_id,
        event,
    )
    .await
    .map_err(error)?;
    authorize_election_permission_labels(&tx, claims, event, None).await
}
#[post("/self-tests/run", format = "json", data = "<body>")]
pub async fn run_self_tests(
    body: Json<Input>,
    claims: JwtClaims,
) -> Result<Json<Value>, (Status, String)> {
    let input = body.into_inner();
    AuditDeadline::database()
        .run(authorize_event(
            &claims,
            &input.election_event_id,
            Permissions::SELF_TEST_RUN,
        ))
        .await
        .map_err(error)??;
    let trigger = input.trigger.unwrap_or(SelfTestTrigger::Requested);
    if !matches!(
        trigger,
        SelfTestTrigger::Requested
            | SelfTestTrigger::BeforeStart
            | SelfTestTrigger::BeforeResume
            | SelfTestTrigger::BeforeCounting
    ) {
        return Err((
            Status::BadRequest,
            "This trigger is reserved for the system".into(),
        ));
    }
    let task = self_tests::enqueue(
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        trigger,
        &claims.sub,
    )
    .await
    .map_err(error)?;
    Ok(Json(json!({"task_id":task.id,"state":"queued"})))
}
#[post("/self-tests/get", format = "json", data = "<body>")]
pub async fn get_self_tests(
    body: Json<Input>,
    claims: JwtClaims,
) -> Result<Json<Value>, (Status, String)> {
    let input = body.into_inner();
    let deadline = AuditDeadline::database();
    deadline
        .run(authorize_event(
            &claims,
            &input.election_event_id,
            Permissions::SELF_TEST_READ,
        ))
        .await
        .map_err(error)??;
    Ok(Json(
        deadline
            .run(self_tests::status(
                &claims.hasura_claims.tenant_id,
                &input.election_event_id,
            ))
            .await
            .map_err(error)?
            .map_err(error)?,
    ))
}
#[post("/self-tests/settings", format = "json", data = "<body>")]
pub async fn update_self_test_settings(
    body: Json<Input>,
    claims: JwtClaims,
) -> Result<Json<Value>, (Status, String)> {
    let input = body.into_inner();
    let deadline = AuditDeadline::database();
    deadline
        .run(authorize_event(
            &claims,
            &input.election_event_id,
            Permissions::SELF_TEST_SETTINGS_WRITE,
        ))
        .await
        .map_err(error)??;
    let settings = input
        .settings
        .ok_or((Status::BadRequest, "Settings are required".into()))?;
    deadline
        .run(self_tests::update_settings(
            &claims.hasura_claims.tenant_id,
            &input.election_event_id,
            &settings,
        ))
        .await
        .map_err(error)?
        .map_err(error)?;
    Ok(Json(json!({"settings":settings})))
}
