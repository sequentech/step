// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::celery_app::get_celery_app;
use windmill::services::tasks_execution::*;
use windmill::tasks::delete_tenant;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteTenantOutput {
    id: String,
    task_execution: TasksExecution,
    error_msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteTenantInput {
    tenant_id: String,
}

/// Only the super-admin tenant passes `authorize` here, and it holds the
/// realm of the administrators who manage every other tenant, so it can never
/// be the deletion target.
fn check_deletion_target(
    claims: &JwtClaims,
    tenant_id: &str,
) -> Result<(), (Status, String)> {
    let target = parse_uuid_v4(tenant_id)
        .map_err(|_| (Status::BadRequest, "Invalid tenant ID".to_string()))?;
    let caller =
        parse_uuid_v4(&claims.hasura_claims.tenant_id).map_err(|_| {
            (Status::Unauthorized, "Invalid tenant identity".to_string())
        })?;
    if target == caller {
        return Err((
            Status::BadRequest,
            "The super-admin tenant cannot delete itself".to_string(),
        ));
    }
    Ok(())
}

/// Deletes a tenant. Only callable by the super-admin tenant (same
/// authorization model as insertTenant), and only once the target tenant has
/// no election events left — see count_tenant_election_events.
#[instrument(skip(claims))]
#[post("/delete-tenant", format = "json", data = "<body>")]
pub async fn delete_tenant_f(
    body: Json<DeleteTenantInput>,
    claims: JwtClaims,
) -> Result<Json<DeleteTenantOutput>, (Status, String)> {
    let input = body.into_inner();
    let executor_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let task_execution = post(
        &claims.hasura_claims.tenant_id,
        None,
        ETasksExecution::DELETE_TENANT,
        &executor_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    if let Err(error) =
        authorize(&claims, true, None, vec![Permissions::TENANT_DELETE])
    {
        let _ = update_fail(
            &task_execution,
            &format!("Failed to authorize executing the task: {error:?}"),
        )
        .await;
        return Err(error);
    };

    if let Err(error) = check_deletion_target(&claims, &input.tenant_id) {
        let _ = update_fail(&task_execution, &error.1).await;
        return Err(error);
    }

    let celery_app = get_celery_app().await;

    let realm = get_tenant_realm(&input.tenant_id);

    let celery_task_result = celery_app
        .send_task(delete_tenant::delete_tenant_t::new(
            input.tenant_id.clone(),
            realm,
            task_execution.clone(),
        ))
        .await;

    let _celery_task = match celery_task_result {
        Ok(task) => task,
        Err(error) => {
            let message = format!("Error sending Delete Tenant task: {error}");
            update_fail(&task_execution, &message)
                .await
                .map_err(|error| {
                    (
                        Status::InternalServerError,
                        format!("Failed to record dispatch failure: {error}"),
                    )
                })?;
            return Err((Status::InternalServerError, message));
        }
    };

    Ok(Json(DeleteTenantOutput {
        id: input.tenant_id,
        error_msg: None,
        task_execution,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn super_admin_claims() -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test", "sub": "admin", "typ": "Bearer", "azp": "admin-portal", "acr": "1", "allowed-origins": [], "scope": "openid", "email_verified": false,
            "https://hasura.io/jwt/claims": {"x-hasura-default-role":"admin-user", "x-hasura-tenant-id":"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "x-hasura-user-id":"admin", "x-hasura-allowed-roles":["tenant-delete"]}
        }))
        .unwrap()
    }

    #[test]
    fn the_super_admin_tenant_cannot_be_the_deletion_target() {
        let claims = super_admin_claims();
        assert_eq!(
            check_deletion_target(
                &claims,
                "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
            ),
            Err((
                Status::BadRequest,
                "The super-admin tenant cannot delete itself".to_string()
            ))
        );
        assert_eq!(
            check_deletion_target(
                &claims,
                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
            ),
            Ok(())
        );
    }
    #[test]
    fn alternate_uuid_spellings_cannot_bypass_the_self_deletion_guard() {
        let claims = super_admin_claims();
        for target in [
            "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
            "aaaaaaaaaaaa4aaa8aaaaaaaaaaaaaaa",
            "urn:uuid:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            "{aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa}",
        ] {
            assert_eq!(
                check_deletion_target(&claims, target),
                Err((
                    Status::BadRequest,
                    "The super-admin tenant cannot delete itself".into()
                )),
                "{target}"
            );
        }
    }

    #[test]
    fn malformed_or_non_v4_targets_are_rejected_before_dispatch() {
        let claims = super_admin_claims();
        for target in ["", "not-a-uuid", "00000000-0000-0000-0000-000000000000"]
        {
            assert_eq!(
                check_deletion_target(&claims, target),
                Err((Status::BadRequest, "Invalid tenant ID".into()))
            );
        }
    }

    #[test]
    fn an_invalid_caller_tenant_cannot_authorize_deletion() {
        let mut claims = super_admin_claims();
        claims.hasura_claims.tenant_id = "not-a-uuid".into();
        assert_eq!(
            check_deletion_target(
                &claims,
                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
            ),
            Err((Status::Unauthorized, "Invalid tenant identity".into()))
        );
    }
}
