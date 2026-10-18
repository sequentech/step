// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Authenticated tenant plugin gateway. Input never selects the tenant or actor.
use crate::ports::identity::IdentityAdmin;
use crate::services::audit;
use crate::services::dependencies::HarvestServices;
use rocket::{
    fairing::AdHoc, http::Status, response::status::Custom, serde::json::Json,
    State,
};
use sequent_core::services::{jwt::JwtClaims, keycloak::get_tenant_realm};
use sequent_core::types::keycloak::Permission;
use sequent_core::types::permissions::Permissions;
use sequent_plugin_api::PluginError;
use sequent_plugin_host::{
    postgres::{PluginState, Registry, RoleProvisioner},
    Identity, TENANT_WRITE,
};
use sequent_plugin_projects::Projects;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

/// Bounds a remote Keycloak failure while the lifecycle row lock is held.
const PROVISIONING_TIMEOUT: Duration = Duration::from_secs(30);
const ADMIN_GROUP: &str = "admin";
const PLUGIN_ROLE_DESCRIPTION: &str = "Bundled plugin permission";

/// Every plugin compiled into this release. Remote code is never registered.
pub fn registry() -> Result<Registry, PluginError> {
    Registry::new(vec![Box::new(Projects)])
}

/// Refuses to launch when a bundled plugin is incompatible with the host.
pub fn registry_fairing() -> AdHoc {
    AdHoc::try_on_ignite("Bundled plugins", |rocket| async {
        match registry() {
            Ok(registry) => Ok(rocket.manage(registry)),
            Err(error) => {
                tracing::error!("Cannot register bundled plugins: {error}");
                Err(rocket)
            }
        }
    })
}

type ApiError = Custom<Json<Value>>;

fn api_error(error: PluginError) -> ApiError {
    let (status, code) = match &error {
        PluginError::Forbidden => (Status::Forbidden, "plugin-forbidden"),
        PluginError::Disabled => (Status::Forbidden, "plugin-disabled"),
        PluginError::Incompatible => (Status::Conflict, "plugin-incompatible"),
        PluginError::NotFound => (Status::NotFound, "plugin-not-found"),
        PluginError::Conflict(_) => (Status::Conflict, "plugin-conflict"),
        PluginError::StateConflict => {
            (Status::Conflict, "plugin-state-conflict")
        }
        PluginError::InvalidInput(_) => {
            (Status::BadRequest, "plugin-invalid-input")
        }
        PluginError::Storage(detail) => {
            tracing::error!("Plugin gateway failed: {detail}");
            (Status::InternalServerError, "plugin-unavailable")
        }
    };
    let current = match &error {
        PluginError::Conflict(record) => Some(record.as_ref()),
        _ => None,
    };
    Custom(
        status,
        Json(json!({
            "message": error.to_string(),
            "extensions": {"code": code, "current": current},
        })),
    )
}

fn unavailable(error: impl ToString) -> ApiError {
    api_error(PluginError::Storage(error.to_string()))
}

fn identity(claims: &JwtClaims) -> Result<Identity, ApiError> {
    let forbidden = || api_error(PluginError::Forbidden);
    let tenant_id = Uuid::parse_str(&claims.hasura_claims.tenant_id)
        .map_err(|_| forbidden())?;
    let tenant_realm =
        format!("/realms/{}", get_tenant_realm(&tenant_id.to_string()));
    // A signed event-realm token is not an administrative tenant session, even
    // if an event realm happens to define a similarly named role.
    if claims.hasura_claims.election_event_id.is_some()
        || !claims.iss.ends_with(&tenant_realm)
        || claims.sub != claims.hasura_claims.user_id
    {
        return Err(forbidden());
    }
    let identity = Identity {
        tenant_id,
        user_id: claims.sub.clone(),
        username: claims
            .preferred_username
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| claims.sub.clone()),
        roles: claims.hasura_claims.allowed_roles.clone(),
    };
    identity
        .require(&Permissions::ADMIN_USER.to_string())
        .map_err(api_error)?;
    Ok(identity)
}

struct KeycloakRoles<'a>(&'a dyn IdentityAdmin);

#[rocket::async_trait]
impl RoleProvisioner for KeycloakRoles<'_> {
    async fn ensure_admin_roles(
        &self,
        tenant: Uuid,
        permissions: &[&str],
    ) -> Result<(), PluginError> {
        let storage =
            |error: anyhow::Error| PluginError::Storage(error.to_string());
        let realm = get_tenant_realm(&tenant.to_string());
        for permission in permissions {
            let (matches, _) = self
                .0
                .client()
                .await
                .map_err(storage)?
                .list_permissions(
                    &realm,
                    Some((*permission).into()),
                    None,
                    None,
                )
                .await
                .map_err(storage)?;
            if !matches
                .iter()
                .any(|role| role.name.as_deref() == Some(permission))
            {
                self.0
                    .client()
                    .await
                    .map_err(storage)?
                    .create_permission(
                        &realm,
                        &Permission {
                            id: None,
                            attributes: None,
                            container_id: None,
                            description: Some(PLUGIN_ROLE_DESCRIPTION.into()),
                            name: Some((*permission).into()),
                        },
                    )
                    .await
                    .map_err(storage)?;
            }
        }
        let (groups, _) = self
            .0
            .client()
            .await
            .map_err(storage)?
            .list_roles(&realm, Some(ADMIN_GROUP.into()), None, None)
            .await
            .map_err(storage)?;
        let admin_id = groups
            .iter()
            .find(|group| group.name.as_deref() == Some(ADMIN_GROUP))
            .and_then(|group| group.id.as_deref())
            .ok_or_else(|| {
                PluginError::Storage(
                    "The tenant's admin group is missing".into(),
                )
            })?;
        // Adds role mappings; the group's other permissions are unchanged.
        self.0
            .client()
            .await
            .map_err(storage)?
            .set_role_permissions(
                &realm,
                admin_id,
                &permissions.iter().map(|name| (*name).to_owned()).collect(),
            )
            .await
            .map_err(storage)
    }
}

#[derive(Serialize)]
pub struct Output {
    data: Value,
}

#[get("/plugins")]
pub async fn tenant_plugins(
    claims: JwtClaims,
    registry: &State<Registry>,
    services: &State<HarvestServices>,
) -> Result<Json<Output>, ApiError> {
    let identity = identity(&claims)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let plugins = registry.list(&tx, &identity).await.map_err(api_error)?;
    Ok(Json(Output {
        data: json!({ "plugins": plugins }),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetState {
    plugin_id: String,
    state: PluginState,
    expected_version: i32,
}

#[post("/plugins/state", format = "json", data = "<input>")]
pub async fn set_tenant_plugin_state(
    claims: JwtClaims,
    input: Json<SetState>,
    registry: &State<Registry>,
    services: &State<HarvestServices>,
) -> Result<Json<Output>, ApiError> {
    let identity = identity(&claims)?;
    identity.require(TENANT_WRITE).map_err(api_error)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    audit::act_for(&tx, &claims)
        .await
        .map_err(|(_, message)| unavailable(message))?;
    // Partial role provisioning is safe to retry; the enabled state is only
    // committed after every role and group mapping exists.
    tokio::time::timeout(
        PROVISIONING_TIMEOUT,
        registry.set_state(
            &tx,
            &identity,
            &input.plugin_id,
            input.state,
            input.expected_version,
            &KeycloakRoles(services.identity.as_ref()),
        ),
    )
    .await
    .map_err(unavailable)?
    .map_err(api_error)?;
    let plugins = registry.list(&tx, &identity).await.map_err(api_error)?;
    tx.commit().await.map_err(unavailable)?;
    kick_audit_log_outbox();
    Ok(Json(Output {
        data: json!({ "plugins": plugins }),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execute {
    plugin_id: String,
    operation: String,
    data: Value,
}

#[post("/plugins/execute", format = "json", data = "<input>")]
pub async fn execute_plugin_operation(
    claims: JwtClaims,
    input: Json<Execute>,
    registry: &State<Registry>,
    services: &State<HarvestServices>,
) -> Result<Json<Output>, ApiError> {
    let identity = identity(&claims)?;
    let request = input.into_inner();
    registry
        .authorize(&identity, &request.plugin_id, &request.operation)
        .map_err(api_error)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    audit::act_for(&tx, &claims)
        .await
        .map_err(|(_, message)| unavailable(message))?;
    // Dropping the transaction on any error rolls back every write.
    let data = registry
        .execute(
            &tx,
            &identity,
            &request.plugin_id,
            &request.operation,
            request.data,
        )
        .await
        .map_err(api_error)?;
    tx.commit().await.map_err(unavailable)?;
    kick_audit_log_outbox();
    Ok(Json(Output { data }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::admin_claims;
    use chrono::Utc;
    use sequent_plugin_api::Record;

    const TENANT: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";

    fn tenant_admin(roles: &[&str]) -> JwtClaims {
        let mut claims = admin_claims(TENANT, roles);
        claims.iss = format!("https://login.example/realms/tenant-{TENANT}");
        claims.sub = claims.hasura_claims.user_id.clone();
        claims
    }

    #[test]
    fn scope_comes_only_from_a_tenant_admin_token() {
        let mut claims = tenant_admin(&["admin-user", "projects-read"]);
        assert_eq!(identity(&claims).unwrap().tenant_id.to_string(), TENANT);

        claims.iss.push_str("-event-other");
        assert!(identity(&claims).is_err());

        let mut claims = tenant_admin(&["admin-user"]);
        claims.sub = "different-user".into();
        assert!(identity(&claims).is_err());

        let mut claims = tenant_admin(&["admin-user"]);
        claims.hasura_claims.election_event_id =
            Some(Uuid::new_v4().to_string());
        assert!(identity(&claims).is_err());

        let mut claims = tenant_admin(&["admin-user"]);
        claims.hasura_claims.tenant_id = "not-a-uuid".into();
        assert!(identity(&claims).is_err());
    }

    #[test]
    fn tokens_without_the_admin_role_are_refused() {
        let error = identity(&tenant_admin(&["projects-read"])).unwrap_err();
        assert_eq!(error.0, Status::Forbidden);
        assert_eq!(error.1 .0["extensions"]["code"], "plugin-forbidden");
    }

    #[test]
    fn requests_cannot_choose_a_tenant_or_unknown_fields() {
        assert!(serde_json::from_value::<Execute>(json!({
            "plugin_id": "projects", "operation": "list-projects",
            "data": {}, "tenant_id": TENANT,
        }))
        .is_err());
        assert!(serde_json::from_value::<SetState>(json!({
            "plugin_id": "projects", "state": "enabled",
            "expected_version": 0, "tenant_id": TENANT,
        }))
        .is_err());
        assert!(serde_json::from_value::<SetState>(json!({
            "plugin_id": "projects", "state": "on", "expected_version": 0,
        }))
        .is_err());
    }

    #[test]
    fn errors_have_stable_codes_without_database_details() {
        let error =
            api_error(PluginError::Storage("private database detail".into()));
        assert_eq!(error.0, Status::InternalServerError);
        assert_eq!(error.1 .0["extensions"]["code"], "plugin-unavailable");
        assert!(!error.1 .0.to_string().contains("private database detail"));

        for (error, status, code) in [
            (PluginError::Disabled, Status::Forbidden, "plugin-disabled"),
            (
                PluginError::Incompatible,
                Status::Conflict,
                "plugin-incompatible",
            ),
            (PluginError::NotFound, Status::NotFound, "plugin-not-found"),
            (
                PluginError::StateConflict,
                Status::Conflict,
                "plugin-state-conflict",
            ),
            (
                PluginError::InvalidInput("Title is required".into()),
                Status::BadRequest,
                "plugin-invalid-input",
            ),
        ] {
            let response = api_error(error);
            assert_eq!(response.0, status);
            assert_eq!(response.1 .0["extensions"]["code"], code);
        }
    }

    #[test]
    fn conflicts_return_the_current_record_for_an_explicit_reload() {
        let current = Record {
            id: Uuid::new_v4(),
            kind: "task".into(),
            parent_id: None,
            data: json!({"title": "Their version"}),
            version: 2,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            updated_by: "planner-b".into(),
            updated_by_name: "planner.b".into(),
        };
        let response = api_error(PluginError::Conflict(Box::new(current)));
        assert_eq!(response.0, Status::Conflict);
        let extensions = &response.1 .0["extensions"];
        assert_eq!(extensions["code"], "plugin-conflict");
        assert_eq!(extensions["current"]["version"], 2);
        assert_eq!(extensions["current"]["data"]["title"], "Their version");
    }

    #[test]
    fn the_bundled_registry_is_compatible() {
        assert!(registry().is_ok());
    }
}

#[cfg(test)]
#[path = "../../tests/support/plugin_platform_routes.rs"]
mod route_tests;
