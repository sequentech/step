// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::tenant::*;
use crate::services::jwks::upsert_realm_jwks;
use crate::types::error::Result;
use celery::error::TaskError;
use immu_board::util::get_tenant_name;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::keycloak::KeycloakAdminClient;
use tracing::{event, instrument, Level};
use std::env;

#[instrument]
pub async fn upsert_keycloak_realm(tenant_id: &str) -> Result<()> {
    let json_realm_config =
        env::var("KEYCLOAK_TENANT_REALM_CONFIG").expect(&format!("KEYCLOAK_TENANT_REALM_CONFIG must be set"));
    let client = KeycloakAdminClient::new().await?;
    let tenant_name = get_tenant_name(tenant_id);
    client.upsert_realm(tenant_name.as_str(), &json_realm_config).await?;
    upsert_realm_jwks(tenant_name.as_str()).await?;
    Ok(())
}

#[instrument(skip(auth_headers))]
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

#[instrument(skip(auth_headers))]
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

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(tenant_id: String, slug: String) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    let tenant_exists = check_tenant_exists(&auth_headers, &slug).await?;
    if tenant_exists {
        event!(Level::INFO, "Tenant with slug {} already exists", slug);
        return Ok(());
    }
    upsert_keycloak_realm(tenant_id.as_str()).await?;
    insert_tenant_db(&auth_headers, &tenant_id, &slug).await?;
    Ok(())
}
