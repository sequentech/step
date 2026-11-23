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
