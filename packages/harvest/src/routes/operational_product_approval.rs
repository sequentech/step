// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ordinary product approvals. A verified, configured production private
//! broker is mandatory. The loopback demo has separate routes and trust
//! configuration and cannot mount this service.

use super::operational_product_broker_client::PrivateProductBrokerClient;
use super::operational_setup::product::{
    self as setup_routes, ProductSetupActorPool, ProductSetupDispatcher,
    ProductSetupInstallations,
};
use super::signing::{signing_failure, SigningReply};
use crate::services::dependencies::HarvestServices;
use crate::services::operational_portal_auth::OperationalPortalAuth;
use crate::services::signing_http::{SigningError, SigningErrorCode};
use chrono::Utc;
use rocket::http::Status;
use rocket::mtls::Certificate;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::{DocumentKind, SigningAction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;
use windmill::services::signing::approve::approve_operational;
use windmill::services::signing::operational_approval::{
    prepare_operational_approval, reauthorize_operational_approval,
    OperationalApprovalInput, OperationalApprovalPlan,
};
use windmill::services::signing::SigningCaller;

const MAX_PORTAL_TOKEN: usize = 32768;
const MAX_OIDC_CODE: usize = 8192;
const REQUEST_URI: &str = "urn:ietf:params:oauth:request_uri:";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductApprovalScope {
    pub tenant: Uuid,
    pub event: Uuid,
    pub request: Uuid,
    pub actor: String,
    pub action: SigningAction,
}

pub(crate) struct ProductRedirect {
    pub attempt: Uuid,
    pub authorization_url: reqwest::Url,
}

pub(crate) enum ProductBrokerStep {
    /// A separate bound Gold ceremony is required for an EML or PDF part.
    Next(ProductRedirect),
    Complete {
        scope: ProductApprovalScope,
        payload_challenge: Uuid,
        artifact_challenge: Option<Uuid>,
    },
    /// An external call might have taken effect. Never reissue it.
    Reconcile,
    Ready,
}

pub(crate) enum ProductBrokerFailure {
    Refused,
    Unavailable,
    Reconcile,
}

/// Implement on the trusted private production client only. Begin and
/// finish perform no mock/SaaS fallback. Finish verifies nonce/state,
/// exact issuer/client/audience, authentication age/Gold and the same
/// subject, reserves each part once, and accepts only settled device
/// receipts. The server-owned plan is never sent to the browser.
#[async_trait::async_trait]
pub(crate) trait ProductApprovalBroker: Send + Sync {
    async fn begin(
        &self,
        scope: &ProductApprovalScope,
        plan: OperationalApprovalPlan,
    ) -> Result<ProductRedirect, ProductBrokerFailure>;

    async fn finish(
        &self,
        scope: &ProductApprovalScope,
        attempt: Uuid,
        state: &str,
        code: &str,
    ) -> Result<ProductBrokerStep, ProductBrokerFailure>;

    /// Read durable existing receipts. Never starts a ceremony, exchanges a
    /// code, or calls the device again after an uncertain response.
    async fn recover(
        &self,
        scope: &ProductApprovalScope,
    ) -> Result<ProductBrokerStep, ProductBrokerFailure>;
}

pub(crate) struct ProductTenantAuthority {
    portal: OperationalPortalAuth,
    gold_authorization_endpoint: reqwest::Url,
    gold_client: String,
    portal_origin: String,
}

impl ProductTenantAuthority {
    pub(crate) fn new(
        portal: OperationalPortalAuth,
        gold_authorization_endpoint: reqwest::Url,
        gold_client: String,
        portal_origin: reqwest::Url,
    ) -> Option<Self> {
        if !production_https(&gold_authorization_endpoint)
            || gold_authorization_endpoint.query().is_some()
            || !production_https(&portal_origin)
            || portal_origin.path() != "/"
            || portal_origin.query().is_some()
            || gold_client.is_empty()
            || gold_client.len() > 128
        {
            return None;
        }
        Some(Self {
            portal,
            gold_authorization_endpoint,
            gold_client,
            portal_origin: portal_origin.origin().ascii_serialization(),
        })
    }
}

pub(crate) struct ProductApprovalInstallation {
    tenants: HashMap<Uuid, ProductTenantAuthority>,
    broker: Arc<dyn ProductApprovalBroker>,
    /// SPKI of the dedicated TLS frontend with the approved browser ingress.
    proxy_spki: [u8; 32],
}

impl ProductApprovalInstallation {
    pub(crate) fn new(
        tenants: HashMap<Uuid, ProductTenantAuthority>,
        broker: Arc<dyn ProductApprovalBroker>,
        proxy_spki: [u8; 32],
    ) -> Option<Self> {
        (proxy_spki != [0; 32]
            && !tenants.is_empty()
            && tenants.keys().all(|tenant| !tenant.is_nil()))
        .then_some(Self {
            tenants,
            broker,
            proxy_spki,
        })
    }

    pub(crate) fn serves_tenant(&self, tenant: Uuid) -> bool {
        self.tenants.contains_key(&tenant)
    }

    fn trusted_peer(&self, der: &[u8]) -> bool {
        openssl::x509::X509::from_der(der)
            .and_then(|cert| cert.public_key())
            .and_then(|key| key.public_key_to_der())
            .ok()
            .map(|key| <[u8; 32]>::from(Sha256::digest(key)))
            .is_some_and(|pin| pin == self.proxy_spki)
    }
}

/// No routes are mounted unless startup has supplied the verified authority
/// and private broker. Never call for an isolated/soft-device installation.
pub(crate) fn mount_product_approval(
    app: Rocket<Build>,
    installed: ProductApprovalInstallation,
) -> Result<Rocket<Build>, SigningError> {
    if cfg!(feature = "operational-signing-demo-fixture")
        || !app
            .figment()
            .extract::<rocket::Config>()
            .ok()
            .is_some_and(|c| c.mtls_enabled())
    {
        return Err(unavailable());
    }
    Ok(app
        .manage(installed)
        .mount("/", rocket::routes![start, finish, reconcile]))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionedTenancy {
    tenant: Uuid,
    portal_origin: String,
    portal_issuer: String,
    portal_client: String,
    portal_audience: String,
    portal_jwks_file: PathBuf,
    gold_endpoint: String,
    gold_client: String,
    setup: Option<ProvisionedSetup>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionedProduct {
    proxy_spki_sha256: String,
    broker_origin: String,
    broker_root_pem_file: PathBuf,
    broker_client_pem_file: PathBuf,
    tenants: Vec<ProvisionedTenancy>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionedSetup {
    actor_pool_file: PathBuf,
    dispatch_pool_file: PathBuf,
    postgres_root_pem_file: PathBuf,
    worker_endpoint: String,
    worker_root_pem_file: PathBuf,
    worker_client_pem_file: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionedSetupDb {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
}

fn setup_database(file: &std::path::Path, ca_bytes: &[u8]) -> Option<deadpool_postgres::Pool> {
    use openssl::{
        ssl::{SslConnector, SslMethod, SslVerifyMode, SslVersion},
        x509::{store::X509StoreBuilder, X509},
    };
    use postgres_openssl::MakeTlsConnector;
    let scope: ProvisionedSetupDb = serde_json::from_slice(&read_own_configuration(file, 65536)?).ok()?;
    if scope.host.is_empty() || scope.host.contains(['/', ',', ' ']) || scope.port == 0
        || scope.database.is_empty() || scope.user.is_empty() || scope.password.is_empty()
    {
        return None;
    }
    let mut certs = X509StoreBuilder::new().ok()?;
    let anchors = X509::stack_from_pem(ca_bytes).ok()?;
    if anchors.is_empty() {
        return None;
    }
    for anchor in anchors {
        certs.add_cert(anchor).ok()?;
    }
    let mut builder = SslConnector::builder(SslMethod::tls_client()).ok()?;
    builder.set_verify(SslVerifyMode::PEER);
    builder.set_min_proto_version(Some(SslVersion::TLS1_2)).ok()?;
    builder.set_cert_store(certs.build());
    let mut tls = MakeTlsConnector::new(builder.build());
    tls.set_callback(|ssl, _| {
        ssl.set_verify_hostname(true);
        Ok(())
    });
    let mut config = tokio_postgres::Config::new();
    config.host(&scope.host).port(scope.port).dbname(&scope.database).user(&scope.user)
        .password(scope.password).ssl_mode(tokio_postgres::config::SslMode::Require)
        .connect_timeout(std::time::Duration::from_secs(5));
    let mgr = deadpool_postgres::Manager::from_config(config, tls,
        deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Verified });
    deadpool_postgres::Pool::builder(mgr).runtime(deadpool_postgres::Runtime::Tokio1)
        .max_size(4).build().ok()
}

impl ProvisionedSetup {
    async fn load(self, tenant: Uuid) -> Option<(ProductSetupActorPool, ProductSetupDispatcher)> {
        let worker = reqwest::Url::parse(&self.worker_endpoint).ok()?;
        if !production_https(&worker) || self.actor_pool_file == self.dispatch_pool_file {
            return None;
        }
        let postgres_root = read_own_configuration(&self.postgres_root_pem_file, 131072)?;
        let actor = ProductSetupActorPool::new(setup_database(&self.actor_pool_file, &postgres_root)?).await?;
        let dispatch_db = setup_database(&self.dispatch_pool_file, &postgres_root)?;
        let worker_root = reqwest::Certificate::from_pem(
            &read_own_configuration(&self.worker_root_pem_file, 131072)?,
        ).ok()?;
        let identity = reqwest::Identity::from_pem(
            &read_own_configuration(&self.worker_client_pem_file, 131072)?,
        ).ok()?;
        let dispatcher = ProductSetupDispatcher::new(tenant, dispatch_db, worker.as_str(), worker_root, identity).await?;
        Some((actor, dispatcher))
    }
}

fn read_own_configuration(file: &std::path::Path, max: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    if !file.is_absolute() {
        return None;
    }
    // Deployment supplies these paths via the operator-managed, readonly
    // secret volume, never via event files, Hasura, or webhook input.
    let mut reader = std::fs::File::open(file).ok()?.take(max + 1);
    let mut result = Vec::new();
    reader.read_to_end(&mut result).ok()?;
    (!result.is_empty() && result.len() as u64 <= max).then_some(result)
}

impl ProvisionedProduct {
    async fn load(self) -> Option<(ProductApprovalInstallation, Option<ProductSetupInstallations>)> {
        if self.tenants.is_empty() || self.tenants.len() > 128 {
            return None;
        }
        let broker = PrivateProductBrokerClient::new(
            reqwest::Url::parse(&self.broker_origin).ok()?,
            &read_own_configuration(&self.broker_root_pem_file, 131072)?,
            &read_own_configuration(&self.broker_client_pem_file, 131072)?,
        )?;
        let mut tenants = HashMap::new();
        let mut setups = HashMap::new();
        for entry in self.tenants {
            let issuer = reqwest::Url::parse(&entry.portal_issuer).ok()?;
            let gold_endpoint =
                reqwest::Url::parse(&entry.gold_endpoint).ok()?;
            if !production_https(&issuer)
                || entry.gold_client == entry.portal_client
                || gold_endpoint.origin() != issuer.origin()
                || gold_endpoint.path()
                    != format!("{}/protocol/openid-connect/auth", issuer.path())
            {
                return None;
            }
            let authority = OperationalPortalAuth::new(
                entry.tenant,
                entry.portal_issuer,
                entry.portal_client,
                entry.portal_audience,
                serde_json::from_slice(&read_own_configuration(
                    &entry.portal_jwks_file,
                    131072,
                )?)
                .ok()?,
            )?;
            let configured = ProductTenantAuthority::new(
                authority,
                gold_endpoint,
                entry.gold_client,
                reqwest::Url::parse(&entry.portal_origin).ok()?,
            )?;
            if tenants.insert(entry.tenant, configured).is_some() {
                return None;
            }
            if let Some(setup) = entry.setup {
                setups.insert(entry.tenant, setup.load(entry.tenant).await?);
            }

        }
        let mounted = ProductApprovalInstallation::new(
            tenants,
            Arc::new(broker),
            hex::decode(self.proxy_spki_sha256).ok()?.try_into().ok()?,
        )?;
        let setup = if setups.is_empty() { None } else { Some(ProductSetupInstallations::new(setups)?) };
        Some((mounted, setup))
    }
}

/// No environment setting means no product mount, never a dev-profile fallback.
/// If an installed configuration is malformed, refuse to start that build.
pub(crate) async fn configure_product_approval(
    app: Rocket<Build>,
) -> Result<Rocket<Build>, SigningError> {
    let Some(path) = std::env::var_os("OPERATIONAL_PRODUCT_APPROVAL_CONFIG")
    else {
        return Ok(app);
    };
    let bytes = read_own_configuration(&PathBuf::from(path), 262144)
        .ok_or_else(unavailable)?;
    let (provisioned, setup) = serde_json::from_slice::<ProvisionedProduct>(&bytes)
        .map_err(|_| unavailable())?
        .load()
        .await
        .ok_or_else(unavailable)?;
    let app = mount_product_approval(app, provisioned)?;
    match setup {
        None => Ok(app),
        Some(setup) => Ok(app.manage(setup).mount("/", rocket::routes![
            setup_routes::request_enrollment, setup_routes::dispatch_enrollment, setup_routes::reconcile_unclaimed
        ])),
    }
}

pub(crate) struct ProductBearer {
    token: String,
    origin: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ProductBearer {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let Some(installed) =
            request.rocket().state::<ProductApprovalInstallation>()
        else {
            return Outcome::Error((Status::ServiceUnavailable, ()));
        };
        // No header or peer IP can stand in for the pinned mTLS connection.
        let Outcome::Success(peer) = request.guard::<Certificate<'_>>().await
        else {
            return Outcome::Error((Status::Forbidden, ()));
        };
        if !installed.trusted_peer(peer.as_bytes()) {
            return Outcome::Error((Status::Forbidden, ()));
        }
        let mut origins = request.headers().get("Origin");
        let Some(origin) = origins.next() else {
            return Outcome::Error((Status::Forbidden, ()));
        };
        if origin.len() > 512 || origins.next().is_some() {
            return Outcome::Error((Status::Forbidden, ()));
        }
        let mut headers = request.headers().get("Authorization");
        let Some(token) =
            headers.next().and_then(|h| h.strip_prefix("Bearer "))
        else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        if token.is_empty()
            || token.len() > MAX_PORTAL_TOKEN
            || headers.next().is_some()
        {
            return Outcome::Error((Status::Unauthorized, ()));
        }
        Outcome::Success(Self {
            token: token.to_owned(),
            origin: origin.to_owned(),
        })
    }
}

fn unavailable() -> SigningError {
    SigningError::new(
        Status::ServiceUnavailable,
        SigningErrorCode::Internal,
        "The managed production signing service is unavailable.",
    )
}

fn broker_error(error: ProductBrokerFailure) -> SigningError {
    match error {
        ProductBrokerFailure::Refused => {
            SigningError::forbidden("The device signing ceremony was refused.")
        }
        ProductBrokerFailure::Unavailable => unavailable(),
        ProductBrokerFailure::Reconcile => SigningError::conflict(
            "The previous device attempt needs reconciliation; do not sign again.",
        ),
    }
}

pub(crate) fn authenticate_claims<'a>(
    installed: &'a ProductApprovalInstallation,
    tenant: &str,
    bearer: &ProductBearer,
) -> Result<(Uuid, JwtClaims, &'a ProductTenantAuthority), SigningError> {
    let unauthorized = || {
        SigningError::new(
            Status::Unauthorized,
            SigningErrorCode::Unauthorized,
            "The approval session is not authorized.",
        )
    };
    let selected = Uuid::parse_str(tenant).map_err(|_| unauthorized())?;
    let authority =
        installed.tenants.get(&selected).ok_or_else(unauthorized)?;
    if authority.portal_origin != bearer.origin {
        return Err(unauthorized());
    }
    let claims = authority
        .portal
        .validate(&bearer.token)
        .ok_or_else(unauthorized)?;
    if claims.hasura_claims.tenant_id != selected.to_string() {
        return Err(unauthorized());
    }
    Ok((selected, claims, authority))
}

fn authenticate<'a>(
    installed: &'a ProductApprovalInstallation,
    tenant: &str,
    bearer: &ProductBearer,
) -> Result<(Uuid, SigningCaller, &'a ProductTenantAuthority), SigningError> {
    let (selected, verified, authority) =
        authenticate_claims(installed, tenant, bearer)?;
    Ok((selected, SigningCaller::from_claims(&verified), authority))
}

pub(super) fn production_https(location: &reqwest::Url) -> bool {
    location.scheme() == "https"
        && location.host_str().is_some_and(|host| {
            let host = host.trim_end_matches('.').to_ascii_lowercase();
            host != "localhost"
                && !host.ends_with(".localhost")
                && !host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| {
                        let ip = ip.to_canonical();
                        ip.is_loopback() || ip.is_unspecified()
                    })
        })
        && location.username().is_empty()
        && location.password().is_none()
        && location.fragment().is_none()
}

fn matches_redirect(
    location: &reqwest::Url,
    authority: &ProductTenantAuthority,
) -> bool {
    if !production_https(location)
        || location.origin() != authority.gold_authorization_endpoint.origin()
        || location.path() != authority.gold_authorization_endpoint.path()
    {
        return false;
    }
    let mut client = false;
    let mut par = false;
    for (name, value) in location.query_pairs() {
        match name.as_ref() {
            "client_id"
                if !client && value.as_ref() == authority.gold_client =>
            {
                client = true
            }
            "request_uri" if !par => {
                let Some(identifier) = value.strip_prefix(REQUEST_URI) else {
                    return false;
                };
                if identifier.is_empty()
                    || identifier.len() > 256
                    || !identifier.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || b"-_.".contains(&b)
                    })
                {
                    return false;
                }
                par = true;
            }
            _ => return false,
        }
    }
    client && par
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductStart {
    request_id: Uuid,
    reviewed_payload_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductFinish {
    request_id: Uuid,
    attempt: Uuid,
    state: String,
    code: String,
}

#[derive(Serialize)]
#[serde(tag = "phase", rename_all = "kebab-case")]
pub(crate) enum ProductReply {
    Gold {
        #[serde(flatten)]
        scope: ProductApprovalScope,
        attempt: Uuid,
        authorization_url: String,
    },
    Recorded {
        #[serde(flatten)]
        scope: ProductApprovalScope,
    },
    Reconcile {
        #[serde(flatten)]
        scope: ProductApprovalScope,
    },
    Ready {
        #[serde(flatten)]
        scope: ProductApprovalScope,
    },
}

fn redirect_reply(
    result: ProductRedirect,
    scope: ProductApprovalScope,
    authority: &ProductTenantAuthority,
) -> SigningReply<ProductReply> {
    if result.attempt.is_nil()
        || !matches_redirect(&result.authorization_url, authority)
    {
        return Err(unavailable());
    }
    Ok(Json(ProductReply::Gold {
        scope,
        attempt: result.attempt,
        authorization_url: result.authorization_url.to_string(),
    }))
}

async fn client(
    services: &HarvestServices,
) -> Result<deadpool_postgres::Client, SigningError> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)
}

fn hash_is_canonical(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/v2/<tenant>/start",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn start(
    tenant: &str,
    bearer: ProductBearer,
    installed: &State<ProductApprovalInstallation>,
    services: &State<HarvestServices>,
    input: Json<ProductStart>,
) -> SigningReply<ProductReply> {
    let (tenant, actor, authority) = authenticate(installed, tenant, &bearer)?;
    if input.request_id.is_nil()
        || !hash_is_canonical(&input.reviewed_payload_sha256)
    {
        return Err(SigningError::invalid(
            "Invalid approval request or reviewed payload.",
        ));
    }
    let mut client = client(services).await?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let plan = prepare_operational_approval(
        &tx,
        &services.signing,
        &actor,
        tenant,
        input.request_id,
        &input.reviewed_payload_sha256,
        Utc::now(),
    )
    .await
    .map_err(signing_failure)?;
    let scope = ProductApprovalScope {
        tenant: plan.tenant_id,
        event: plan.event_id,
        request: plan.request_id,
        actor: plan.actor.clone(),
        action: plan.action,
    };
    // Do not hold request locks across the private network/Gold redirect.
    // The broker revalidates the grant and the current PDF parent.
    tx.commit().await.map_err(SigningError::internal)?;
    let begun = installed
        .broker
        .begin(&scope, plan)
        .await
        .map_err(broker_error)?;
    redirect_reply(begun, scope, authority)
}

fn receipt_ids_match(
    scope: &ProductApprovalScope,
    received: &ProductApprovalScope,
    payload: Uuid,
    artifact: Option<Uuid>,
) -> bool {
    scope == received
        && !payload.is_nil()
        && match (scope.action.document(), artifact) {
            (DocumentKind::NoDocument, None) => true,
            (DocumentKind::Eml | DocumentKind::Pdf, Some(id)) => {
                !id.is_nil() && id != payload
            }
            _ => false,
        }
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/v2/<tenant>/finish",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn finish(
    tenant: &str,
    bearer: ProductBearer,
    installed: &State<ProductApprovalInstallation>,
    services: &State<HarvestServices>,
    input: Json<ProductFinish>,
) -> SigningReply<ProductReply> {
    let (tenant, actor, authority) = authenticate(installed, tenant, &bearer)?;
    let bounded = |value: &str, max: usize| {
        !value.is_empty()
            && value.len() <= max
            && value.bytes().all(|b| b.is_ascii_graphic())
    };
    if input.request_id.is_nil()
        || input.attempt.is_nil()
        || !bounded(&input.state, 512)
        || !bounded(&input.code, MAX_OIDC_CODE)
    {
        return Err(SigningError::invalid("Invalid approval return."));
    }
    let mut client = client(services).await?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let request =
        reauthorize_operational_approval(&tx, &actor, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    let scope = ProductApprovalScope {
        tenant,
        event: request.election_event_id,
        request: request.id,
        actor: actor.user_id.clone(),
        action: request.action,
    };
    tx.commit().await.map_err(SigningError::internal)?;
    let step = installed
        .broker
        .finish(&scope, input.attempt, &input.state, &input.code)
        .await
        .map_err(broker_error)?;
    settle_broker_result(
        &mut client,
        services,
        &actor,
        scope,
        authority,
        step,
        false,
    )
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductReconcile {
    request_id: Uuid,
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/operational-signing/v2/<tenant>/reconcile",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn reconcile(
    tenant: &str,
    bearer: ProductBearer,
    installed: &State<ProductApprovalInstallation>,
    services: &State<HarvestServices>,
    input: Json<ProductReconcile>,
) -> SigningReply<ProductReply> {
    let (tenant, actor, authority) = authenticate(installed, tenant, &bearer)?;
    if input.request_id.is_nil() {
        return Err(SigningError::invalid("Invalid approval request."));
    }
    let mut client = client(services).await?;
    let tx = client.transaction().await.map_err(SigningError::internal)?;
    let request =
        reauthorize_operational_approval(&tx, &actor, tenant, input.request_id)
            .await
            .map_err(signing_failure)?;
    let scope = ProductApprovalScope {
        tenant,
        event: request.election_event_id,
        request: request.id,
        actor: actor.user_id.clone(),
        action: request.action,
    };
    tx.commit().await.map_err(SigningError::internal)?;
    let step = installed
        .broker
        .recover(&scope)
        .await
        .map_err(broker_error)?;
    settle_broker_result(
        &mut client,
        services,
        &actor,
        scope,
        authority,
        step,
        true,
    )
    .await
}

async fn settle_broker_result(
    client: &mut deadpool_postgres::Client,
    services: &HarvestServices,
    actor: &SigningCaller,
    scope: ProductApprovalScope,
    authority: &ProductTenantAuthority,
    step: ProductBrokerStep,
    recovering: bool,
) -> SigningReply<ProductReply> {
    match step {
        ProductBrokerStep::Next(next)
            if !recovering
                && scope.action.document() != DocumentKind::NoDocument =>
        {
            redirect_reply(next, scope, authority)
        }
        ProductBrokerStep::Complete {
            scope: received,
            payload_challenge,
            artifact_challenge,
        } => {
            if !receipt_ids_match(
                &scope,
                &received,
                payload_challenge,
                artifact_challenge,
            ) {
                return Err(SigningError::forbidden(
                    "The device proof is for another approval.",
                ));
            }
            approve_operational(
                client,
                &services.signing,
                actor,
                scope.tenant,
                &OperationalApprovalInput {
                    request_id: scope.request,
                    approval_challenge: payload_challenge,
                    artifact_challenge,
                },
                Utc::now(),
            )
            .await
            .map_err(signing_failure)?;
            Ok(Json(ProductReply::Recorded { scope }))
        }
        ProductBrokerStep::Ready if recovering => {
            Ok(Json(ProductReply::Ready { scope }))
        }
        ProductBrokerStep::Reconcile
        | ProductBrokerStep::Ready
        | ProductBrokerStep::Next(_) => {
            Ok(Json(ProductReply::Reconcile { scope }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_all_actions_reject_wrong_actor_scope_and_missing_artifact() {
        let payload = Uuid::from_u128(1);
        let document = Uuid::from_u128(2);
        for action in SigningAction::iter() {
            let scope = ProductApprovalScope {
                tenant: Uuid::from_u128(3),
                event: Uuid::from_u128(4),
                request: Uuid::from_u128(5),
                actor: "trusted-official".into(),
                action,
            };
            let material = (action.document() != DocumentKind::NoDocument)
                .then_some(document);
            assert!(receipt_ids_match(&scope, &scope, payload, material));
            assert!(!receipt_ids_match(&scope, &scope, payload, Some(payload)));
            assert!(!receipt_ids_match(&scope, &scope, Uuid::nil(), material));
            assert!(!receipt_ids_match(
                &scope,
                &ProductApprovalScope {
                    actor: "wrong".into(),
                    ..scope.clone()
                },
                payload,
                material
            ));
            assert!(!receipt_ids_match(
                &scope,
                &ProductApprovalScope {
                    event: Uuid::from_u128(9),
                    ..scope.clone()
                },
                payload,
                material
            ));
            assert!(!receipt_ids_match(
                &scope,
                &scope,
                payload,
                if material.is_some() {
                    None
                } else {
                    Some(document)
                }
            ));
        }
    }

    #[test]
    fn production_never_accepts_loopback_or_cleartext_gold_redirects() {
        for url in [
            "http://login.example.test/realms/tenant-abc/protocol/openid-connect/auth",
            "https://localhost/realms/tenant-abc/protocol/openid-connect/auth",
            "https://localhost./realms/tenant-abc/protocol/openid-connect/auth",
            "https://issuer.localhost/realms/tenant-abc/protocol/openid-connect/auth",
            "https://[::ffff:127.0.0.1]/realms/tenant-abc/protocol/openid-connect/auth",
            "https://127.0.0.2/realms/tenant-abc/protocol/openid-connect/auth",
            "https://[::1]/realms/tenant-abc/protocol/openid-connect/auth",
            "https://alice@login.example.test/realms/tenant-abc/protocol/openid-connect/auth",
        ] {
            assert!(!production_https(&reqwest::Url::parse(url).unwrap()), "{url}");
        }
    }
}
