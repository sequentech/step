// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::apply_configuration_package::PACKAGE_PERMISSIONS;
use crate::services::{audit, authorization::authorize};
use rocket::{http::Status, serde::json::Json};
use sequent_core::election_config::webauthn_proof::MAX_REQUIRED_APPROVALS;
use sequent_core::services::jwt::JwtClaims;
use serde::{Deserialize, Serialize};
use serde_json::json;
use windmill::services::{
    audit_log::{self, AdminAction, AuditObject, FieldChange},
    database::get_hasura_pool,
};

type RouteError = (Status, String);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadInput {
    tenant_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteInput {
    tenant_id: String,
    required_approvals: Option<i16>,
    generation: String,
}

#[derive(Serialize)]
pub struct ApprovalPolicy {
    required_approvals: Option<i16>,
    generation: String,
}

fn internal(_: impl std::fmt::Display) -> RouteError {
    (
        Status::InternalServerError,
        "Architect approval policy unavailable".into(),
    )
}

fn authorized(claims: &JwtClaims, tenant: &str) -> Result<(), RouteError> {
    authorize(
        claims,
        true,
        Some(tenant.to_owned()),
        PACKAGE_PERMISSIONS.to_vec(),
    )
}

fn validate(required: Option<i16>) -> Result<(), RouteError> {
    if required.is_some_and(|n| n < 1 || n as usize > MAX_REQUIRED_APPROVALS) {
        return Err((
            Status::BadRequest,
            "Required approvals must be blank or an integer from 1 to 16"
                .into(),
        ));
    }
    Ok(())
}

const READ: &str =
    "SELECT p.required_approvals, COALESCE(p.generation, 0::bigint)
    FROM sequent_backend.tenant t
    LEFT JOIN sequent_backend.architect_approval_policy p ON p.tenant_id = t.id
    WHERE t.id = $1::text::uuid";

fn absent() -> RouteError {
    (Status::NotFound, "Tenant not found".into())
}

#[post("/get-architect-approval-policy", format = "json", data = "<body>")]
pub async fn get_architect_approval_policy(
    body: Json<ReadInput>,
    claims: JwtClaims,
) -> Result<Json<ApprovalPolicy>, RouteError> {
    authorized(&claims, &body.tenant_id)?;
    let client = get_hasura_pool().await.get().await.map_err(internal)?;
    let row = client
        .query_opt(READ, &[&body.tenant_id])
        .await
        .map_err(internal)?
        .ok_or_else(absent)?;
    Ok(Json(ApprovalPolicy {
        required_approvals: row.get(0),
        generation: row.get::<_, i64>(1).to_string(),
    }))
}

#[post("/set-architect-approval-policy", format = "json", data = "<body>")]
pub async fn set_architect_approval_policy(
    body: Json<WriteInput>,
    claims: JwtClaims,
) -> Result<Json<ApprovalPolicy>, RouteError> {
    authorized(&claims, &body.tenant_id)?;
    validate(body.required_approvals)?;
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    // A legacy Architect activation checks the original quorum under a
    // shared lock in this same authority database.
    tx.execute(
        "SELECT pg_advisory_xact_lock(hashtextextended('architect-approval-policy:' || $1::text::uuid::text, 0))",
        &[&body.tenant_id],
    ).await.map_err(internal)?;
    tx.query_opt("SELECT id FROM sequent_backend.tenant WHERE id = $1::text::uuid FOR UPDATE", &[&body.tenant_id])
        .await.map_err(internal)?.ok_or_else(absent)?;
    let row = tx
        .query_one(READ, &[&body.tenant_id])
        .await
        .map_err(internal)?;
    let old: Option<i16> = row.get(0);
    let mut generation: i64 = row.get(1);
    if generation.to_string() != body.generation {
        return Err((
            Status::Conflict,
            "The policy changed; reload Tenant Settings before saving".into(),
        ));
    }
    if old != body.required_approvals {
        generation = generation
            .checked_add(1)
            .ok_or_else(|| internal("generation exhausted"))?;
        tx.execute("INSERT INTO sequent_backend.architect_approval_policy
            (tenant_id, required_approvals, generation) VALUES ($1::text::uuid, $2, $3)
            ON CONFLICT (tenant_id) DO UPDATE
            SET required_approvals = EXCLUDED.required_approvals, generation = EXCLUDED.generation",
            &[&body.tenant_id, &body.required_approvals, &generation]).await.map_err(internal)?;
        let change = audit::change(&claims, audit::target(&body.tenant_id, None)?,
            AuditObject::new("tenant", &body.tenant_id, None), AdminAction::Update)
            .with_changes(vec![FieldChange::new("architect_approval_policy.required_approvals",
                Some(&json!(old)), Some(&json!(body.required_approvals)))])
            .with_extra(json!({"generation": generation, "previous_effective": old.unwrap_or(2),
                "effective": body.required_approvals.unwrap_or(2)}))
            .with_description("Updated the tenant's Architect revision approval policy");
        audit_log::stage(&tx, &change).await.map_err(internal)?;
    }
    tx.commit().await.map_err(internal)?;
    if old != body.required_approvals {
        windmill::tasks::audit_log_outbox::kick_audit_log_outbox();
    }
    Ok(Json(ApprovalPolicy {
        required_approvals: body.required_approvals,
        generation: generation.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_blank_or_supported_positive_integer() {
        for valid in [None, Some(1), Some(2), Some(16)] {
            assert!(validate(valid).is_ok());
        }
        for invalid in [Some(-1), Some(0), Some(17)] {
            assert_eq!(validate(invalid).unwrap_err().0, Status::BadRequest);
        }
    }
}
