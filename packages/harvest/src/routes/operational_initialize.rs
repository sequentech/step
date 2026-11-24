// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Initialize approval routes. Install only with an
//! authenticated private broker that implements the exact owned ceremony,
//! reservation, verified device, and durable reconciliation protocol.
//! No credential, bytes, actor, certificate, or HSM mode is accepted from
//! the browser. Uses the real Initialize voting approval and action paths.

use super::signing::{signing_failure, SigningReply};
use crate::services::audit;
use crate::services::dependencies::HarvestServices;
use crate::services::operational_portal_auth::OperationalPortalClaims;
use crate::services::signing_http::{
    authorize_403, SigningError, SigningErrorCode,
};
use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{future::Future, pin::Pin};
use uuid::Uuid;
use windmill::postgres::audit_log_outbox::AuditLogTarget;
use windmill::services::audit_log::{
    self, AdminAction, AdminChange, AuditObject, FieldChange,
};
use windmill::services::signing::approve::approve_operational;
use windmill::services::signing::operational_approval::OperationalApprovalInput;
use windmill::services::signing::operational_initialize::{
    initialize_signing_applied, initialize_signing_scope,
    prepare_initialize_signing, InitializeSigningScope,
};
use windmill::services::signing::SigningCaller;
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

#[cfg(all(
    feature = "felix3-local-initialize",
    not(feature = "operational-signing-demo-fixture")
))]
#[path = "local_initialize_authority.rs"]
pub(crate) mod local;

#[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
#[path = "local_initialize_self_setup.rs"]
pub(crate) mod local_setup;

/// The ledger's attempt table has no `audit_change` trigger: a route that
/// changes an attempt stages the entry itself, in the event's log.
const ATTEMPT: &str = "operational_initialize_attempt";

fn attempt_change(
    claims: &JwtClaims,
    scope: &InitializeSigningScope,
    attempt: Uuid,
) -> AdminChange {
    audit::change(
        claims,
        AuditLogTarget {
            tenant_id: scope.tenant(),
            election_event_id: Some(scope.event()),
        },
        AuditObject::new(ATTEMPT, attempt, None),
        AdminAction::Update,
    )
}

/// An official discarded their own unused attempt.
fn attempt_cancelled(
    claims: &JwtClaims,
    scope: &InitializeSigningScope,
    attempt: Uuid,
) -> AdminChange {
    attempt_change(claims, scope, attempt)
        .with_changes(vec![FieldChange::new(
            "step",
            None,
            Some(&json!("discarded")),
        )])
        .with_extra(json!({"signing_request_id": scope.request()}))
        .with_description("Cancelled an unused Initialize signing attempt")
}

/// The fixed page an attempt returns its official to.
fn attempt_return_recorded(
    claims: &JwtClaims,
    scope: &InitializeSigningScope,
    attempt: Uuid,
    page: ReturnPage,
) -> AdminChange {
    let election = page.election_id.map(|id| json!(id));
    attempt_change(claims, scope, attempt)
        .with_changes(vec![
            FieldChange::new(
                "return_path",
                None,
                Some(&json!(page.path.as_str())),
            ),
            FieldChange::new(
                "return_panel",
                None,
                Some(&json!(page.panel.as_str())),
            ),
            FieldChange::new("return_election_id", None, election.as_ref()),
        ])
        .with_extra(json!({"signing_request_id": scope.request()}))
        .with_description(
            "Recorded the return page of an Initialize signing attempt",
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    request_id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Return {
    request_id: Uuid,
    attempt: Uuid,
    state: String,
    code: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PagePath {
    EventBase,
    EventShow,
    EventEdit,
    ElectionBase,
    ElectionShow,
    ElectionEdit,
}
impl PagePath {
    fn as_str(self) -> &'static str {
        match self {
            Self::EventBase => "event-base",
            Self::EventShow => "event-show",
            Self::EventEdit => "event-edit",
            Self::ElectionBase => "election-base",
            Self::ElectionShow => "election-show",
            Self::ElectionEdit => "election-edit",
        }
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PagePanel {
    Publish,
    SignaturesRequests,
}
impl PagePanel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Publish => "publish",
            Self::SignaturesRequests => "signatures-requests",
        }
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReturnPage {
    path: PagePath,
    panel: PagePanel,
    #[serde(default)]
    election_id: Option<Uuid>,
}

impl ReturnPage {
    fn matches_request(self, election: Option<Uuid>) -> bool {
        match self.path {
            PagePath::EventBase | PagePath::EventShow | PagePath::EventEdit => {
                self.election_id.is_none()
            }
            PagePath::ElectionBase
            | PagePath::ElectionShow
            | PagePath::ElectionEdit => {
                matches!(self.panel, PagePanel::Publish)
                    && self
                        .election_id
                        .is_some_and(|id| !id.is_nil() && Some(id) == election)
            }
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartGold {
    request_id: Uuid,
    payload_sha256: String,
    page: ReturnPage,
}
#[derive(Serialize)]
pub struct GoldReturnView {
    tenant: Uuid,
    event: Uuid,
    request: Uuid,
    official: Uuid,
    attempt: Uuid,
    phase: Phase,
    page: ReturnPage,
}

#[derive(Serialize)]
pub struct Begin {
    attempt: Uuid,
    redirect: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Unavailable,
    ChallengeAvailable,
    WaitingForIdentity,
    Pending,
    ApprovalRecorded,
    Reconcile,
}

#[derive(Serialize)]
pub struct Outcome {
    phase: Phase,
}

#[derive(Serialize)]
pub(crate) struct PendingState {
    scope: PortalScope,
    phase: Phase,
    // Publicly safe reference for a never-used owned attempt, not bearer proof.
    cancellable_attempt: Option<Uuid>,
    device_profile: &'static str,
}
#[derive(Clone, Copy, Serialize)]
struct PortalScope {
    tenant: Uuid,
    event: Uuid,
    request: Uuid,
    official: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelAttempt {
    request_id: Uuid,
    attempt: Uuid,
}

pub(crate) struct BrokerRedirect {
    pub attempt: Uuid,
    pub location: reqwest::Url,
}

pub(crate) struct Settled {
    pub scope: BrokerScope,
    pub approval_challenge: Uuid,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct BrokerScope {
    pub tenant: Uuid,
    pub event: Uuid,
    pub request: Uuid,
    pub official: Uuid,
    pub payload_sha256: String,
}

impl From<&InitializeSigningScope> for BrokerScope {
    fn from(value: &InitializeSigningScope) -> Self {
        Self {
            tenant: value.tenant(),
            event: value.event(),
            request: value.request(),
            official: value.actor(),
            payload_sha256: value.payload_sha256().to_owned(),
        }
    }
}

pub(crate) enum BrokerFailure {
    Refused,
    Unavailable,
    Reconcile,
}

pub(crate) type BrokerFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, BrokerFailure>> + Send + 'a>>;

pub(crate) trait OperationalInitializeBroker: Send + Sync {
    fn can_offer<'a>(
        &'a self,
        prepared: &'a BrokerScope,
    ) -> BrokerFuture<'a, bool>;
    fn start<'a>(
        &'a self,
        prepared: &'a BrokerScope,
    ) -> BrokerFuture<'a, BrokerRedirect>;

    /// Only after validating the stored state ownership and the exact IdP
    /// callback/step-up, one atomic ledger reservation, and independently
    /// verified device settlement. Retried completed requests return existing
    /// exact evidence only; never make another device sign call.
    fn finish<'a>(
        &'a self,
        prepared: &'a BrokerScope,
        attempt: Uuid,
        state: &'a str,
        code: &'a str,
    ) -> BrokerFuture<'a, Settled>;

    /// The installed OIDC authorization endpoint and dedicated client.
    fn authorization_endpoint(&self) -> &reqwest::Url;
    fn oidc_client_id(&self) -> &str;
}

pub(crate) struct InitializeBroker(pub Box<dyn OperationalInitializeBroker>);

/// Checked against the same protected install marker as startup, independently
/// of browser data and the broker response. Other scopes never reach the broker.
pub(crate) struct DemoSigningScope {
    pub tenant: Uuid,
    pub event: Uuid,
    pub run: Uuid,
    pub profile: &'static str,
}

impl DemoSigningScope {
    async fn verify(
        &self,
        tx: &deadpool_postgres::Transaction<'_>,
        request: &InitializeSigningScope,
    ) -> Result<(), SigningError> {
        let refused = || {
            SigningError::forbidden("Signing is unavailable for this request.")
        };
        if (request.tenant(), request.event()) != (self.tenant, self.event)
            || !request.election().is_some_and(|post| !post.is_nil())
        {
            return Err(refused());
        }
        let run: Option<Uuid> = tx
            .query_one("SELECT sequent_backend.operational_signing_demo_fixture_id($1,$2,$3,$4)",
                       &[&self.tenant, &self.event, &"initialize-voting", &self.profile])
            .await.map_err(SigningError::internal)?
            .try_get(0).map_err(SigningError::internal)?;
        if !self.run.is_nil() && run == Some(self.run) {
            Ok(())
        } else {
            Err(refused())
        }
    }
}

pub(crate) enum ApprovalInstallation<'r> {
    Demo(&'r DemoSigningScope),
    #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
    Local(&'r local::LocalInitializePost),
}

// Keep the mounted route's managed-state check at Rocket ignition, including
// the sealed installation's original DemoSigningScope requirement.
impl rocket::Sentinel for ApprovalInstallation<'_> {
    fn abort(rocket: &rocket::Rocket<rocket::Ignite>) -> bool {
        #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
        {
            use crate::services::operational_portal_auth::OperationalPortalAuth;
            if rocket.state::<OperationalPortalAuth>().is_some_and(|a| a.requires_local_ingress()) {
                return rocket.state::<local::LocalInitializePost>().is_none()
                    || rocket.state::<local::LocalIngress>().is_none();
            }
        }
        rocket.state::<DemoSigningScope>().is_none()
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for ApprovalInstallation<'r> {
    type Error = ();
    async fn from_request(req: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, ()> {
        use rocket::request::Outcome;
        let unavailable = || Outcome::Error((Status::ServiceUnavailable, ()));
        #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
        {
            use crate::services::operational_portal_auth::OperationalPortalAuth;
            if req.rocket().state::<OperationalPortalAuth>().is_some_and(|a| a.requires_local_ingress()) {
                return req.rocket().state::<local::LocalInitializePost>()
                    .map(|s| Outcome::Success(Self::Local(s))).unwrap_or_else(unavailable);
            }
        }
        req.rocket().state::<DemoSigningScope>()
            .map(|s| Outcome::Success(Self::Demo(s))).unwrap_or_else(unavailable)
    }
}

impl ApprovalInstallation<'_> {
    async fn verify(
        &self, tx: &deadpool_postgres::Transaction<'_>, services: &HarvestServices,
        actor: &SigningCaller, scope: &InitializeSigningScope,
    ) -> Result<(), SigningError> {
        match self {
            Self::Demo(pin) => {
                let _ = (services, actor);
                pin.verify(tx, scope).await
            }
            #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
            Self::Local(pin) => pin.verify(tx, services, actor, scope).await,
        }
    }

    fn permits_broker(&self, broker: &dyn OperationalInitializeBroker) -> bool {
        match self {
            Self::Demo(_) => broker.authorization_endpoint().scheme() == "https",
            #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
            Self::Local(pin) => pin.permits_broker(broker),
        }
    }

    fn profile(&self) -> &'static str {
        match self {
            Self::Demo(pin) => pin.profile,
            #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
            Self::Local(pin) => pin.profile,
        }
    }
}

fn caller(claims: &JwtClaims) -> Result<(Uuid, SigningCaller), SigningError> {
    authorize_403(
        claims,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::SIGN_INITIALIZE_VOTING],
    )?;
    let tenant = Uuid::parse_str(&claims.hasura_claims.tenant_id)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or_else(|| SigningError::forbidden("Invalid signing tenant."))?;
    Ok((tenant, SigningCaller::from_claims(claims)))
}

fn broker_error(failure: BrokerFailure) -> SigningError {
    match failure {
        BrokerFailure::Refused => {
            SigningError::forbidden("The device signing attempt was refused.")
        }
        BrokerFailure::Unavailable => SigningError::new(
            Status::ServiceUnavailable,
            SigningErrorCode::Internal,
            "The private signing service is unavailable.",
        ),
        BrokerFailure::Reconcile => SigningError::conflict(
            "The device result needs reconciliation. Check your approval before signing again.",
        ),
    }
}

#[cfg(test)]
fn permitted_redirect(
    location: &reqwest::Url,
    configured: &reqwest::Url,
    oidc_client: &str,
) -> bool {
    location.scheme() == "https" && matches_pushed_redirect(location, configured, oidc_client)
}

pub(super) fn matches_pushed_redirect(
    location: &reqwest::Url,
    configured: &reqwest::Url,
    oidc_client: &str,
) -> bool {
    const PREFIX: &str = "urn:ietf:params:oauth:request_uri:";
    if location.origin() != configured.origin()
        || location.path() != configured.path()
        || !location.username().is_empty()
        || location.password().is_some()
        || location.fragment().is_some()
    {
        return false;
    }
    let mut client = false;
    let mut request_uri = None;
    for (name, value) in location.query_pairs() {
        match name.as_ref() {
            "client_id" if !client && value == oidc_client => client = true,
            "request_uri" if request_uri.is_none() => {
                let Some(suffix) = value.strip_prefix(PREFIX) else {
                    return false;
                };
                if suffix.is_empty()
                    || suffix.len() > 256
                    || !suffix.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || b"-_.".contains(&b)
                    })
                {
                    return false;
                }
                request_uri = Some(value);
            }
            _ => return false,
        }
    }
    let Some(request_uri) = request_uri else { return false };
    if !client { return false; }
    if configured.scheme() == "http" {
        let mut canonical = configured.clone();
        canonical.query_pairs_mut().append_pair("client_id", oidc_client)
            .append_pair("request_uri", &request_uri);
        return location == &canonical;
    }
    true
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/initialize/status",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn status(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    installed: ApprovalInstallation<'_>,
    broker: Option<&State<InitializeBroker>>,
    input: Json<Start>,
) -> SigningReply<PendingState> {
    let (tenant, caller) = caller(&claims.0)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let scoped =
        initialize_signing_scope(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    installed.verify(&tx, services, &caller, &scoped).await?;
    let scope = PortalScope {
        tenant,
        event: scoped.event(),
        request: scoped.request(),
        official: scoped.actor(),
    };
    let response = |phase, cancellable_attempt| -> SigningReply<PendingState> {
        Ok(Json(PendingState {
            scope,
            phase,
            cancellable_attempt,
            device_profile: installed.profile(),
        }))
    };
    if initialize_signing_applied(&tx, &caller, tenant, input.request_id)
        .await
        .map_err(signing_failure)?
    {
        return response(Phase::ApprovalRecorded, None);
    }
    if let Some((stage, attempt, claimed)) =
        current_attempt(&tx, &scoped, &caller).await?
    {
        return match stage.as_str() {
            "prepared" if !claimed => {
                response(Phase::WaitingForIdentity, Some(attempt))
            }
            "reserved" | "invoked" | "prepared" => {
                response(Phase::Pending, None)
            }
            _ => response(Phase::Reconcile, None),
        };
    }
    let Ok(open) =
        prepare_initialize_signing(&tx, &caller, tenant, input.request_id)
            .await
    else {
        return response(Phase::Unavailable, None);
    };
    tx.commit().await.map_err(SigningError::internal)?;
    if let Some(broker) = broker {
        if broker
            .0
            .can_offer(&BrokerScope::from(&open))
            .await
            .unwrap_or(false)
        {
            return response(Phase::ChallengeAvailable, None);
        }
    }
    response(Phase::Unavailable, None)
}

async fn current_attempt(
    tx: &deadpool_postgres::Transaction<'_>,
    scoped: &InitializeSigningScope,
    caller: &SigningCaller,
) -> Result<Option<(String, Uuid, bool)>, SigningError> {
    let row=tx.query_opt(
        "SELECT phase,challenge,callback_claimed FROM sequent_backend.operational_initialize_current_state($1,$2,$3,$4,$5)",
        &[&scoped.tenant(),&scoped.event(),&scoped.request(),&caller.user_id,&scoped.payload_sha256()],
    ).await.map_err(SigningError::internal)?;
    row.map(|r| -> Result<_, tokio_postgres::Error> {
        Ok((r.try_get(0)?, r.try_get(1)?, r.try_get(2)?))
    })
    .transpose()
    .map_err(SigningError::internal)
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/initialize/cancel",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn cancel_unused(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    installed: ApprovalInstallation<'_>,
    input: Json<CancelAttempt>,
) -> SigningReply<Outcome> {
    let (tenant, caller) = caller(&claims.0)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let scoped =
        initialize_signing_scope(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    installed.verify(&tx, services, &caller, &scoped).await?;
    let discarded: bool = tx
        .query_one(
            "SELECT sequent_backend.operational_initialize_discard_unredeemed($1,$2,$3,$4,$5,$6)",
            &[
                &tenant,
                &scoped.event(),
                &scoped.request(),
                &caller.user_id,
                &scoped.payload_sha256(),
                &input.attempt,
            ],
        )
        .await
        .map_err(SigningError::internal)?
        .try_get(0)
        .map_err(SigningError::internal)?;
    if !discarded {
        return Err(SigningError::conflict(
            "Can't cancel an attempt after it could have reached the device.",
        ));
    }
    // The entry commits with the discard, or neither does.
    audit_log::stage(
        &tx,
        &attempt_cancelled(&claims.0, &scoped, input.attempt),
    )
    .await
    .map_err(SigningError::internal)?;
    tx.commit().await.map_err(SigningError::internal)?;
    kick_audit_log_outbox();
    Ok(Json(Outcome {
        phase: Phase::Unavailable,
    }))
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/initialize/reconcile",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn reconcile(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    installed: ApprovalInstallation<'_>,
    input: Json<Start>,
) -> SigningReply<Outcome> {
    let (tenant, caller) = caller(&claims.0)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let scoped =
        initialize_signing_scope(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    installed.verify(&tx, services, &caller, &scoped).await?;
    if initialize_signing_applied(&tx, &caller, tenant, input.request_id)
        .await
        .map_err(signing_failure)?
    {
        return Ok(Json(Outcome {
            phase: Phase::ApprovalRecorded,
        }));
    }
    let Some((phase, challenge, _)) =
        current_attempt(&tx, &scoped, &caller).await?
    else {
        return Ok(Json(Outcome {
            phase: Phase::Unavailable,
        }));
    };
    if phase != "completed" {
        return Ok(Json(Outcome {
            phase: Phase::Reconcile,
        }));
    }
    tx.commit().await.map_err(SigningError::internal)?;
    // Only an already-existing completed receipt is applied; this never
    // initiates another IdP step-up or device operation.
    let outcome = approve_operational(
        &mut client,
        &services.signing,
        &caller,
        tenant,
        &OperationalApprovalInput {
            request_id: input.request_id,
            approval_challenge: challenge,
            artifact_challenge: None,
        },
        Utc::now(),
    )
    .await;
    if outcome.is_ok() {
        return Ok(Json(Outcome {
            phase: Phase::ApprovalRecorded,
        }));
    }
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let applied =
        initialize_signing_applied(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    Ok(Json(Outcome {
        phase: if applied {
            Phase::ApprovalRecorded
        } else {
            Phase::Reconcile
        },
    }))
}
#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/initialize/start",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn start(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    installed: ApprovalInstallation<'_>,
    broker: &State<InitializeBroker>,
    input: Json<StartGold>,
) -> SigningReply<Begin> {
    let (tenant, caller) = caller(&claims.0)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let prepared =
        prepare_initialize_signing(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    installed.verify(&tx, services, &caller, &prepared).await?;
    if input.payload_sha256 != prepared.payload_sha256() {
        return Err(SigningError::conflict(
            "The reviewed signing request no longer matches. Review it again before approving.",
        ));
    }
    let page = input.page;
    if !page.matches_request(prepared.election()) {
        return Err(SigningError::forbidden(
            "The return page does not match this Initialize request.",
        ));
    }
    if initialize_signing_applied(&tx, &caller, tenant, input.request_id)
        .await
        .map_err(signing_failure)?
    {
        return Err(SigningError::conflict(
            "Your approval is already recorded.",
        ));
    }
    if !installed.permits_broker(broker.0.as_ref()) {
        return Err(broker_error(BrokerFailure::Unavailable));
    }
    tx.commit().await.map_err(SigningError::internal)?;
    let redirect = broker
        .0
        .start(&(&prepared).into())
        .await
        .map_err(broker_error)?;
    if redirect.attempt.is_nil()
        || !matches_pushed_redirect(
            &redirect.location,
            broker.0.authorization_endpoint(),
            broker.0.oidc_client_id(),
        )
    {
        return Err(broker_error(BrokerFailure::Unavailable));
    }
    // The return page and its audit entry commit together.
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
    if let ApprovalInstallation::Local(pin) = &installed {
        let current = prepare_initialize_signing(&tx, &caller, tenant, input.request_id)
            .await.map_err(signing_failure)?;
        if current.payload_sha256() != prepared.payload_sha256() {
            return Err(broker_error(BrokerFailure::Refused));
        }
        pin.verify(&tx, services, &caller, &current).await?;
    }
    let saved: bool=tx.query_one(
       "SELECT sequent_backend.operational_initialize_record_browser_return($1,$2,$3,$4,$5,$6,$7,$8)",
       &[&tenant,&prepared.event(),&prepared.request(),&caller.user_id,&redirect.attempt,
         &page.path.as_str(),&page.panel.as_str(),&page.election_id],
    ).await.map_err(SigningError::internal)?.try_get(0).map_err(SigningError::internal)?;
    // If page persistence failed the broker attempt is still unused, and the
    // authenticated status can cancel it. Never expose this redirect anyway.
    if !saved {
        return Err(SigningError::conflict(
            "This signing attempt could not preserve its return page. Check its status.",
        ));
    }
    audit_log::stage(
        &tx,
        &attempt_return_recorded(&claims.0, &prepared, redirect.attempt, page),
    )
    .await
    .map_err(SigningError::internal)?;
    tx.commit().await.map_err(SigningError::internal)?;
    kick_audit_log_outbox();
    Ok(Json(Begin {
        attempt: redirect.attempt,
        redirect: redirect.location.to_string(),
    }))
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/initialize/finish",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn finish(
    claims: OperationalPortalClaims,
    services: &State<HarvestServices>,
    installed: ApprovalInstallation<'_>,
    broker: &State<InitializeBroker>,
    input: Json<Return>,
) -> SigningReply<GoldReturnView> {
    let (tenant, caller) = caller(&claims.0)?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let authorized =
        initialize_signing_scope(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    installed.verify(&tx, services, &caller, &authorized).await?;
    let page =
        read_browser_return(&tx, &authorized, &caller, input.attempt).await?;
    let response = |phase| {
        Ok(Json(GoldReturnView {
            tenant,
            event: authorized.event(),
            request: authorized.request(),
            official: authorized.actor(),
            attempt: input.attempt,
            phase,
            page,
        }))
    };
    if initialize_signing_applied(&tx, &caller, tenant, input.request_id)
        .await
        .map_err(signing_failure)?
    {
        return response(Phase::ApprovalRecorded);
    }
    let prepared =
        prepare_initialize_signing(&tx, &caller, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    tx.commit().await.map_err(SigningError::internal)?;
    if input.attempt.is_nil()
        || input.state.len() != 43
        || !input
            .state
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || input.code.is_empty()
        || input.code.len() > 4096
        || !input.code.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(SigningError::invalid("Invalid signing callback."));
    }
    if !installed.permits_broker(broker.0.as_ref()) {
        return Err(broker_error(BrokerFailure::Unavailable));
    }
    let scope = BrokerScope::from(&prepared);
    let receipt = match broker
        .0
        .finish(&scope, input.attempt, &input.state, &input.code)
        .await
    {
        Ok(receipt) => receipt,
        Err(BrokerFailure::Reconcile) => return response(Phase::Reconcile),
        Err(other) => return Err(broker_error(other)),
    };
    if receipt.scope != scope || receipt.approval_challenge != input.attempt {
        return Err(broker_error(BrokerFailure::Refused));
    }
    #[cfg(all(feature = "felix3-local-initialize", not(feature = "operational-signing-demo-fixture")))]
    if let ApprovalInstallation::Local(pin) = &installed {
        let tx = client.transaction().await.map_err(SigningError::internal)?;
        let current = prepare_initialize_signing(&tx, &caller, tenant, input.request_id)
            .await.map_err(signing_failure)?;
        if current.payload_sha256() != prepared.payload_sha256() {
            return Err(broker_error(BrokerFailure::Refused));
        }
        pin.verify(&tx, services, &caller, &current).await?;
        tx.commit().await.map_err(SigningError::internal)?;
    }
    let result = approve_operational(
        &mut client,
        &services.signing,
        &caller,
        tenant,
        &OperationalApprovalInput {
            request_id: input.request_id,
            approval_challenge: receipt.approval_challenge,
            artifact_challenge: None,
        },
        Utc::now(),
    )
    .await;
    match result {
        Ok(_) => response(Phase::ApprovalRecorded),
        Err(error) => {
            // A lost commit response is not permission to sign again.
            // Re-read the exact, actor-scoped durable state.
            let tx =
                client.transaction().await.map_err(SigningError::internal)?;
            if initialize_signing_applied(
                &tx,
                &caller,
                tenant,
                input.request_id,
            )
            .await
            .map_err(signing_failure)?
            {
                response(Phase::ApprovalRecorded)
            } else {
                Err(signing_failure(error))
            }
        }
    }
}

async fn read_browser_return(
    tx: &deadpool_postgres::Transaction<'_>,
    s: &InitializeSigningScope,
    caller: &SigningCaller,
    attempt: Uuid,
) -> Result<ReturnPage, SigningError> {
    let invalid =
        || SigningError::forbidden("No owned signing return for this attempt.");
    if attempt.is_nil() {
        return Err(invalid());
    }
    let row=tx.query_opt(
        "SELECT path_kind,panel,election_id FROM sequent_backend.operational_initialize_get_browser_return($1,$2,$3,$4,$5)",
        &[&s.tenant(),&s.event(),&s.request(),&caller.user_id,&attempt],
    ).await.map_err(SigningError::internal)?.ok_or_else(invalid)?;
    let path: String = row.try_get(0).map_err(SigningError::internal)?;
    let panel: String = row.try_get(1).map_err(SigningError::internal)?;
    let election_id: Option<Uuid> =
        row.try_get(2).map_err(SigningError::internal)?;
    let page: ReturnPage = serde_json::from_value(serde_json::json!({
        "path": path, "panel": panel, "election_id": election_id,
    }))
    .map_err(|_| invalid())?;
    if !page.matches_request(s.election()) {
        return Err(invalid());
    }
    Ok(page)
}

#[cfg(test)]
mod return_page_tests {
    use super::*;

    #[test]
    fn current_exact_post_only_for_election_publish_returns() {
        let election = Uuid::from_u128(1);
        let other = Uuid::from_u128(2);
        for path in [
            PagePath::ElectionBase,
            PagePath::ElectionShow,
            PagePath::ElectionEdit,
        ] {
            assert!(ReturnPage {
                path,
                panel: PagePanel::Publish,
                election_id: Some(election)
            }
            .matches_request(Some(election)));
            for invalid in [Some(other), Some(Uuid::nil()), None] {
                assert!(!ReturnPage {
                    path,
                    panel: PagePanel::Publish,
                    election_id: invalid
                }
                .matches_request(Some(election)));
            }
            assert!(!ReturnPage {
                path,
                panel: PagePanel::SignaturesRequests,
                election_id: Some(election)
            }
            .matches_request(Some(election)));
        }
        for path in [
            PagePath::EventBase,
            PagePath::EventShow,
            PagePath::EventEdit,
        ] {
            assert!(ReturnPage {
                path,
                panel: PagePanel::SignaturesRequests,
                election_id: None
            }
            .matches_request(Some(election)));
            assert!(!ReturnPage {
                path,
                panel: PagePanel::Publish,
                election_id: Some(election)
            }
            .matches_request(Some(election)));
        }
    }

    #[test]
    fn accepts_legacy_event_shape_but_never_user_supplied_paths() {
        for page in [
            serde_json::json!({"path":"event-show", "panel":"publish"}),
            serde_json::json!({"path":"event-show", "panel":"publish", "election_id":null}),
        ] {
            assert!(serde_json::from_value::<ReturnPage>(page)
                .unwrap()
                .matches_request(None));
        }
        assert!(serde_json::from_value::<ReturnPage>(serde_json::json!({
            "path":"https://not-local.invalid", "panel":"publish"
        }))
        .is_err());
    }

    #[test]
    fn only_exact_authorization_endpoint_and_par_parameters_can_redirect_the_browser(
    ) {
        let root = "https://issuer.invalid/realms/tenant-example/protocol/openid-connect/auth";
        let expected = reqwest::Url::parse(root).unwrap();
        let client = "private-oidc-client";
        let valid_pair = "client_id=private-oidc-client&request_uri=urn:ietf:params:oauth:request_uri:safe_opaque.id";
        let admit = |url: String| {
            permitted_redirect(
                &reqwest::Url::parse(&url).unwrap(),
                &expected,
                client,
            )
        };
        assert!(admit(format!("{root}?{valid_pair}")));
        for target in [
            root.replace("https", "http"),
            root.replace("issuer.invalid", "other.invalid"),
            root.replace("issuer.invalid", "name:pass@issuer.invalid"),
            root.replace("/auth", "/token"),
        ] {
            assert!(!admit(format!("{target}?{valid_pair}")));
        }
        for params in [
            String::new(),
            "client_id=private-oidc-client".into(),
            "request_uri=urn:ietf:params:oauth:request_uri:safe".into(),
            "client_id=wrong&request_uri=urn:ietf:params:oauth:request_uri:safe".into(),
            "client_id=private-oidc-client&request_uri=https://wrong.invalid".into(),
            "client_id=private-oidc-client&request_uri=urn:ietf:params:oauth:request_uri:".into(),
            "client_id=private-oidc-client&request_uri=urn:ietf:params:oauth:request_uri:not%2Fopaque".into(),
            format!("{valid_pair}#fragment"),
            format!("{valid_pair}&client_id={client}"),
            format!("{valid_pair}&redirect_uri=https://wrong.invalid"),
            format!("client_id={client}&request_uri=urn:ietf:params:oauth:request_uri:{}", "s".repeat(257)),
        ] {
            assert!(!admit(format!("{root}?{params}")));
        }
    }
}
