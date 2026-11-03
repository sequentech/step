// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::tenant_activity::{TenantAction, TenantActivityRecorder};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::Tenant;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::instrument;
use windmill::postgres::tenant::get_tenant_by_id_if_exist;
use windmill::services::audit_log::{
    diff_values, AdminAction, AdminChange, AuditObject,
};
use windmill::services::limit_access_by_countries::{
    handle_limit_ip_access_by_countries, CountryCode,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct LimitAccessByCountriesInput {
    voting_countries: Vec<CountryCode>,
    enroll_countries: Vec<CountryCode>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LimitAccessByCountriesOutput {
    success: bool,
}

const VOTING_COUNTRIES: &str = "voting_countries";
const ENROLL_COUNTRIES: &str = "enroll_countries";

fn server_error(error: impl std::fmt::Debug) -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        &format!("{error:?}"),
        ErrorCode::InternalServerError,
    )
}

/// The country lists of a tenant, as its settings keep them: what the
/// rules apply. A list never set is empty.
fn country_lists(tenant: Option<&Tenant>) -> Value {
    let settings = tenant.and_then(|tenant| tenant.settings.as_ref());
    let list = |name: &str| {
        settings
            .and_then(|settings| settings.get(name))
            .filter(|list| !list.is_null())
            .cloned()
            .unwrap_or_else(|| json!([]))
    };
    json!({
        VOTING_COUNTRIES: list(VOTING_COUNTRIES),
        ENROLL_COUNTRIES: list(ENROLL_COUNTRIES),
    })
}

/// The tenant log's entry of the rules set: the country lists before and
/// after.
fn countries_change(
    claims: &JwtClaims,
    tenant: Option<&Tenant>,
    input: &LimitAccessByCountriesInput,
) -> Result<AdminChange, (Status, String)> {
    let tenant_id = &claims.hasura_claims.tenant_id;
    let after = json!({
        VOTING_COUNTRIES: input.voting_countries,
        ENROLL_COUNTRIES: input.enroll_countries,
    });
    Ok(audit::change(
        claims,
        audit::target(tenant_id, None)?,
        AuditObject::new(
            audit::TENANT,
            tenant_id,
            tenant.map(|tenant| tenant.slug.as_str()),
        ),
        AdminAction::Update,
    )
    .with_changes(diff_values(
        "",
        Some(&country_lists(tenant)),
        Some(&after),
    )))
}

/// The tenant, read before its rules change.
async fn current_tenant(
    services: &HarvestServices,
    tenant_id: &str,
) -> Result<Option<Tenant>> {
    let mut client = services.databases.hasura().await.get().await?;
    let transaction = client.transaction().await?;
    let tenant = get_tenant_by_id_if_exist(&transaction, tenant_id).await?;
    transaction.commit().await?;
    Ok(tenant)
}

#[instrument(skip(claims, services))]
#[post("/limit-access-by-countries", format = "json", data = "<body>")]
pub async fn limit_access_by_countries(
    claims: JwtClaims,
    body: Json<LimitAccessByCountriesInput>,
    activity: TenantActivityRecorder<'_>,
    services: &State<HarvestServices>,
) -> Result<Json<LimitAccessByCountriesOutput>, JsonError> {
    let input = body.into_inner();
    activity.record(
        &claims,
        TenantAction::LimitAccessByCountries,
        json!({
            "voting_countries": input.voting_countries,
            "enroll_countries": input.enroll_countries,
        }),
    );
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::CLOUDFLARE_WRITE],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{e:?}"),
            ErrorCode::Unauthorized,
        )
    })?;

    info!(
        "Limiting access to tenant {} by countries: {:?} and enroll {:?}",
        claims.hasura_claims.tenant_id,
        input.voting_countries,
        input.enroll_countries
    );

    let tenant = current_tenant(services, &claims.hasura_claims.tenant_id)
        .await
        .map_err(server_error)?;
    let entry = countries_change(&claims, tenant.as_ref(), &input)
        .map_err(server_error)?;

    handle_limit_ip_access_by_countries(
        claims.hasura_claims.tenant_id.clone(),
        input.voting_countries,
        input.enroll_countries,
    )
    .await
    .map_err(server_error)?;

    // Cloudflare holds the rules: the entry is queued once they are set.
    audit::record(services, &entry)
        .await
        .map_err(server_error)?;

    Ok(Json(LimitAccessByCountriesOutput { success: true }))
}

#[cfg(test)]
#[path = "../../tests/support/limit_access_by_countries_routes.rs"]
mod route_tests;
