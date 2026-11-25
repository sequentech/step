// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{PortalScope, ReturnPage, SigningReply};
use super::transport::{Delivery, SetupServices};
use super::{
    begin, checked_scope, refused, release, storage_error, BrokerScope, HarvestServices, LocalInitializePost,
    OperationalPortalClaims, SigningError,
};
use crate::services::audit;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{serde::json::Json, Request, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use windmill::postgres::audit_log_outbox::AuditLogTarget;
use windmill::services::audit_log::{self, AdminAction, AuditObject};
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

pub(crate) struct SetupSession {
    claims: OperationalPortalClaims,
    digest: String,
}

#[derive(Deserialize)]
struct SessionId {
    sid: Option<Uuid>,
    session_state: Option<Uuid>,
}

impl SessionId {
    fn id(self) -> Option<Uuid> {
        let id = self.sid.or(self.session_state)?;
        (!id.is_nil() && self.session_state.is_none_or(|other| other == id)).then_some(id)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SetupSession {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        // This guard verifies the actual ingress peer and the signature, issuer,
        // client, audience and actor of the single immutable Authorization header.
        let claims = match OperationalPortalClaims::from_request(request).await {
            Outcome::Success(claims) => claims,
            Outcome::Error(error) => return Outcome::Error(error),
            Outcome::Forward(status) => return Outcome::Forward(status),
        };
        let session = request.headers().get_one("Authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .and_then(|jwt| jwt.split('.').nth(1))
            .and_then(|body| URL_SAFE_NO_PAD.decode(body).ok())
            .and_then(|bytes| serde_json::from_slice::<SessionId>(&bytes).ok())
            .and_then(SessionId::id);
        let Some(id) = session else {
            return Outcome::Error((Status::Forbidden, ()));
        };
        let mut hash = Sha256::new();
        hash.update(b"felix3-local-initialize/setup/portal-session/v1\0");
        hash.update(id.as_bytes());
        Outcome::Success(Self { claims, digest: hex::encode(hash.finalize()) })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Phase {
    SetupAvailable,
    WaitingForSetupGold,
    SetupConfirmationRequired,
    SetupRetryAvailable,
    SetupPending,
    SetupRecoveryRequired,
    CertificateOnFile,
    Unavailable,
}

#[derive(Clone, Copy)]
struct Owned {
    attempt: Uuid,
    page: ReturnPage,
    revision: i32,
}

pub(super) struct Snapshot {
    scope: BrokerScope,
    device_profile: &'static str,
    phase: Phase,
    owned: Option<Owned>,
}

impl Snapshot {
    pub(super) fn from_row(
        scope: BrokerScope, pin: &LocalInitializePost, row: &tokio_postgres::Row,
    ) -> Result<Self, SigningError> {
        let phase: &str = row.try_get(0).map_err(storage_error)?;
        let mut phase: Phase = serde_json::from_value(json!(phase)).map_err(|_| refused())?;
        let id: Option<Uuid> = row.try_get(1).map_err(storage_error)?;
        let owned = if let Some(attempt) = id {
            let path: &str = row.try_get(2).map_err(storage_error)?;
            let panel: &str = row.try_get(3).map_err(storage_error)?;
            let page: ReturnPage = serde_json::from_value(json!({
                "path": path,
                "panel": panel,
                "election_id": path.starts_with("election-").then_some(pin.post),
            })).map_err(|_| refused())?;
            if attempt.is_nil() || !page.matches_request(Some(pin.post)) {
                return Err(refused());
            }
            let revision = row.try_get(4).map_err(storage_error)?;
            Some(Owned { attempt, page, revision })
        } else {
            None
        };
        if owned.is_none() && matches!(phase,
            Phase::WaitingForSetupGold | Phase::SetupConfirmationRequired | Phase::SetupRetryAvailable)
        {
            phase = Phase::Unavailable;
        }
        Ok(Self { scope, device_profile: pin.profile, phase, owned })
    }

    fn original(&self, expected: Uuid) -> Result<Owned, SigningError> {
        self.owned.filter(|o| o.attempt == expected).ok_or_else(refused)
    }

    fn view(&self) -> View {
        let scope = PortalScope {
            tenant: self.scope.tenant,
            event: self.scope.event,
            request: self.scope.request,
            official: self.scope.official,
        };
        let (attempt, delivery_revision) = match (self.phase, self.owned) {
            (Phase::WaitingForSetupGold, Some(original)) => (Some(original.attempt), None),
            (Phase::SetupConfirmationRequired | Phase::SetupRetryAvailable, Some(original)) => {
                (Some(original.attempt), Some(original.revision))
            }
            _ => (None, None),
        };
        View { scope, device_profile: self.device_profile, phase: self.phase, attempt, delivery_revision }
    }
}

#[derive(Serialize)]
pub(crate) struct View {
    scope: PortalScope,
    device_profile: &'static str,
    phase: Phase,
    attempt: Option<Uuid>,
    delivery_revision: Option<i32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Review {
    request_id: Uuid,
    payload_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Start {
    request_id: Uuid,
    payload_sha256: String,
    page: ReturnPage,
    attempt: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Callback {
    request_id: Uuid,
    payload_sha256: String,
    attempt: Uuid,
    state: String,
    code: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Register {
    request_id: Uuid,
    payload_sha256: String,
    attempt: Uuid,
    delivery_revision: i32,
}

#[derive(Serialize)]
pub(crate) struct Redirect {
    attempt: Uuid,
    redirect: String,
    page: ReturnPage,
}

#[derive(Serialize)]
pub(crate) struct Confirmed {
    #[serde(flatten)]
    scope: PortalScope,
    attempt: Uuid,
    page: ReturnPage,
    phase: Phase,
}

// The event outbox and the setup ledger have different least-privilege logins.
// Record only an authorized intent, before the private operation. No proof,
// nonce, session digest, redirect or device response goes in the event log.
async fn record_intent(
    services: &HarvestServices, session: &SetupSession, scope: &BrokerScope,
    attempt: Uuid, action: AdminAction, description: &'static str,
) -> Result<(), SigningError> {
    let entry = audit::change(
        &session.claims.0,
        AuditLogTarget { tenant_id: scope.tenant, election_event_id: Some(scope.event) },
        AuditObject::new("local_initialize_self_setup", attempt, None),
        action,
    )
    .with_extra(json!({"signing_request_id": scope.request}))
    .with_description(description);
    let mut db = services.databases.hasura().await.get().await.map_err(storage_error)?;
    let tx = db.transaction().await.map_err(storage_error)?;
    audit_log::stage(&tx, &entry).await.map_err(storage_error)?;
    tx.commit().await.map_err(storage_error)?;
    kick_audit_log_outbox();
    Ok(())
}

#[rocket::post("/operational-signing/initialize/self-setup/status", format = "json", data = "<input>")]
pub(crate) async fn status(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    input: Json<Review>,
) -> SigningReply<View> {
    let current = super::status(app, pin, &session.claims, &setup.pool, input.request_id,
        &input.payload_sha256, &session.digest).await?;
    Ok(Json(current.view()))
}

#[rocket::post("/operational-signing/initialize/self-setup/start", format = "json", data = "<input>")]
pub(crate) async fn start(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    input: Json<Start>,
) -> SigningReply<Redirect> {
    let request = input.request_id;
    let digest = &input.payload_sha256;
    let current = super::status(app, pin, &session.claims, &setup.pool, request, digest, &session.digest).await?;
    if !input.page.matches_request(Some(pin.post)) {
        return Err(refused());
    }
    let owned = match (current.phase, input.attempt) {
        (Phase::SetupAvailable, None) => {
            let attempt = Uuid::new_v4();
            record_intent(app, &session, &current.scope, attempt, AdminAction::Create,
                "Requested Initialize signing key setup").await?;
            let scope = checked_scope(app, pin, &session.claims, request, digest).await?;
            let nonce = setup.gold.nonce(&Delivery::new(pin, &scope, attempt)).await?;
            begin(app, pin, &session.claims, &setup.pool, request, digest, attempt,
                &nonce, &session.digest, input.page).await?;
            Owned { attempt, page: input.page, revision: 0 }
        }
        (Phase::WaitingForSetupGold, Some(expected)) => current.original(expected)?,
        _ => return Err(refused()),
    };
    let scope = checked_scope(app, pin, &session.claims, request, digest).await?;
    let redirect = setup.gold.par(&Delivery::new(pin, &scope, owned.attempt), &pin.authorization).await?;
    Ok(Json(Redirect { attempt: owned.attempt, redirect, page: owned.page }))
}

#[rocket::post("/operational-signing/initialize/self-setup/confirm", format = "json", data = "<input>")]
pub(crate) async fn confirm(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    input: Json<Callback>,
) -> SigningReply<Confirmed> {
    let request = input.request_id;
    let digest = &input.payload_sha256;
    let before = super::status(app, pin, &session.claims, &setup.pool, request, digest, &session.digest).await?;
    let owned = before.original(input.attempt)?;
    if before.phase != Phase::WaitingForSetupGold
        || input.state.len() != 43
        || !input.state.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || !(1..=4096).contains(&input.code.len())
        || !input.code.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(refused());
    }
    record_intent(app, &session, &before.scope, owned.attempt, AdminAction::Update,
        "Returned from Initialize setup identity verification").await?;
    let scope = checked_scope(app, pin, &session.claims, request, digest).await?;
    setup.gold.confirm(&Delivery::new(pin, &scope, owned.attempt), &input.state, &input.code).await?;
    let after = super::status(app, pin, &session.claims, &setup.pool, request, digest, &session.digest).await?;
    let original = after.original(owned.attempt)?;
    if after.phase != Phase::SetupConfirmationRequired {
        return Err(refused());
    }
    Ok(Json(Confirmed {
        scope: after.view().scope,
        phase: after.phase,
        attempt: original.attempt,
        page: original.page,
    }))
}

#[rocket::post("/operational-signing/initialize/self-setup/register", format = "json", data = "<input>")]
pub(crate) async fn register(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    input: Json<Register>,
) -> SigningReply<View> {
    let request = input.request_id;
    let digest = &input.payload_sha256;
    let before = super::status(app, pin, &session.claims, &setup.pool, request, digest, &session.digest).await?;
    let owned = before.original(input.attempt)?;
    if !matches!(before.phase, Phase::SetupConfirmationRequired | Phase::SetupRetryAvailable)
        || input.delivery_revision != owned.revision
    {
        return Err(refused());
    }
    record_intent(app, &session, &before.scope, owned.attempt, AdminAction::Update,
        "Requested key registration for the confirmed Initialize setup attempt").await?;
    let released = release(app, pin, &session.claims, &setup.pool, request, digest, owned.attempt,
        &session.digest, owned.revision).await?;
    let scope = checked_scope(app, pin, &session.claims, request, digest).await?;
    if scope != released.scope {
        return Err(refused());
    }
    // A lost or invalid receipt can follow a commit. Reconcile against 2200
    // before answering. Only an explicit click on an unclaimed revision can resend.
    let delivery = setup.device.deliver(pin, &released).await;
    let after = super::status(app, pin, &session.claims, &setup.pool, request, digest, &session.digest).await?;
    match (delivery, after.phase) {
        (Ok(()), _) | (Err(_), Phase::SetupRetryAvailable | Phase::SetupPending
            | Phase::SetupRecoveryRequired | Phase::CertificateOnFile) => Ok(Json(after.view())),
        (Err(error), _) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_projection_requires_one_consistent_non_nil_session() {
        let id = Uuid::from_u128(7);
        let other = Uuid::from_u128(8);
        for claims in [json!({"sid": id}), json!({"session_state": id}),
                       json!({"sid": id, "session_state": id})] {
            assert_eq!(serde_json::from_value::<SessionId>(claims).unwrap().id(), Some(id));
        }
        for claims in [json!({}), json!({"sid": Uuid::nil()}),
                       json!({"sid": id, "session_state": other})] {
            assert_eq!(serde_json::from_value::<SessionId>(claims).unwrap().id(), None);
        }
    }

    #[test]
    fn the_browser_cannot_supply_a_setup_principal_or_device() {
        let original = json!({"request_id": Uuid::from_u128(1), "payload_sha256": "ab".repeat(32),
            "attempt": Uuid::from_u128(2), "delivery_revision": 0});
        assert!(serde_json::from_value::<Register>(original.clone()).is_ok());
        for field in ["official", "role", "sid", "issuer", "device", "fence"] {
            let mut unknown = original.clone();
            unknown[field] = json!("extra");
            assert!(serde_json::from_value::<Register>(unknown).is_err());
        }
    }
}
