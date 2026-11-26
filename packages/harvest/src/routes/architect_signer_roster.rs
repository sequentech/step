// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Receiver-owned trust provisioning, entirely separate from package imports
//! and from the Architect producer's revision approval threshold.

use crate::routes::apply_configuration_package::PACKAGE_PERMISSIONS;
use crate::services::{audit, authorization::authorize};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::{http::Status, serde::json::Json};
use sequent_core::services::{
    jwt::JwtClaims,
    keycloak::{
        get_tenant_realm, validate_keycloak_path_segment, KeycloakAdminClient,
    },
    uuid_validation::parse_uuid_v4,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use windmill::{
    postgres::configuration_packages::try_lock_package_work,
    services::{
        audit_log::{self, AdminAction, AuditObject, FieldChange},
        browser_package_policy::BrowserPolicy,
        database::get_hasura_pool,
        managed_webauthn_roster::{
            self as managed, Candidate, Config, Installed,
        },
    },
};

type RouteError = JsonError;

const DEFAULT_FLOOR: i32 = 2;
const FRESH_AUTH_SECONDS: i64 = 300;
const DIRECTORY_SEARCH_LIMIT: i32 = 20;
const DIRECTORY_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(5);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusInput {
    tenant_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewInput {
    tenant_id: String,
    roster_json: String,
    expected_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivateInput {
    tenant_id: String,
    roster_json: String,
    expected_sha256: String,
    generation: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    NotEnabled,
    SitePinned,
    PortalManaged,
    Unavailable,
}

#[derive(Serialize)]
pub struct RosterStatus {
    mode: Mode,
    generation: String,
    active_sha256: Option<String>,
    required_approvals: i32,
    can_manage: bool,
}

impl RosterStatus {
    fn inactive(mode: Mode) -> Self {
        Self {
            mode,
            generation: "0".into(),
            active_sha256: None,
            required_approvals: DEFAULT_FLOOR,
            can_manage: false,
        }
    }
}

#[derive(Serialize)]
pub struct RosterPreview {
    generation: String,
    candidate_sha256: String,
    approver_count: i32,
    final_signer_count: i32,
    rp_id: String,
    origins: Vec<String>,
}

fn internal(_: impl std::fmt::Display) -> RouteError {
    ErrorResponse::new(
        Status::InternalServerError,
        "Signer roster unavailable",
        ErrorCode::ArchitectRosterUnavailable,
    )
}

fn bad(message: &str) -> RouteError {
    ErrorResponse::new(
        Status::BadRequest,
        message,
        ErrorCode::ArchitectRosterInvalid,
    )
}

fn stale() -> RouteError {
    ErrorResponse::new(
        Status::Conflict,
        "The signer roster changed; preview again",
        ErrorCode::ArchitectRosterStaleGeneration,
    )
}

fn busy() -> RouteError {
    ErrorResponse::new(
        Status::Conflict,
        "Wait for the running package operation, then preview again",
        ErrorCode::ArchitectRosterPackageBusy,
    )
}

fn fresh_auth_required() -> RouteError {
    ErrorResponse::new(
        Status::Forbidden,
        "Sign in again before activating a signer roster",
        ErrorCode::ArchitectRosterFreshAuthRequired,
    )
}

fn authorize_tenant(
    claims: &JwtClaims,
    tenant: &str,
) -> Result<(), RouteError> {
    let uuid = parse_uuid_v4(tenant)
        .map_err(|_| bad("A canonical tenant UUID is required"))?;
    if uuid.hyphenated().to_string() != tenant {
        return Err(bad("A canonical tenant UUID is required"));
    }
    // No super-admin cross-tenant bypass. Package permissions alone never
    // confer authority to change receiver trust.
    authorize(
        claims,
        false,
        Some(tenant.to_owned()),
        PACKAGE_PERMISSIONS.to_vec(),
    )
    .map_err(|_| {
        ErrorResponse::new(
            Status::Unauthorized,
            "Tenant configuration permissions are required",
            ErrorCode::Unauthorized,
        )
    })
}

pub(super) fn is_tenant_portal_token(claims: &JwtClaims, tenant: &str) -> bool {
    // The generic JWT guard already verified the configured issuer base
    // and token signature, but also accepts election-event issuers.
    // An OIDC subject is only unique within its exact issuer realm.
    claims.typ == "Bearer"
        && claims.azp == "admin-portal"
        && claims.iss.ends_with(&format!("/realms/tenant-{tenant}"))
}

fn manager_claims(claims: &JwtClaims, tenant: &str) -> bool {
    authorize_tenant(claims, tenant).is_ok()
        && is_tenant_portal_token(claims, tenant)
        && claims.sub == claims.hasura_claims.user_id
        && claims.sub.len() <= 255
        && claims.sub.bytes().all(|byte| byte.is_ascii_graphic())
        && validate_keycloak_path_segment(&claims.sub).is_ok()
}

/// Query the currently enabled account and its exact top-level /admin
/// membership in the same tenant realm. No labels, name heuristics, or
/// group assignments from another realm can establish this authority.
async fn manager(claims: &JwtClaims, tenant: &str) -> bool {
    if !manager_claims(claims, tenant) {
        return false;
    }

    tokio::time::timeout(DIRECTORY_TIMEOUT, current_admin(&claims.sub, tenant))
        .await
        .unwrap_or(false)
}

fn human_directory_search(username: &str) -> Option<String> {
    // Keycloak tokenizes search on whitespace before applying quoted exact
    // matching. Keep the directory-returned username one ordinary search term.
    if username.is_empty()
        || username.len() > 255
        || username
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '"')
    {
        return None;
    }
    Some(format!("\"{username}\""))
}

async fn current_admin(subject: &str, tenant: &str) -> bool {
    // The standard Keycloak client's startup settings are mandatory for
    // directory access. Missing settings must fail closed on this route.
    if [
        "KEYCLOAK_URL",
        "KEYCLOAK_ADMIN_CLIENT_ID",
        "KEYCLOAK_ADMIN_CLIENT_SECRET",
        "SUPER_ADMIN_TENANT_ID",
    ]
    .into_iter()
    .any(|key| !std::env::var(key).is_ok_and(|value| !value.is_empty()))
    {
        return false;
    }
    let realm = get_tenant_realm(tenant);
    let Ok(directory) = KeycloakAdminClient::new().await else {
        return false;
    };
    let Ok(user) = directory
        .client
        .realm_users_with_user_id_get(&realm, subject, None)
        .await
    else {
        return false;
    };
    if user.id.as_deref() != Some(subject)
        || user.enabled != Some(true)
        || user.service_account_client_id.is_some()
    {
        return false;
    }

    let Ok(groups) = directory
        .client
        .realm_users_with_user_id_groups_get(
            &realm, subject, None, None, None, None,
        )
        .await
    else {
        return false;
    };
    if !groups
        .iter()
        .any(|group| group.path.as_deref() == Some("/admin"))
    {
        return false;
    }

    // GET /users/{id} need not include serviceAccountClientId. The stock
    // Keycloak directory's general search excludes service accounts at the
    // storage layer; its separate username/id filters do not. Use search
    // only to narrow the result. The exact matching subject is authoritative.
    let Some(search) =
        user.username.as_deref().and_then(human_directory_search)
    else {
        return false;
    };
    directory
        .client
        .realm_users_get(
            &realm,
            Some(true), // brief representation; only id is required
            None,
            None,
            Some(true), // enabled
            None,
            None,
            None,
            None,
            None,
            None,
            Some(DIRECTORY_SEARCH_LIMIT),
            None,
            Some(search),
            None,
        )
        .await
        .is_ok_and(|people| {
            people.iter().any(|person| {
                person.id.as_deref() == Some(subject)
                    && person.service_account_client_id.is_none()
            })
        })
}

fn forbidden_manager() -> RouteError {
    ErrorResponse::new(
        Status::Forbidden,
        "Portal signer roster management is not authorized",
        ErrorCode::ArchitectRosterManagementForbidden,
    )
}

async fn managed_for_write(
    claims: &JwtClaims,
    tenant: &str,
) -> Result<Config, RouteError> {
    authorize_tenant(claims, tenant)?;
    match BrowserPolicy::load(tenant) {
        BrowserPolicy::Managed(config) if manager(claims, tenant).await => {
            Ok(config)
        }
        _ => Err(forbidden_manager()),
    }
}

fn is_fresh_login(claims: &JwtClaims, now: i64) -> bool {
    claims
        .auth_time
        .and_then(|authenticated| now.checked_sub(authenticated))
        .is_some_and(|age| (0..=FRESH_AUTH_SECONDS).contains(&age))
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn parse_generation(input: &str) -> Result<i64, RouteError> {
    input
        .parse::<i64>()
        .ok()
        .filter(|generation| {
            *generation >= 0 && generation.to_string() == input
        })
        .ok_or_else(|| bad("The roster generation is invalid"))
}

fn validated_candidate(
    config: &Config,
    raw: &str,
    expected: &str,
) -> Result<Candidate, RouteError> {
    if !valid_fingerprint(expected) {
        return Err(bad(
            "Enter the independently confirmed lowercase SHA-256 fingerprint",
        ));
    }
    let candidate = config.candidate(raw.as_bytes()).map_err(|_| {
        bad("The roster is not valid for this trusted receiver deployment")
    })?;
    if candidate.sha256 != expected {
        return Err(bad(
            "The file does not match the independently confirmed fingerprint",
        ));
    }
    Ok(candidate)
}

fn valid_installed(config: &Config, installed: &Installed) -> bool {
    installed.generation > 0
        && valid_fingerprint(&installed.sha256)
        && config
            .candidate(installed.raw.as_bytes())
            .is_ok_and(|candidate| candidate.sha256 == installed.sha256)
}

fn ready_status(
    config: &Config,
    installed: Option<&Installed>,
    can_manage: bool,
) -> RosterStatus {
    RosterStatus {
        mode: Mode::PortalManaged,
        generation: installed.map_or(0, |row| row.generation).to_string(),
        active_sha256: installed.map(|row| row.sha256.clone()),
        required_approvals: config.required as i32,
        can_manage,
    }
}

#[post(
    "/get-architect-signer-roster-status",
    format = "json",
    data = "<body>"
)]
pub async fn get_architect_signer_roster_status(
    claims: JwtClaims,
    body: Json<StatusInput>,
) -> Result<Json<RosterStatus>, RouteError> {
    let tenant = &body.tenant_id;
    authorize_tenant(&claims, tenant)?;
    let deployment = BrowserPolicy::load(tenant);
    if matches!(
        deployment,
        BrowserPolicy::Unusable(_) | BrowserPolicy::ManagedOnly(..)
    ) {
        return Ok(Json(RosterStatus::inactive(Mode::Unavailable)));
    }
    let can_manage = matches!(&deployment, BrowserPolicy::Managed(_))
        && manager(&claims, tenant).await;
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    let installed = managed::read(&tx, tenant).await.map_err(internal)?;
    let status = match (deployment, installed.as_ref()) {
        (BrowserPolicy::Managed(config), installed) => {
            if installed.is_some_and(|row| !valid_installed(&config, row)) {
                RosterStatus::inactive(Mode::Unavailable)
            } else {
                ready_status(&config, installed, can_manage)
            }
        }
        (_, Some(_)) => RosterStatus::inactive(Mode::Unavailable),
        (BrowserPolicy::Off | BrowserPolicy::OtherTenant, None) => {
            RosterStatus::inactive(Mode::NotEnabled)
        }
        (BrowserPolicy::BrowserOnly(pinned), None) => RosterStatus {
            active_sha256: Some(pinned.sha256().into()),
            required_approvals: pinned.required() as i32,
            ..RosterStatus::inactive(Mode::SitePinned)
        },
        _ => RosterStatus::inactive(Mode::Unavailable),
    };
    Ok(Json(status))
}

#[post("/preview-architect-signer-roster", format = "json", data = "<body>")]
pub async fn preview_architect_signer_roster(
    claims: JwtClaims,
    body: Json<PreviewInput>,
) -> Result<Json<RosterPreview>, RouteError> {
    let tenant = &body.tenant_id;
    let config = managed_for_write(&claims, tenant).await?;
    let candidate =
        validated_candidate(&config, &body.roster_json, &body.expected_sha256)?;
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    let installed = managed::read(&tx, tenant).await.map_err(internal)?;
    if installed
        .as_ref()
        .is_some_and(|row| !valid_installed(&config, row))
    {
        return Err(internal("invalid installed trust"));
    }
    Ok(Json(RosterPreview {
        generation: installed.map_or(0, |row| row.generation).to_string(),
        candidate_sha256: candidate.sha256,
        approver_count: candidate.approver_count as i32,
        final_signer_count: candidate.final_signer_count as i32,
        rp_id: candidate.rp_id,
        origins: candidate.origins,
    }))
}

#[post("/activate-architect-signer-roster", format = "json", data = "<body>")]
pub async fn activate_architect_signer_roster(
    claims: JwtClaims,
    body: Json<ActivateInput>,
) -> Result<Json<RosterStatus>, RouteError> {
    let tenant = &body.tenant_id;
    let config = managed_for_write(&claims, tenant).await?;
    if !is_fresh_login(&claims, chrono::Utc::now().timestamp()) {
        return Err(fresh_auth_required());
    }
    let expected_generation = parse_generation(&body.generation)?;
    let candidate =
        validated_candidate(&config, &body.roster_json, &body.expected_sha256)?;
    let mut client = get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    // Hold both locks from validation of the active generation through the
    // trust and audit writes. Nothing can apply/import across a rotation.
    if !try_lock_package_work(&tx, tenant).await.map_err(internal)? {
        return Err(busy());
    }
    let old = managed::read(&tx, tenant).await.map_err(internal)?;
    if old
        .as_ref()
        .is_some_and(|row| !valid_installed(&config, row))
    {
        return Err(internal("invalid installed trust"));
    }
    if old.as_ref().map_or(0, |row| row.generation) != expected_generation {
        return Err(stale());
    }
    if old
        .as_ref()
        .is_some_and(|row| row.sha256 == candidate.sha256)
    {
        return Ok(Json(ready_status(&config, old.as_ref(), true)));
    }
    // Recheck current administrator membership and authentication freshness
    // after acquiring the application lock and immediately before writing.
    if !manager(&claims, tenant).await {
        return Err(forbidden_manager());
    }
    if !is_fresh_login(&claims, chrono::Utc::now().timestamp()) {
        return Err(fresh_auth_required());
    }
    let active = managed::install(
        &tx,
        &config,
        &candidate,
        expected_generation,
        &claims.sub,
    )
    .await
    .map_err(internal)?
    .ok_or_else(stale)?;
    let old_digest = old.as_ref().map(|row| json!(row.sha256));
    let new_digest = json!(active.sha256);
    let change = audit::change(
        &claims,
        audit::target(tenant, None)
            .map_err(|_| internal("invalid audit target"))?,
        AuditObject::new("architect_signer_roster", tenant, None),
        AdminAction::Update,
    )
    .with_changes(vec![FieldChange::new(
        "architect_signer_roster.sha256",
        old_digest.as_ref(),
        Some(&new_digest),
    )])
    .with_extra(json!({
        "generation": active.generation,
        "previous_generation": expected_generation,
        "fingerprint_confirmation": "operator_supplied",
    }))
    .with_description("Activated the receiver's Architect signer roster");
    audit_log::stage(&tx, &change).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    windmill::tasks::audit_log_outbox::kick_audit_log_outbox();
    Ok(Json(ready_status(&config, Some(&active), true)))
}

#[cfg(test)]
#[path = "../../tests/support/architect_signer_roster_tests.rs"]
mod tests;
