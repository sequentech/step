// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Own, first-use key setup for the exact approved Initialize Post request.
//! Device work neither attaches a staff certificate nor approves the request.

use super::signing::SigningReply;
use crate::services::{
    audit,
    dependencies::HarvestServices,
    operational_portal_auth::OperationalPortalClaims,
    signing_http::{authorize_403, SigningError},
};
use chrono::{DateTime, Utc};
use reqwest::{Certificate, Client, Identity, Url};
use rocket::{serde::json::Json, State};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;
use windmill::{
    postgres::audit_log_outbox::AuditLogTarget,
    services::{
        audit_log::{self, AdminAction, AuditObject},
        signing::{
            operational_initialize::prepare_initialize_signing, SigningCaller,
        },
    },
    tasks::audit_log_outbox::kick_audit_log_outbox,
};

pub(crate) struct SetupDispatcher {
    tenant: Uuid,
    event: Uuid,
    endpoint: Url,
    client: Client,
}

impl SetupDispatcher {
    pub(crate) fn new(
        tenant: Uuid,
        event: Uuid,
        device_origin: &str,
        root: Certificate,
        identity: Identity,
    ) -> Option<Self> {
        let origin = Url::parse(device_origin).ok()?;
        if tenant.is_nil()
            || event.is_nil()
            || origin.scheme() != "https"
            || origin.host_str().is_none()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !origin.username().is_empty()
            || origin.password().is_some()
        {
            return None;
        }
        Some(Self {
            tenant,
            event,
            endpoint: origin
                .join("/private/v1/operational-signing/setup/generate")
                .ok()?,
            client: Client::builder()
                .use_rustls_tls()
                .tls_built_in_root_certs(false)
                .add_root_certificate(root)
                .identity(identity)
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .https_only(true)
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(10))
                .build()
                .ok()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    request_id: Uuid,
}

#[derive(Clone, Copy, Serialize)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
    request: Uuid,
    official: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum Phase {
    SetupAvailable,
    SetupRetryAvailable,
    SetupPending,
    SetupRecoveryRequired,
    CertificateOnFile,
    Unavailable,
}

#[derive(Serialize)]
pub(crate) struct Reply {
    scope: Scope,
    phase: Phase,
}

fn fresh_gold(
    acr: &str,
    auth_time: Option<i64>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let time = DateTime::from_timestamp(auth_time?, 0)?;
    (acr == Permissions::GOLD.to_string()
        && time <= now
        && now.signed_duration_since(time) < chrono::Duration::seconds(60))
    .then_some(time)
}

fn unavailable() -> SigningError {
    SigningError::forbidden("Signing setup is unavailable for this request.")
}

#[rocket::post(
    "/operational-signing/initialize/setup/status",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn status(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    dispatcher: &State<SetupDispatcher>,
    input: Json<Input>,
) -> SigningReply<Reply> {
    setup(claims, services, dispatcher, input.into_inner(), false).await
}

#[rocket::post(
    "/operational-signing/initialize/setup",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn start(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    dispatcher: &State<SetupDispatcher>,
    input: Json<Input>,
) -> SigningReply<Reply> {
    setup(claims, services, dispatcher, input.into_inner(), true).await
}

async fn setup(
    OperationalPortalClaims(claims): OperationalPortalClaims,
    services: &HarvestServices,
    dispatcher: &SetupDispatcher,
    input: Input,
    dispatch: bool,
) -> SigningReply<Reply> {
    let tenant = dispatcher.tenant;
    authorize_403(
        &claims,
        Some(tenant.to_string()),
        vec![Permissions::SIGN_INITIALIZE_VOTING],
    )?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let caller = SigningCaller::from_claims(&claims);
    let request =
        prepare_initialize_signing(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(|_| unavailable())?;
    let post = request
        .election()
        .filter(|post| !post.is_nil())
        .ok_or_else(unavailable)?;
    let scope = Scope {
        tenant,
        event: request.event(),
        request: request.request(),
        official: request.actor(),
    };
    if scope.event != dispatcher.event {
        return Err(unavailable());
    }
    let hash = request.payload_sha256();
    let phase = scoped_phase(&tx, scope, post, hash).await?;
    if !dispatch
        || !matches!(phase, Phase::SetupAvailable | Phase::SetupRetryAvailable)
    {
        return Ok(Json(Reply { scope, phase }));
    }
    let auth_time = fresh_gold(&claims.acr, claims.auth_time, Utc::now())
        .ok_or_else(|| {
            SigningError::forbidden("Sign in again to set up signing.")
                .with("gold_required", true)
        })?;
    let initial = matches!(phase, Phase::SetupAvailable);
    let attempt: Option<Uuid> = if initial {
        let attempt = Uuid::new_v4();
        let reserved: bool = tx.query_one(
            "SELECT sequent_backend.operational_initialize_start_self_setup($1,$2,$3,$4,$5,$6,$7,$8)",
            &[&tenant, &scope.event, &scope.request, &post, &scope.official, &hash, &auth_time, &attempt],
        ).await.map_err(SigningError::internal)?.try_get(0).map_err(SigningError::internal)?;
        reserved.then_some(attempt)
    } else {
        tx.query_one(
            "SELECT sequent_backend.operational_initialize_redispatch_self_setup($1,$2,$3,$4,$5,$6,$7)",
            &[&tenant, &scope.event, &scope.request, &post, &scope.official, &hash, &auth_time],
        ).await.map_err(SigningError::internal)?.try_get(0).map_err(SigningError::internal)?
    };
    let Some(attempt) = attempt else {
        let phase = scoped_phase(&tx, scope, post, hash).await?;
        return Ok(Json(Reply { scope, phase }));
    };
    let entry = audit::change(
        &claims,
        AuditLogTarget {
            tenant_id: tenant,
            election_event_id: Some(scope.event),
        },
        AuditObject::new("operational_initialize_self_setup", attempt, None),
        if initial {
            AdminAction::Create
        } else {
            AdminAction::Update
        },
    )
    .with_extra(
        json!({"signing_request_id": scope.request, "election_id": post}),
    )
    .with_description(if initial {
        "Requested first-use signing setup"
    } else {
        "Redelivered an unclaimed signing setup attempt"
    });
    audit_log::stage(&tx, &entry)
        .await
        .map_err(SigningError::internal)?;
    tx.commit().await.map_err(SigningError::internal)?;
    kick_audit_log_outbox();

    // An HTTP result is not evidence of a claim, a device outcome, or an
    // attached certificate. Never auto-retry; reread only the durable ledger.
    let delivered = dispatcher
        .client
        .post(dispatcher.endpoint.clone())
        .json(&json!({
            "tenant": tenant,
            "event": scope.event,
            "official": scope.official,
            "attempt": attempt,
        }))
        .send()
        .await;
    if !delivered.is_ok_and(|response| {
        response.status() == reqwest::StatusCode::NO_CONTENT
    }) {
        tracing::warn!("Operational setup delivery not confirmed; reconciling durable state");
    }
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let phase = scoped_phase(&tx, scope, post, hash).await?;
    Ok(Json(Reply { scope, phase }))
}

async fn scoped_phase(
    tx: &deadpool_postgres::Transaction<'_>,
    scope: Scope,
    post: Uuid,
    hash: &str,
) -> Result<Phase, SigningError> {
    let result: Option<String> = tx.query_one(
        "SELECT sequent_backend.operational_initialize_scoped_setup_status($1,$2,$3,$4,$5,$6)",
        &[&scope.tenant, &scope.event, &scope.request, &post, &scope.official, &hash],
    ).await.map_err(SigningError::internal)?.try_get(0).map_err(SigningError::internal)?;
    Ok(match result.as_deref() {
        Some("setup-available") => Phase::SetupAvailable,
        Some("setup-retry-available") => Phase::SetupRetryAvailable,
        Some("setup-pending") => Phase::SetupPending,
        Some("setup-recovery-required") => Phase::SetupRecoveryRequired,
        Some("certificate-on-file") => Phase::CertificateOnFile,
        _ => Phase::Unavailable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_actual_recent_gold_authentication() {
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        let t = now.timestamp();
        assert!(fresh_gold("gold", Some(t - 59), now).is_some());
        assert!(fresh_gold("gold", Some(t), now).is_some());
        assert!(fresh_gold("gold", Some(t + 1), now).is_none());
        assert!(fresh_gold("gold", Some(t - 60), now).is_none());
        assert!(fresh_gold("gold", None, now).is_none());
        assert!(fresh_gold("1", Some(t), now).is_none());
    }
}
