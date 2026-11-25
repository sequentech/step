// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Admission for setup on the normal Initialize request. The composition
//! supplies the protected local pin and the separate setup-only SQL pool.

use super::{caller, local::LocalInitializePost, prepare_initialize_signing, BrokerScope, ReturnPage};
use crate::services::dependencies::HarvestServices;
use crate::services::operational_portal_auth::OperationalPortalClaims;
use crate::services::signing_http::{SigningError, SigningErrorCode};
use deadpool_postgres::Pool;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

#[path = "local_officer_bridge.rs"]
pub(super) mod attachment;
#[path = "local_setup_ingress.rs"]
pub(super) mod ingress;
#[path = "local_setup_transport.rs"]
pub(super) mod transport;

const BINDING: &str = "ROW($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,$6::uuid,$7::uuid,$8::text)::felix3_initialize_setup.binding";
pub(super) const SETUP_CLIENT: &str = "felix3-local-initialize-setup";

fn refused() -> SigningError {
    SigningError::forbidden("Signing setup is unavailable for this request.")
}

// Database diagnostics can contain row values. Keep them out of responses
// and request telemetry for this private ceremony.
fn storage_error<E>(_: E) -> SigningError {
    SigningError::new(
        rocket::http::Status::ServiceUnavailable,
        SigningErrorCode::Internal,
        "Signing setup status is temporarily unavailable.",
    )
}

/// Repeat at each private handoff, including the Gold return. Only request
/// and reviewed digest come from the page. Actor and Post come from the
/// ordinary authorizer plus current realm membership and both Post labels.
pub(crate) async fn checked_scope(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    request: Uuid, reviewed_digest: &str,
) -> Result<BrokerScope, SigningError> {
    let (tenant, actor) = caller(&claims.0)?;
    if tenant != pin.tenant || pin.profile != "softhsm-test"
        || claims.0.iss != format!("http://localhost:18090/realms/tenant-{tenant}")
        || claims.0.azp == SETUP_CLIENT || pin.approval_client == SETUP_CLIENT
    {
        return Err(refused());
    }
    let mut db = services.databases.hasura().await.get().await.map_err(storage_error)?;
    let tx = db.transaction().await.map_err(storage_error)?;
    let current = prepare_initialize_signing(&tx, &actor, tenant, request).await.map_err(|_| refused())?;
    pin.verify(&tx, services, &actor, &current).await?;
    if current.payload_sha256() != reviewed_digest { return Err(refused()); }
    let scope = BrokerScope::from(&current);
    // Release app locks before the distinct SQL principal acquires them.
    tx.commit().await.map_err(storage_error)?;
    Ok(scope)
}

fn params<'a>(pin: &'a LocalInitializePost, scope: &'a BrokerScope) -> Vec<&'a (dyn ToSql + Sync)> {
    vec![&pin.installation, &pin.tenant, &pin.event, &pin.post, &pin.publication,
         &scope.request, &scope.official, &scope.payload_sha256]
}

/// One statement observes the phase and the session's original reference.
/// 2200 owns the phase, including whether a released attempt is still unclaimed.
async fn status(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    pool: &Pool, request: Uuid, reviewed_digest: &str, session_digest: &str,
) -> Result<ingress::Snapshot, SigningError> {
    let scope = checked_scope(services, pin, claims, request, reviewed_digest).await?;
    let db = pool.get().await.map_err(storage_error)?;
    let sql = format!("SELECT felix3_initialize_setup.status({BINDING}), a.attempt, a.path, a.panel, a.revision
        FROM (VALUES (1)) AS one(n) LEFT JOIN felix3_initialize_setup.browser_attempt({BINDING},$9) a ON true");
    let mut args = params(pin, &scope);
    args.push(&session_digest);
    let row = db.query_one(sql.as_str(), &args).await.map_err(storage_error)?;
    ingress::Snapshot::from_row(scope, pin, &row)
}

/// Call after an explicit user click and the private setup broker's nonce
/// commitment. A successful write allows only the dedicated Gold ceremony.
pub(crate) async fn begin(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    pool: &Pool, request: Uuid, reviewed_digest: &str, attempt: Uuid, broker_nonce_hash: &str,
    session_digest: &str, page: ReturnPage,
) -> Result<BrokerScope, SigningError> {
    let scope = checked_scope(services, pin, claims, request, reviewed_digest).await?;
    if !page.matches_request(Some(pin.post)) { return Err(refused()); }
    let mut db = pool.get().await.map_err(storage_error)?;
    let tx = db.transaction().await.map_err(storage_error)?;
    let sql = format!("SELECT felix3_initialize_setup.begin_setup({BINDING},$9,$10)");
    let mut args = params(pin, &scope);
    args.extend_from_slice(&[&attempt, &broker_nonce_hash]);
    let saved: bool = tx.query_one(sql.as_str(), &args).await.and_then(|row| row.try_get(0))
        .map_err(storage_error)?;
    if !saved { return Err(refused()); }
    let sql = format!("SELECT felix3_initialize_setup.bind_browser({BINDING},$9,$10,$11,$12)");
    let mut args = params(pin, &scope);
    let path = page.path.as_str();
    let panel = page.panel.as_str();
    args.extend_from_slice(&[&attempt, &session_digest, &path, &panel]);
    let bound: bool = tx.query_one(sql.as_str(), &args).await.and_then(|row| row.try_get(0))
        .map_err(storage_error)?;
    if !bound { return Err(refused()); }
    tx.commit().await.map_err(storage_error)?;
    Ok(scope)
}

/// Constructed only after 2200 accepts release on the original attempt.
pub(crate) struct ReleasedAttempt { scope: BrokerScope, attempt: Uuid }

/// The final registration click (or explicit redelivery of the same unclaimed
/// attempt). Returning from Keycloak alone cannot release a key generation.
pub(crate) async fn release(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    pool: &Pool, request: Uuid, reviewed_digest: &str, attempt: Uuid,
    session_digest: &str, reviewed_revision: i32,
) -> Result<ReleasedAttempt, SigningError> {
    let scope = checked_scope(services, pin, claims, request, reviewed_digest).await?;
    let mut db = pool.get().await.map_err(storage_error)?;
    let tx = db.transaction().await.map_err(storage_error)?;
    let sql = format!("SELECT felix3_initialize_setup.release({BINDING},$9)");
    let mut args = params(pin, &scope);
    args.push(&attempt);
    let released: bool = tx.query_one(sql.as_str(), &args).await.and_then(|row| row.try_get(0))
        .map_err(storage_error)?;
    if !released { return Err(refused()); }
    let sql = format!("SELECT felix3_initialize_setup.take_delivery({BINDING},$9,$10,$11)");
    args.extend_from_slice(&[&session_digest, &reviewed_revision]);
    let taken: bool = tx.query_one(sql.as_str(), &args).await.and_then(|row| row.try_get(0))
        .map_err(storage_error)?;
    if !taken { return Err(refused()); }
    tx.commit().await.map_err(storage_error)?;
    Ok(ReleasedAttempt { scope, attempt })
}
