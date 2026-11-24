// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Admission for setup on the normal Initialize request. The composition
//! supplies the protected local pin and the separate setup-only SQL pool.

use super::{caller, local::LocalInitializePost, prepare_initialize_signing, BrokerScope};
use crate::services::dependencies::HarvestServices;
use crate::services::operational_portal_auth::OperationalPortalClaims;
use crate::services::signing_http::SigningError;
use deadpool_postgres::Pool;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

const BINDING: &str = "ROW($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,$6::uuid,$7::uuid,$8::text)::felix3_initialize_setup.binding";

fn refused() -> SigningError {
    SigningError::forbidden("Signing setup is unavailable for this request.")
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
        || claims.0.azp == "felix3-local-initialize-setup"
        || pin.approval_client == "felix3-local-initialize-setup"
    {
        return Err(refused());
    }
    let mut db = services.databases.hasura().await.get().await.map_err(SigningError::internal)?;
    let tx = db.transaction().await.map_err(SigningError::internal)?;
    let current = prepare_initialize_signing(&tx, &actor, tenant, request).await.map_err(|_| refused())?;
    pin.verify(&tx, services, &actor, &current).await?;
    if current.payload_sha256() != reviewed_digest { return Err(refused()); }
    let scope = BrokerScope::from(&current);
    // Release app locks before the distinct SQL principal acquires them.
    tx.commit().await.map_err(SigningError::internal)?;
    Ok(scope)
}

fn params<'a>(pin: &'a LocalInitializePost, scope: &'a BrokerScope) -> Vec<&'a (dyn ToSql + Sync)> {
    vec![&pin.installation, &pin.tenant, &pin.event, &pin.post, &pin.publication,
         &scope.request, &scope.official, &scope.payload_sha256]
}

pub(crate) async fn status(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    pool: &Pool, request: Uuid, reviewed_digest: &str,
) -> Result<String, SigningError> {
    let scope = checked_scope(services, pin, claims, request, reviewed_digest).await?;
    let db = pool.get().await.map_err(SigningError::internal)?;
    let sql = format!("SELECT felix3_initialize_setup.status({BINDING})");
    db.query_one(sql.as_str(), &params(pin, &scope)).await.and_then(|row| row.try_get(0))
        .map_err(SigningError::internal)
}

/// Call after an explicit user click and the private setup broker's nonce
/// commitment. A successful write allows only the dedicated Gold ceremony.
pub(crate) async fn begin(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    pool: &Pool, request: Uuid, reviewed_digest: &str, attempt: Uuid, broker_nonce_hash: &str,
) -> Result<BrokerScope, SigningError> {
    let scope = checked_scope(services, pin, claims, request, reviewed_digest).await?;
    let db = pool.get().await.map_err(SigningError::internal)?;
    let sql = format!("SELECT felix3_initialize_setup.begin_setup({BINDING},$9,$10)");
    let mut args = params(pin, &scope);
    args.extend_from_slice(&[&attempt, &broker_nonce_hash]);
    let saved: bool = db.query_one(sql.as_str(), &args).await.and_then(|row| row.try_get(0))
        .map_err(SigningError::internal)?;
    if saved { Ok(scope) } else { Err(refused()) }
}

/// The final registration click (or explicit redelivery of the same unclaimed
/// attempt). Returning from Keycloak alone cannot release a key generation.
pub(crate) async fn release(
    services: &HarvestServices, pin: &LocalInitializePost, claims: &OperationalPortalClaims,
    pool: &Pool, request: Uuid, reviewed_digest: &str, attempt: Uuid,
) -> Result<BrokerScope, SigningError> {
    let scope = checked_scope(services, pin, claims, request, reviewed_digest).await?;
    let db = pool.get().await.map_err(SigningError::internal)?;
    let sql = format!("SELECT felix3_initialize_setup.release({BINDING},$9)");
    let mut args = params(pin, &scope);
    args.push(&attempt);
    let released: bool = db.query_one(sql.as_str(), &args).await.and_then(|row| row.try_get(0))
        .map_err(SigningError::internal)?;
    if released { Ok(scope) } else { Err(refused()) }
}
