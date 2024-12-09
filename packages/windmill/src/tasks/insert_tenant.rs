// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::tenant::*;
use crate::services::jwks::upsert_realm_jwks;
use crate::types::error::Result;
use celery::error::TaskError;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use std::{env, fs};
use tracing::{event, instrument, Level};

#[instrument(err)]
pub async fn upsert_keycloak_realm(tenant_id: &str, slug: &str) -> Result<()> {
    let realm_config_path = env::var("KEYCLOAK_TENANT_REALM_CONFIG_PATH")
        .expect(&format!("KEYCLOAK_TENANT_REALM_CONFIG_PATH must be set"));
    let realm_config = fs::read_to_string(&realm_config_path)
        .expect(&format!("Should have been able to read the configuration file at KEYCLOAK_TENANT_REALM_CONFIG_PATH={realm_config_path}"));
    let client = KeycloakAdminClient::new().await?;
    let realm = get_tenant_realm(tenant_id);
    client
        .upsert_realm(
            realm.as_str(),
            &realm_config,
            tenant_id,
            true,
            Some(slug.to_string()),
            None,
        )
        .await?;
    upsert_realm_jwks(realm.as_str()).await?;
    Ok(())
}

#[instrument(skip(auth_headers), err)]
pub async fn insert_tenant_db(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    slug: &str,
) -> Result<()> {
    // fetch tenant
    let found_tenant = get_tenant(auth_headers.clone(), tenant_id.to_string())
        .await?
        .data
        .expect("expected data".into())
        .sequent_backend_tenant;

    if found_tenant.len() > 0 {
        event!(Level::INFO, "Tenant with id {} already exists", tenant_id);
        return Ok(());
    }

    let _hasura_response = insert_tenant(auth_headers.clone(), tenant_id, slug).await?;

    Ok(())
}

#[instrument(skip(auth_headers), err)]
pub async fn check_tenant_exists(
    auth_headers: &connection::AuthHeaders,
    slug: &str,
) -> Result<bool> {
    // fetch tenant
    let found_tenant = get_tenant_by_slug(auth_headers.clone(), slug.to_string())
        .await?
        .data
        .expect("expected data".into())
        .sequent_backend_tenant;
    Ok(found_tenant.len() > 0)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(tenant_id: String, slug: String) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    let tenant_exists = check_tenant_exists(&auth_headers, &slug).await?;
    if tenant_exists {
        event!(Level::INFO, "Tenant with slug {} already exists", slug);
        return Ok(());
    }
    upsert_keycloak_realm(tenant_id.as_str(), slug.as_str()).await?;
    insert_tenant_db(&auth_headers, &tenant_id, &slug).await?;
    Ok(())
}
