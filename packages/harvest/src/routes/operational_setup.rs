// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Unmounted read-only per-official setup. No caller-supplied subject, device
//! configuration, enrollment attributes, or certificates. An attached staff
//! certificate is not current CA approval, device liveness, or permission to sign.

use super::signing::SigningReply;
use crate::services::dependencies::HarvestServices;
use crate::services::operational_portal_auth::OperationalPortalClaims;
use crate::services::signing_http::{authorize_403, SigningError};
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetupStatusInput {
    election_event_id: Uuid,
}

#[derive(Serialize)]
pub(crate) struct SetupStatus {
    tenant: Uuid,
    event: Uuid,
    official: Uuid,
    phase: Phase,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum Phase {
    KeyNotEnrolled,
    CertificateNotAttached,
    CertificateOnFile,
    CertificateNeedsReview,
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/setup/status",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn status(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    input: Json<SetupStatusInput>,
) -> SigningReply<SetupStatus> {
    let claims = claims.0;
    authorize_403(
        &claims,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::SIGNING_CERTIFICATES_READ],
    )?;
    let scope = || {
        SigningError::forbidden(
            "Signing setup is not available for this event.",
        )
    };
    let tenant = Uuid::parse_str(&claims.hasura_claims.tenant_id)
        .map_err(|_| scope())?;
    let official =
        Uuid::parse_str(&claims.hasura_claims.user_id).map_err(|_| scope())?;
    let event = input.election_event_id;
    if tenant.is_nil() || official.is_nil() || event.is_nil() {
        return Err(scope());
    }
    let db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let actor = official.to_string();
    let phase = db.query_one(
        "SELECT sequent_backend.operational_official_setup_status($1,$2,$3)",
        &[&tenant, &event, &actor],
    ).await.map_err(SigningError::internal)?
        .try_get::<_, Option<String>>(0).map_err(SigningError::internal)?;
    let phase = match phase.as_deref() {
        Some("key-not-enrolled") => Phase::KeyNotEnrolled,
        Some("certificate-not-attached") => Phase::CertificateNotAttached,
        Some("certificate-on-file") => Phase::CertificateOnFile,
        Some("certificate-needs-review") => Phase::CertificateNeedsReview,
        _ => return Err(scope()),
    };
    Ok(Json(SetupStatus {
        tenant,
        event,
        official,
        phase,
    }))
}

#[cfg(feature = "operational-signing-product")]
pub(crate) mod product {
use super::*;

pub(crate) struct ProductSetupInstallations {
    tenants: std::collections::HashMap<Uuid, (ProductSetupActorPool, ProductSetupDispatcher)>,
}

impl ProductSetupInstallations {
    pub(crate) fn new(
        tenants: std::collections::HashMap<Uuid, (ProductSetupActorPool, ProductSetupDispatcher)>,
    ) -> Option<Self> {
        (!tenants.is_empty() && tenants.iter().all(|(tenant, (_, dispatcher))|
            !tenant.is_nil() && *tenant == dispatcher.tenant))
            .then_some(Self { tenants })
    }

    pub(crate) fn serves(&self, tenant: Uuid) -> bool {
        self.tenants.contains_key(&tenant)
    }

    fn tenant(&self, tenant: Uuid) -> Option<&(ProductSetupActorPool, ProductSetupDispatcher)> {
        self.tenants.get(&tenant)
    }
}

/// Dedicated application reader/actor endpoint pool, not the broad normal
/// Hasura pool, dispatch, device, CA, approval or ledger owner.
pub(crate) struct ProductSetupActorPool(pub(crate) deadpool_postgres::Pool);

impl ProductSetupActorPool {
    pub(crate) async fn new(pool: deadpool_postgres::Pool) -> Option<Self> {
        let db = pool.get().await.ok()?;
        let role_ok: bool = db.query_one(
            "SELECT current_user=session_user
             AND pg_has_role(current_user,'sequent_os_product_setup_actor','USAGE')
             AND pg_has_role(current_user,'sequent_os_mode_reader','USAGE')
             AND NOT pg_has_role(current_user,'sequent_os_product_setup_dispatch','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_product_setup_device','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_setup_authority','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_product_setup_ca','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_approve','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER')
             AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_event_setup_request','SELECT,INSERT,UPDATE,REFERENCES')
             AND NOT has_any_column_privilege(current_user,'sequent_backend.signing_request','INSERT,UPDATE,REFERENCES')",
            &[],
        ).await.ok()?.try_get(0).ok()?;
        drop(db);
        role_ok.then_some(Self(pool))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnrollmentInput {
    request_id: Uuid,
    reviewed_payload_sha256: String,
}

#[derive(Serialize)]
pub(crate) struct EnrollmentReply {
    ticket: Uuid,
    tenant: Uuid,
    event: Uuid,
    request: Uuid,
    official: Uuid,
    phase: windmill::services::signing::operational_enrollment::EnrollmentPhase,
}

/// Mount only with the independent operational setup officer, genuine
/// hardware worker and private staff CA workflows configured.
#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/v2/<tenant>/setup",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn request_enrollment(
    tenant: &str,
    bearer: crate::routes::operational_product_approval::ProductBearer,
    installed: &State<crate::routes::operational_product_approval::ProductApprovalInstallation>,
    setups: &State<ProductSetupInstallations>,
    input: Json<EnrollmentInput>,
) -> SigningReply<EnrollmentReply> {
    use windmill::services::signing::{operational_enrollment, SigningCaller};

    let unavailable = || SigningError::forbidden("Signing setup is unavailable for this request.");
    let (tenant, claims, _) = crate::routes::operational_product_approval::authenticate_claims(
        installed, tenant, &bearer,
    )?;
    let (actor_pool, _) = setups.tenant(tenant).ok_or_else(unavailable)?;
    let mut db = actor_pool.0.get().await.map_err(SigningError::internal)?;
    let tx = db.transaction().await.map_err(SigningError::internal)?;
    let requested = operational_enrollment::request_enrollment(
        &tx, &SigningCaller::from_claims(&claims), tenant, input.request_id,
        &input.reviewed_payload_sha256, &claims.acr, claims.auth_time,
    ).await.map_err(|error| match error {
        windmill::services::signing::SigningError::Internal(error) => {
            tracing::error!(?error, "hardware setup authorization failed");
            SigningError::internal(error)
        }
        _ => {
            tracing::info!(tenant = %tenant, "hardware setup authorization refused");
            unavailable()
        }
    })?;
    // The narrow issuance function atomically retains scoped authorization,
    // authenticated actor, Gold time and review digest, including on network loss.
    tx.commit().await.map_err(SigningError::internal)?;
    Ok(Json(EnrollmentReply {
        ticket: requested.ticket,
        tenant,
        event: requested.event,
        request: input.request_id,
        official: requested.official,
        phase: requested.phase,
    }))
}

/// Independently provisioned product officer ingress. All values are owned by
/// operators and must be loaded from protected installation configuration,
/// never from a certificate, URL, principal or credential supplied by a user.
pub(crate) struct ProductSetupDispatcher {
    tenant: Uuid,
    database: deadpool_postgres::Pool,
    endpoint: reqwest::Url,
    client: reqwest::Client,
}

impl ProductSetupDispatcher {
    pub(crate) async fn new(
        tenant: Uuid,
        database: deadpool_postgres::Pool,
        endpoint: &str,
        worker_tls_ca: reqwest::Certificate,
        independent_officer_identity: reqwest::Identity,
    ) -> Option<Self> {
        let endpoint = reqwest::Url::parse(endpoint).ok()?;
        if tenant.is_nil() || endpoint.scheme() != "https" || endpoint.host().is_none()
            || !endpoint.username().is_empty() || endpoint.password().is_some()
            || endpoint.path() != "/private/v2/operational-signing/setup/generate"
            || endpoint.query().is_some() || endpoint.fragment().is_some()
        {
            return None;
        }
        let db = database.get().await.ok()?;
        let role_ok: bool = db.query_one(
            "SELECT current_user=session_user
             AND pg_has_role(current_user,'sequent_os_product_setup_dispatch','USAGE')
             AND NOT pg_has_role(current_user,'sequent_os_product_setup_device','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_product_setup_actor','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_setup_authority','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_product_setup_ca','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_approve','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER')
             AND NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER')",
            &[],
        ).await.ok()?.try_get(0).ok()?;
        if !role_ok {
            return None;
        }
        drop(db);
        let client = reqwest::Client::builder()
            .https_only(true)
            .tls_built_in_root_certs(false)
            .add_root_certificate(worker_tls_ca)
            .identity(independent_officer_identity)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(std::time::Duration::from_secs(8))
            .build().ok()?;
        Some(Self { tenant, database, endpoint, client })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetupDispatchInput {
    ticket: Uuid,
    event: Uuid,
}

#[derive(Serialize)]
pub(crate) struct SetupDispatchReply {
    attempt: Uuid,
    phase: &'static str,
}

/// Explicit independent officer action after the regular actor has requested
/// setup. This endpoint cannot sign an action, choose the device or attach
/// any certificate. Do not mount without the private worker ACLs and mTLS.
#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/v2/<tenant>/setup/dispatch",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn dispatch_enrollment(
    tenant: &str,
    bearer: crate::routes::operational_product_approval::ProductBearer,
    installed: &State<crate::routes::operational_product_approval::ProductApprovalInstallation>,
    setups: &State<ProductSetupInstallations>,
    input: Json<SetupDispatchInput>,
) -> SigningReply<SetupDispatchReply> {
    use windmill::services::signing::{operational_enrollment, SigningCaller};

    let (tenant, claims, _) = crate::routes::operational_product_approval::authenticate_claims(
        installed, tenant, &bearer,
    )?;
    let refuse = || SigningError::forbidden("Hardware setup dispatch unavailable.");
    let (_, dispatcher) = setups.tenant(tenant).ok_or_else(refuse)?;
    if input.ticket.is_nil() || input.event.is_nil() {
        return Err(refuse());
    }
    let mut db = dispatcher.database.get().await.map_err(SigningError::internal)?;
    let tx = db.transaction().await.map_err(SigningError::internal)?;
    let official = SigningCaller::from_claims(&claims);
    let delivery = operational_enrollment::prepare_setup_dispatch(
        &tx, &official, tenant, input.event, input.ticket, &claims.acr, claims.auth_time,
    ).await.map_err(|error| match error {
        windmill::services::signing::SigningError::Internal(error) => SigningError::internal(error),
        _ => refuse(),
    })?;
    // The narrow SQL operation durably records the officer, auth time,
    // actor, ticket, exact scope and attempt before contacting the worker.
    tx.commit().await.map_err(SigningError::internal)?;
    // An uncertain reply never repeats the hardware action.
    let result = dispatcher.client.post(dispatcher.endpoint.clone()).json(&delivery).send().await;
    let phase = match result {
        Ok(reply) if reply.status() == reqwest::StatusCode::NO_CONTENT =>
            "hardware-generated-certificate-pending",
        _ => "reconciliation-required",
    };
    Ok(Json(SetupDispatchReply { attempt: delivery.attempt, phase }))
}


/// Never redispatch automatically. The independent officer may release an
/// expired non-claim and the actor must explicitly ask again afterward.
#[tracing::instrument(skip_all)]
#[rocket::post("/operational-signing/v2/<tenant>/setup/reconcile-unclaimed", format="json", data="<input>")]
pub(crate) async fn reconcile_unclaimed(
    tenant: &str,
    bearer: crate::routes::operational_product_approval::ProductBearer,
    installed: &State<crate::routes::operational_product_approval::ProductApprovalInstallation>,
    setups: &State<ProductSetupInstallations>,
    input: Json<SetupDispatchInput>,
) -> Result<rocket::http::Status, SigningError> {
    use windmill::services::signing::{operational_enrollment, SigningCaller};
    let (tenant, claims, _) = crate::routes::operational_product_approval::authenticate_claims(
        installed, tenant, &bearer,
    )?;
    let refuse = || SigningError::forbidden("No expired, never-claimed hardware setup can be released.");
    let (_, dispatcher) = setups.tenant(tenant).ok_or_else(refuse)?;
    let mut db = dispatcher.database.get().await.map_err(SigningError::internal)?;
    let tx = db.transaction().await.map_err(SigningError::internal)?;
    operational_enrollment::reconcile_unclaimed_setup(
        &tx, &SigningCaller::from_claims(&claims), tenant, input.event, input.ticket,
        &claims.acr, claims.auth_time,
    ).await.map_err(|error| match error {
        windmill::services::signing::SigningError::Internal(error) => SigningError::internal(error),
        _ => refuse(),
    })?;
    tx.commit().await.map_err(SigningError::internal)?;
    Ok(rocket::http::Status::NoContent)
}

}
