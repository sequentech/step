// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;

use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::{get_client_credentials};


use tracing::{event, Level, instrument};

use crate::hasura::tenant::*;


use crate::types::error::Result;


#[instrument(skip(auth_headers))]
pub async fn insert_tenant_db(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    slug: &str,
) -> Result<()> {
    // fetch tenant
    let found_tenant = get_tenant(
        auth_headers.clone(),
        tenant_id.to_string(),
    )
        .await?
        .data
        .expect("expected data".into())
        .sequent_backend_tenant;

    if found_tenant.len() > 0 {
        event!(Level::INFO, "Tenant {} already exists", tenant_id);
        return Ok(())
    }

    let _hasura_response = insert_tenant(auth_headers.clone(), tenant_id, slug).await?;

    Ok(())
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_tenant(id: String, slug: String) -> Result<()> {
    let auth_headers = get_client_credentials().await?;
    insert_tenant_db(&auth_headers, &slug, &id).await?;
    Ok(())
}
