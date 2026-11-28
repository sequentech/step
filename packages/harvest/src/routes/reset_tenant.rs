// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Settings → Configuration package → Reset: returns the admin's own tenant
//! to what it was before its first configuration package, so the package
//! and its roster can be imported again. See `windmill::services::tenant_reset`.

use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::tenant_activity::{TenantAction, TenantActivityRecorder};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::tasks_execution::{post, update_fail};
use windmill::services::tenant_reset::{
    self, Preview, Refusal, ResetOptions, ERASE_BALLOTS_PHRASE,
};
use windmill::tasks::reset_tenant::reset_tenant_t;
use windmill::types::tasks::ETasksExecution;

/// How recent the admin's sign-in must be to reset.
const FRESH_AUTH_SECONDS: i64 = 300;

#[derive(Deserialize, Debug)]
pub struct TenantResetPreviewInput {
    tenant_id: String,
}

#[derive(Serialize, Debug)]
pub struct TenantResetPreviewOutput {
    #[serde(flatten)]
    preview: Preview,
    /// What the admin types to confirm.
    slug: String,
    /// The admin must sign in again before resetting.
    fresh_login_required: bool,
}

#[derive(Deserialize, Debug)]
pub struct ResetTenantInput {
    tenant_id: String,
    /// The tenant's slug, typed by the admin.
    confirmation: String,
    /// `ERASE_BALLOTS_PHRASE`, typed by the admin, to erase cast or sealed
    /// ballots where the deployment allows it.
    #[serde(default)]
    erase_ballots_confirmation: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ResetTenantOutput {
    task_execution: TasksExecution,
}

type RouteError = (Status, String);

fn internal(error: impl std::fmt::Display) -> RouteError {
    (Status::InternalServerError, error.to_string())
}

fn is_fresh_login(claims: &JwtClaims, now: i64) -> bool {
    signed_in_recently(claims.auth_time, now)
}

fn signed_in_recently(auth_time: Option<i64>, now: i64) -> bool {
    auth_time
        .and_then(|authenticated| now.checked_sub(authenticated))
        .is_some_and(|age| (0..=FRESH_AUTH_SECONDS).contains(&age))
}

/// The admin resets their own tenant only.
fn own_tenant(
    claims: &JwtClaims,
    tenant_id: &str,
) -> Result<String, RouteError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TENANT_RESET],
    )?;
    let target = Uuid::parse_str(tenant_id)
        .map_err(|_| (Status::BadRequest, "Invalid tenant ID".to_string()))?;
    let caller =
        Uuid::parse_str(&claims.hasura_claims.tenant_id).map_err(|_| {
            (Status::Unauthorized, "Invalid tenant identity".to_string())
        })?;
    if target != caller {
        return Err((
            Status::Forbidden,
            "A tenant can only be reset by its own admins".to_string(),
        ));
    }
    Ok(target.to_string())
}

/// The tenant's slug, and whether it is the super-admin tenant while
/// other tenants exist: its realm holds the admins of every other tenant.
async fn tenant_facts(tenant_id: &str) -> Result<(String, bool), RouteError> {
    let client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tenant = Uuid::parse_str(tenant_id).map_err(internal)?;
    let slug: String = client
        .query_one(
            "SELECT slug FROM sequent_backend.tenant WHERE id = $1",
            &[&tenant],
        )
        .await
        .map_err(internal)?
        .get(0);
    let super_admin = std::env::var("SUPER_ADMIN_TENANT_ID")
        .ok()
        .and_then(|id| Uuid::parse_str(&id).ok())
        .is_some_and(|id| id == tenant);
    let shared = if super_admin {
        let others: i64 = client
            .query_one(
                "SELECT count(*) FROM sequent_backend.tenant WHERE id <> $1",
                &[&tenant],
            )
            .await
            .map_err(internal)?
            .get(0);
        others > 0
    } else {
        false
    };
    Ok((slug, shared))
}

/// Refusals that stop a reset the admin confirmed: a ballot refusal does
/// not when the deployment allows erasing them and the admin typed the
/// phrase.
pub fn blocking_refusals(
    refusals: &[Refusal],
    erase_ballots: bool,
) -> Vec<Refusal> {
    refusals
        .iter()
        .filter(|refusal| {
            !matches!(
                refusal,
                Refusal::Ballots {
                    overridable: true,
                    ..
                } if erase_ballots
            )
        })
        .cloned()
        .collect()
}

#[instrument(skip(claims))]
#[post("/tenant-reset-preview", format = "json", data = "<body>")]
pub async fn tenant_reset_preview(
    body: Json<TenantResetPreviewInput>,
    claims: JwtClaims,
) -> Result<Json<TenantResetPreviewOutput>, RouteError> {
    let tenant_id = own_tenant(&claims, &body.tenant_id)?;
    let (slug, shared_super_admin) = tenant_facts(&tenant_id).await?;
    let mut preview =
        tenant_reset::preview(&tenant_id).await.map_err(internal)?;
    if shared_super_admin {
        preview.refusals.push(Refusal::SuperAdminTenant);
    }
    Ok(Json(TenantResetPreviewOutput {
        preview,
        slug,
        fresh_login_required: !is_fresh_login(
            &claims,
            chrono::Utc::now().timestamp(),
        ),
    }))
}

#[instrument(skip(claims, activity))]
#[post("/reset-tenant", format = "json", data = "<body>")]
pub async fn reset_tenant_f(
    body: Json<ResetTenantInput>,
    claims: JwtClaims,
    activity: TenantActivityRecorder<'_>,
) -> Result<Json<ResetTenantOutput>, RouteError> {
    let input = body.into_inner();
    activity.record(
        &claims,
        TenantAction::ResetTenant,
        json!({"tenant_id": input.tenant_id}),
    );
    let tenant_id = own_tenant(&claims, &input.tenant_id)?;
    if !is_fresh_login(&claims, chrono::Utc::now().timestamp()) {
        return Err((
            Status::Forbidden,
            "Sign in again before resetting the tenant".to_string(),
        ));
    }
    let (slug, shared_super_admin) = tenant_facts(&tenant_id).await?;
    if shared_super_admin {
        return Err((
            Status::Conflict,
            "The super-admin tenant can't be reset while other tenants exist"
                .to_string(),
        ));
    }
    if input.confirmation.trim() != slug {
        return Err((
            Status::BadRequest,
            "Type the tenant's identifier to confirm".to_string(),
        ));
    }
    let erase_ballots = input
        .erase_ballots_confirmation
        .as_deref()
        .is_some_and(|phrase| phrase.trim() == ERASE_BALLOTS_PHRASE);
    let preview = tenant_reset::preview(&tenant_id).await.map_err(internal)?;
    let blocking = blocking_refusals(&preview.refusals, erase_ballots);
    if !blocking.is_empty() {
        return Err((
            Status::Conflict,
            serde_json::to_string(&blocking).map_err(internal)?,
        ));
    }

    let executor_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &tenant_id,
        None,
        ETasksExecution::RESET_TENANT,
        &executor_name,
    )
    .await
    .map_err(|error| {
        internal(format!("Failed to insert task execution record: {error:?}"))
    })?;

    let sent = get_celery_app()
        .await
        .send_task(reset_tenant_t::new(
            tenant_id.clone(),
            claims.hasura_claims.user_id.clone(),
            ResetOptions { erase_ballots },
            task_execution.clone(),
            Some(audit::actor(&claims)),
        ))
        .await;
    if let Err(error) = sent {
        let message = format!("Error sending the Reset Tenant task: {error}");
        let _ = update_fail(&task_execution, &message).await;
        return Err(internal(message));
    }
    Ok(Json(ResetTenantOutput { task_execution }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_phrase_lifts_only_an_overridable_ballot_refusal() {
        let overridable = Refusal::Ballots {
            cast: 1,
            sealed: 0,
            overridable: true,
        };
        let fixed = Refusal::Ballots {
            cast: 1,
            sealed: 0,
            overridable: false,
        };
        assert!(blocking_refusals(&[overridable.clone()], true).is_empty());
        assert_eq!(
            blocking_refusals(&[overridable.clone()], false),
            [overridable]
        );
        assert_eq!(blocking_refusals(&[fixed.clone()], true), [fixed]);
        assert_eq!(
            blocking_refusals(&[Refusal::Disabled], true),
            [Refusal::Disabled]
        );
    }

    #[test]
    fn a_reset_needs_a_recent_sign_in() {
        assert!(signed_in_recently(Some(1_000), 1_000 + FRESH_AUTH_SECONDS));
        assert!(!signed_in_recently(Some(1_000), 1_001 + FRESH_AUTH_SECONDS));
        assert!(!signed_in_recently(Some(1_000), 999));
        assert!(!signed_in_recently(None, 1_000));
    }
}
