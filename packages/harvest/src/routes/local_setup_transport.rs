// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Outbound setup adapters. Only the protected local installation can construct these clients.
use super::super::{local::protected, matches_pushed_redirect};
use super::{
    BrokerScope, LocalInitializePost, ReleasedAttempt, SigningError,
    SETUP_CLIENT,
};
use crate::routes::operational_initialize_broker::exact_body;
use crate::services::operational_portal_auth::OperationalPortalAuth;
use deadpool_postgres::{Manager, Pool};
use openssl::x509::{X509Req, X509};
use reqwest::{Certificate, Client, Identity, StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{net::SocketAddrV4, path::Path, time::Duration};
use tokio_postgres::NoTls;
use uuid::Uuid;

const GOLD: &str = "/private/v1/operational-signing/initialize/self-setup/";
const GENERATE: &str = "/private/v1/operational-signing/setup/generate";

#[derive(Serialize)]
pub(super) struct Binding<'a> {
    installation: Uuid,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
    publication: Uuid,
    request: Uuid,
    official: Uuid,
    payload_sha256: &'a str,
}

#[derive(Serialize)]
pub(super) struct Delivery<'a> {
    binding: Binding<'a>,
    attempt: Uuid,
}

impl<'a> Delivery<'a> {
    pub(super) fn new(
        pin: &LocalInitializePost,
        scope: &'a BrokerScope,
        attempt: Uuid,
    ) -> Self {
        let binding = Binding {
            installation: pin.installation,
            tenant: scope.tenant,
            event: scope.event,
            post: pin.post,
            publication: pin.publication,
            request: scope.request,
            official: scope.official,
            payload_sha256: &scope.payload_sha256,
        };
        Self { binding, attempt }
    }
}

fn unavailable() -> SigningError {
    SigningError::conflict(
        "Signing setup delivery could not be confirmed. Check setup status.",
    )
}

struct Peer {
    client: Client,
    origin: Url,
}

fn spki(pem: &[u8]) -> Option<String> {
    let key = X509::from_pem(pem).ok()?.public_key().ok()?;
    Some(hex::encode(Sha256::digest(key.public_key_to_der().ok()?)))
}

impl Peer {
    fn load(
        root: &Path,
        name: &str,
        addr: SocketAddrV4,
        port: u16,
        pin: &str,
    ) -> Option<Self> {
        if !addr.ip().is_private() || addr.port() != port {
            return None;
        }
        let file = |suffix: &str, private| {
            protected(
                &root.join(format!("harvest/setup/{name}-{suffix}")),
                private,
            )
        };
        let cert = file("client.pem", false)?;
        if spki(&cert)?.as_str() != pin {
            return None;
        }
        let mut key = file("client-key.pem", true)?;
        let identity = Identity::from_pkcs8_pem(&cert, &key);
        key.fill(0);
        let authority = Certificate::from_pem(&file("ca.pem", false)?).ok()?;
        let client = Client::builder()
            .use_native_tls()
            .https_only(true)
            .http1_only()
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .tls_built_in_root_certs(false)
            .add_root_certificate(authority)
            .identity(identity.ok()?)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(0)
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .build()
            .ok()?;
        Some(Self {
            client,
            origin: Url::parse(&format!("https://{addr}/")).ok()?,
        })
    }

    async fn post(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, SigningError> {
        let url = self.origin.join(path).map_err(|_| unavailable())?;
        self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|_| unavailable())
    }
}

pub(super) struct Gold(Peer);
pub(super) struct Device(Peer);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nonce {
    nonce_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pushed {
    redirect: String,
}

impl Gold {
    pub(super) async fn nonce(
        &self,
        body: &Delivery<'_>,
    ) -> Result<String, SigningError> {
        let reply = self.0.post(&format!("{GOLD}nonce"), body).await?;
        let out: Nonce =
            exact_body(reply, false).await.map_err(|_| unavailable())?;
        if out.nonce_sha256.len() != 64
            || !out
                .nonce_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(unavailable());
        }
        Ok(out.nonce_sha256)
    }

    pub(super) async fn par(
        &self,
        body: &Delivery<'_>,
        endpoint: &Url,
    ) -> Result<String, SigningError> {
        let reply = self.0.post(&format!("{GOLD}par"), body).await?;
        let out: Pushed =
            exact_body(reply, false).await.map_err(|_| unavailable())?;
        if out.redirect.len() > 2048 {
            return Err(unavailable());
        }
        let location = Url::parse(&out.redirect).map_err(|_| unavailable())?;
        if location.as_str() != out.redirect
            || !matches_pushed_redirect(&location, endpoint, SETUP_CLIENT)
        {
            return Err(unavailable());
        }
        Ok(out.redirect)
    }

    pub(super) async fn confirm(
        &self,
        body: &Delivery<'_>,
        state: &str,
        code: &str,
    ) -> Result<(), SigningError> {
        #[derive(Serialize)]
        struct Callback<'a, 'b> {
            #[serde(flatten)]
            delivery: &'a Delivery<'b>,
            state: &'a str,
            code: &'a str,
        }
        let reply = self
            .0
            .post(
                &format!("{GOLD}confirm"),
                &Callback {
                    delivery: body,
                    state,
                    code,
                },
            )
            .await?;
        if reply.status() != StatusCode::NO_CONTENT {
            return Err(unavailable());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DeviceDelivery {
    installation: Uuid,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
    publication: Uuid,
    action: String,
    request: Uuid,
    official: Uuid,
    payload_sha256: String,
    challenge: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Enrollment {
    enrollment_id: Uuid,
    binding: DeviceDelivery,
    spki_sha256: String,
    public_csr_pem: String,
}

impl Device {
    /// The receiver alone claims the 2200 fence and commits before token access.
    /// This single POST has no fallback, HTTP redirect, or application retry.
    pub(super) async fn deliver(
        &self,
        pin: &LocalInitializePost,
        released: &ReleasedAttempt,
    ) -> Result<(), SigningError> {
        let scope = &released.scope;
        let delivery = DeviceDelivery {
            installation: pin.installation,
            tenant: scope.tenant,
            event: scope.event,
            post: pin.post,
            publication: pin.publication,
            action: "initialize-voting".into(),
            request: scope.request,
            official: scope.official,
            payload_sha256: scope.payload_sha256.clone(),
            challenge: released.attempt,
        };
        let response = self.0.post(GENERATE, &delivery).await?;
        let enrollment: Enrollment = exact_body(response, true)
            .await
            .map_err(|_| unavailable())?;
        if enrollment.enrollment_id.is_nil() || enrollment.binding != delivery {
            return Err(unavailable());
        }
        let csr = X509Req::from_pem(enrollment.public_csr_pem.as_bytes())
            .map_err(|_| unavailable())?;
        let key = csr.public_key().map_err(|_| unavailable())?;
        let spki = key.public_key_to_der().map_err(|_| unavailable())?;
        if hex::encode(Sha256::digest(spki)) != enrollment.spki_sha256
            || !csr.verify(&key).map_err(|_| unavailable())?
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    installation: Uuid,
    setup_client: String,
    authorization_endpoint: String,
    registered_callback: String,
    database: String,
    gold_peer: SocketAddrV4,
    device_peer: SocketAddrV4,
    gold_client_spki_sha256: String,
    device_client_spki_sha256: String,
}

pub(crate) struct SetupServices {
    pub(super) pool: Pool,
    pub(super) gold: Gold,
    pub(super) device: Device,
}

impl SetupServices {
    pub(in crate::routes::operational_initialize) async fn load(
        root: &Path,
        auth: &OperationalPortalAuth,
        pin: &LocalInitializePost,
        app_database: &str,
    ) -> Option<Self> {
        let bytes =
            protected(&root.join("harvest/setup/registration.json"), false)?;
        let cfg: Registration = serde_json::from_slice(&bytes).ok()?;
        if cfg.installation != pin.installation
            || pin.profile != "softhsm-test"
            || cfg.setup_client != SETUP_CLIENT
            || pin.approval_client == SETUP_CLIENT
            || cfg.database != app_database
            || cfg.database.is_empty()
            || cfg.database.len() > 63
            || cfg.gold_peer == cfg.device_peer
            || cfg.gold_client_spki_sha256 == cfg.device_client_spki_sha256
        {
            return None;
        }
        let (tenant, endpoint) = auth.local_approval_endpoint(
            &cfg.authorization_endpoint,
            &cfg.registered_callback,
            SETUP_CLIENT,
        )?;
        if tenant != pin.tenant || endpoint != pin.authorization {
            return None;
        }
        let approval =
            spki(&protected(&root.join("harvest/broker-client.pem"), false)?)?;
        if approval == cfg.gold_client_spki_sha256
            || approval == cfg.device_client_spki_sha256
        {
            return None;
        }
        let gold = Gold(Peer::load(
            root,
            "gold",
            cfg.gold_peer,
            22443,
            &cfg.gold_client_spki_sha256,
        )?);
        let device = Device(Peer::load(
            root,
            "device",
            cfg.device_peer,
            22443,
            &cfg.device_client_spki_sha256,
        )?);
        let socket = root.join("postgres");
        if socket.canonicalize().ok()? != socket || !socket.is_dir() {
            return None;
        }
        let mut password =
            protected(&root.join("harvest/setup/database-password"), true)?;
        let mut db = tokio_postgres::Config::new();
        let line = password.strip_suffix(b"\n").unwrap_or(&password);
        if line.is_empty()
            || line.iter().any(|b| matches!(b, 0 | b'\r' | b'\n'))
        {
            password.fill(0);
            return None;
        }
        db.host_path(&socket)
            .port(5432)
            .dbname(&cfg.database)
            .user("felix3_local_setup_portal")
            .password(line)
            .options("-c search_path=pg_catalog")
            .connect_timeout(Duration::from_secs(3));
        password.fill(0);
        let pool = Pool::builder(Manager::new(db, NoTls))
            .max_size(4)
            .build()
            .ok()?;
        let client = pool.get().await.ok()?;
        let valid: bool = client.query_one("SELECT current_user = session_user
            AND session_user = 'felix3_local_setup_portal' AND current_database()::text = $1 AND inet_client_addr() IS NULL
            AND has_schema_privilege(current_user,'felix3_initialize_setup','USAGE')
            AND has_type_privilege(current_user,'felix3_initialize_setup.binding','USAGE')
            AND (SELECT array_agg(p.proname::text ORDER BY p.proname) = ARRAY[
                'begin_setup','bind_browser','browser_attempt','release','status','take_delivery']
                FROM pg_proc p WHERE p.pronamespace = 'felix3_initialize_setup'::regnamespace
                AND has_function_privilege(current_user,p.oid,'EXECUTE'))", &[&cfg.database]).await.ok()?.try_get(0).ok()?;
        valid.then_some(Self { pool, gold, device })
    }
}
