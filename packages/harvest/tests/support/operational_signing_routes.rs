// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The default-off operational signing candidate routes, mounted on a local
//! Rocket client because the application does not mount them: the portal
//! token guard, the official's setup status, the Initialize voting ceremony
//! through an in-process broker that drives the real ledger contract, the
//! private broker client against a local HTTPS stand-in and the private CA
//! assessment server's peer pin.
//!
//! TEST FICTION: there is no hardware and no identity provider here. The
//! officials' keys are software P-256 keys of the test PKI, enrolled under
//! a device profile by calling the ledger's contract functions as the test
//! database's superuser, which also plays the broker and the device worker.
//! Each test works in fresh tenants.

use crate::route_services::{RowOnlyExports, Services};
use crate::routes::operational_initialize::{
    BrokerFailure, BrokerFuture, BrokerRedirect, BrokerScope, InitializeBroker,
    OperationalInitializeBroker, Settled,
};
use crate::routes::operational_initialize_broker::PrivateInitializeBroker;
use crate::services::operational_portal_auth::OperationalPortalAuth;
use crate::test_claims::{signing, Claims};
use chrono::Utc;
use deadpool_postgres::Pool;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::ssl::{SslAcceptor, SslMethod, SslVerifyMode};
use openssl::x509::extension::{
    BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName,
};
use openssl::x509::{X509Builder, X509NameBuilder, X509};
use rocket::config::LogLevel;
use rocket::http::{ContentType, Header as HttpHeader, Status};
use rocket::local::asynchronous::Client;
use sequent_core::signing::{
    RequesterSigning, SigningAction, SigningRequirement, SigningRule,
    StaffCertificateRegistration,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;
use windmill::postgres::signing::{
    insert_staff_certificate, upsert_signing_rule, NewStaffCertificate,
};
use windmill::services::signing::approve::{NoDocumentSigner, SigningServices};
use windmill::services::signing::certificates::{
    CertificateIdentity, OpensslCertificateVerifier,
};
use windmill::services::signing::crl::{
    refresh_chain_crls, refresh_event_crls, CrlFetcher,
};
use windmill::services::signing::directory::UserDirectory;
use windmill::services::signing::executors::SigningExecutorRegistry;
use windmill::services::signing::guard::{
    guard, GuardOutcome, GuardRequest, RequestScope,
};
use windmill::services::signing::issuers::import_staff_issuers;
use windmill::services::signing::log::Actor;
use windmill::services::signing::{Allowance, SigningCaller};

// Formatted by windmill, which owns it (harvest's rustfmt.toml is narrower).
#[rustfmt::skip]
#[path = "../../../windmill/tests/support/signing_pki.rs"]
mod signing_pki;
use signing_pki::{Issued, Pki, CRL_URL, ROOT_CRL_URL};

const STATUS: &str = "/operational-signing/initialize/status";
const CANCEL: &str = "/operational-signing/initialize/cancel";
const RECONCILE: &str = "/operational-signing/initialize/reconcile";
const START: &str = "/operational-signing/initialize/start";
const FINISH: &str = "/operational-signing/initialize/finish";
const SETUP: &str = "/operational-signing/setup/status";
/// The Admin Portal client and audience the portal guard is installed with.
const PORTAL: &str = "admin-portal";
/// The dedicated step-up client the broker is installed with.
const OIDC_CLIENT: &str = "operational-initialize";
/// The verified issuer the broker records; nothing connects to it.
const ISSUER: &str = "https://idp.invalid/realms/operational-signing";
const KEYCLOAK: &str = "https://identity.invalid";
const PAR_PREFIX: &str = "urn:ietf:params:oauth:request_uri:";
const LABEL: &str = "initialize-post";
const UNAVAILABLE: &str = "Device signing for this request is not available.";

fn sha(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

fn binding(attempt: Uuid) -> String {
    sha(&format!("binding of {attempt}"))
}

fn nonce(attempt: Uuid) -> String {
    sha(&format!("nonce of {attempt}"))
}

fn quiet() -> rocket::Config {
    rocket::Config {
        log_level: LogLevel::Off,
        cli_colors: false,
        ..Default::default()
    }
}

/// The realm's published test keys, as installation provisions them.
fn realm_keys() -> JwkSet {
    serde_json::from_value(signing::key_set()).expect("the test key set")
}

fn portal_auth(tenant: Uuid) -> OperationalPortalAuth {
    OperationalPortalAuth::new(
        tenant,
        signing::issuer_of(&tenant.to_string()),
        PORTAL.into(),
        PORTAL.into(),
        realm_keys(),
    )
    .expect("a valid portal guard")
}

/// The claims of an Admin Portal token of `user` in `tenant`.
fn portal_claims(
    tenant: Uuid,
    user: &str,
    roles: &[Permissions],
    labels: &[&str],
) -> Value {
    let now = Utc::now().timestamp();
    let roles: Vec<String> = roles.iter().map(ToString::to_string).collect();
    json!({
        "exp": signing::EXPIRY, "iat": now, "auth_time": now,
        "jti": "synthetic", "iss": signing::issuer_of(&tenant.to_string()),
        "aud": PORTAL, "sub": user, "typ": "Bearer", "azp": PORTAL,
        "acr": "1", "allowed-origins": [], "scope": "openid",
        "email_verified": false, "preferred_username": format!("user-{user}"),
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "user",
            "x-hasura-tenant-id": tenant.to_string(),
            "x-hasura-user-id": user,
            "x-hasura-allowed-roles": roles,
            "x-hasura-permission-labels": format!("{{{}}}", labels.join(",")),
        }
    })
}

fn bearer(claims: &Value) -> String {
    format!("Bearer {}", signing::token(claims))
}

/// POSTs `body` with these `Authorization` values; the status and the JSON
/// body, if there is one.
async fn send(
    client: &Client,
    path: &'static str,
    authorization: &[&str],
    body: &Value,
) -> (Status, Value) {
    let mut request = client
        .post(path)
        .header(ContentType::JSON)
        .body(body.to_string());
    for value in authorization {
        request =
            request.header(HttpHeader::new("Authorization", value.to_string()));
    }
    let response = request.dispatch().await;
    let status = response.status();
    let body = response.into_string().await.unwrap_or_default();
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

fn message(body: &Value) -> &str {
    body["message"].as_str().unwrap_or_default()
}

/// Serves the PKI's revocation lists.
struct Lists;

#[rocket::async_trait]
impl CrlFetcher for Lists {
    async fn fetch(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let pki = Pki::get();
        match url {
            CRL_URL => Ok(pki.individual_crl()),
            ROOT_CRL_URL => Ok(pki.root_crl()),
            _ => anyhow::bail!("connection refused"),
        }
    }
}

struct Names;

#[rocket::async_trait]
impl UserDirectory for Names {
    async fn display_names(
        &self,
        _: Uuid,
        user_ids: &[String],
    ) -> anyhow::Result<HashMap<String, String>> {
        Ok(user_ids
            .iter()
            .map(|user| (user.clone(), format!("Display {user}")))
            .collect())
    }
}

/// A third P-256 signer of the PKI's individual CA.
fn third_signer() -> &'static Issued {
    static SIGNER: OnceLock<Issued> = OnceLock::new();
    SIGNER.get_or_init(|| {
        let subject = signing_pki::person("Test Official", "PH-0020");
        signing_pki::issued(
            &signing_pki::Spec::signer(&subject, 1020),
            signing_pki::ec_key(),
            Some(&Pki::get().individual_ca),
        )
    })
}

/// What the broker and the device need of an official's enrolled key.
struct OfficialKey {
    user: String,
    signer: &'static Issued,
    fingerprint: String,
    spki: String,
    profile: &'static str,
}

struct Official {
    id: Uuid,
    key: Arc<OfficialKey>,
    certificate: Uuid,
    enrollment: Option<Uuid>,
}

/// A tenant with one event and one labelled Post, an initialize-voting rule
/// of `signatures` and the test PKI's issuers and lists, on the migrated
/// test database. Its tenant requires hardware when `hardware`.
struct World {
    services: Services,
    pool: Arc<Pool>,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
}

impl World {
    async fn new(signatures: u16, hardware: bool) -> World {
        let services =
            Services::on_test_database()
                .await
                .with_signing(SigningServices {
                    verifier: Arc::new(
                        OpensslCertificateVerifier::with_directory(Arc::new(
                            Names,
                        )),
                    ),
                    executors: SigningExecutorRegistry::default(),
                    documents: Arc::new(NoDocumentSigner),
                    exports: Arc::new(RowOnlyExports),
                });
        let pool = services.hasura.clone();
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        let post = Uuid::new_v4();
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &tenant.to_string()],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election_event
                 (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &tenant],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, permission_label)
             VALUES ($1, $2, $3, $4)",
            &[&post, &tenant, &event, &LABEL],
        )
        .await
        .unwrap();
        upsert_signing_rule(
            &tx,
            tenant,
            event,
            &SigningRule {
                action: SigningAction::InitializeVoting,
                requirement: SigningRequirement::Required,
                signatures,
                requester_signing: RequesterSigning::NotAllowed,
                expires_minutes: Some(60),
                revision: 0,
            },
            0,
            "manager",
            Some("Configuration Manager"),
        )
        .await
        .unwrap()
        .unwrap();
        let pki = Pki::get();
        import_staff_issuers(
            &tx,
            tenant,
            event,
            &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
            &Actor {
                user_id: "officer".into(),
                username: "officer".into(),
            },
            Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
            Utc::now(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        refresh_event_crls(&mut client, tenant, event, &Lists, Utc::now())
            .await
            .unwrap();
        if hardware {
            // A statement of its own: the policy lock is its first lock.
            client
                .execute(
                    "INSERT INTO sequent_backend.operational_signing_required_tenant
                         (tenant_id)
                     VALUES ($1)",
                    &[&tenant],
                )
                .await
                .unwrap();
        }
        World {
            services,
            pool,
            tenant,
            event,
            post,
        }
    }

    /// A waiting initialize-voting request of the Post.
    async fn request(&self) -> Uuid {
        let operator = Claims::new(&self.tenant.to_string(), "operator")
            .username("operator")
            .roles([Permissions::ELECTION_STATE_WRITE])
            .build();
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let outcome = guard(
            &tx,
            &SigningCaller::from_claims(&operator),
            &GuardRequest {
                action: SigningAction::InitializeVoting,
                scope: RequestScope {
                    tenant_id: self.tenant,
                    election_event_id: self.event,
                    election_id: Some(self.post),
                    area_id: None,
                    trustee_id: None,
                    subject_key: None,
                },
                subject: json!({"channel": "online"}),
                document: None,
                config_revision: None,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let GuardOutcome::SigningRequired(summary) = outcome else {
            panic!("initialize-voting needs signatures");
        };
        summary.id
    }

    async fn make_overdue(&self, request: Uuid) {
        self.execute(
            "UPDATE sequent_backend.signing_request
             SET expires_at = clock_timestamp() - interval '1 second'
             WHERE id = $1",
            &[&request],
        )
        .await;
    }

    async fn execute(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(sql, params)
            .await
            .unwrap();
    }

    /// A new official whose `signer` certificate is registered for the
    /// event, with the lists of its chain; no key is enrolled yet.
    async fn official(
        &self,
        signer: &'static Issued,
        profile: &'static str,
    ) -> Official {
        let id = Uuid::new_v4();
        let user = id.to_string();
        let identity = CertificateIdentity::of(&signer.cert, &[]).unwrap();
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let certificate = insert_staff_certificate(
            &tx,
            &NewStaffCertificate {
                tenant_id: self.tenant,
                election_event_id: self.event,
                user_id: user.clone(),
                username: format!("user-{user}"),
                election_id: Some(self.post),
                fingerprint_sha256: identity.fingerprint_sha256.clone(),
                spki_sha256: identity.spki_sha256.clone(),
                holder_sha256: identity.holder_sha256.clone(),
                serial: identity.serial.clone(),
                subject: identity.subject.clone(),
                issuer: identity.issuer.clone(),
                not_before: identity.not_before,
                not_after: identity.not_after,
                pem: identity.pem.clone(),
                registration: StaffCertificateRegistration::SecurityOfficer,
                linked_to: None,
                registered_by: "officer".into(),
                user_display_name: None,
                registered_by_name: None,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        refresh_chain_crls(
            &mut client,
            self.tenant,
            self.event,
            &[signer.pem()],
            &Lists,
            Utc::now(),
        )
        .await
        .unwrap();
        Official {
            id,
            key: Arc::new(OfficialKey {
                user,
                signer,
                fingerprint: identity.fingerprint_sha256,
                spki: identity.spki_sha256,
                profile,
            }),
            certificate: certificate.id,
            enrollment: None,
        }
    }

    /// The device verifier enrolls the official's key.
    async fn enroll_key(&self, official: &mut Official) {
        let key = &official.key;
        let enrollment: Option<Uuid> = self
            .pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT sequent_backend.operational_signing_enroll_official(
                     $1, $2, $3, 1, $4, $5, true, $6)",
                &[
                    &self.tenant,
                    &key.user,
                    &format!("official/{}/{}", self.tenant, key.user),
                    &key.spki,
                    &key.profile,
                    &sha(&format!("device evidence of {}", key.user)),
                ],
            )
            .await
            .unwrap()
            .get(0);
        official.enrollment = Some(enrollment.expect("the key is enrolled"));
    }

    /// The Security Officer attaches the certificate to the enrollment.
    async fn attach(&self, official: &Official) {
        let key = &official.key;
        let pem = key.signer.pem();
        let attached: bool = self
            .pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT sequent_backend.operational_signing_bind_official_certificate(
                     $1, $2, $3, $4, $5, $6, $7)",
                &[
                    &self.tenant,
                    &self.event,
                    &key.user,
                    &official.enrollment,
                    &official.certificate,
                    &key.fingerprint,
                    &vec![pem],
                ],
            )
            .await
            .unwrap()
            .get(0);
        assert!(attached, "the certificate is attached");
    }

    /// An official with an enrolled key and an attached certificate.
    async fn enrolled(
        &self,
        signer: &'static Issued,
        profile: &'static str,
    ) -> Official {
        let mut official = self.official(signer, profile).await;
        self.enroll_key(&mut official).await;
        self.attach(&official).await;
        official
    }

    async fn scope(&self, official: &Official, request: Uuid) -> BrokerScope {
        let payload: String = self
            .pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT payload_sha256 FROM sequent_backend.signing_request
                 WHERE id = $1",
                &[&request],
            )
            .await
            .unwrap()
            .get(0);
        BrokerScope {
            tenant: self.tenant,
            event: self.event,
            request,
            official: official.id,
            payload_sha256: payload,
        }
    }

    /// The official's Admin Portal token with `roles`.
    fn token(&self, official: Uuid, roles: &[Permissions]) -> String {
        bearer(&portal_claims(
            self.tenant,
            &official.to_string(),
            roles,
            &[LABEL],
        ))
    }

    fn signer_token(&self, official: &Official) -> String {
        self.token(official.id, &[Permissions::SIGN_INITIALIZE_VOTING])
    }

    /// Every candidate route, with the portal guard and `broker` installed.
    async fn client(&self, broker: LedgerBroker) -> Client {
        Client::tracked(
            rocket::build()
                .configure(quiet())
                .manage(self.services.harvest_services())
                .manage(portal_auth(self.tenant))
                .manage(InitializeBroker(Box::new(broker)))
                .mount(
                    "/",
                    routes![
                        crate::routes::operational_initialize::status,
                        crate::routes::operational_initialize::cancel_unused,
                        crate::routes::operational_initialize::reconcile,
                        crate::routes::operational_initialize::start,
                        crate::routes::operational_initialize::finish,
                        crate::routes::operational_setup::status,
                    ],
                ),
        )
        .await
        .expect("the candidate routes mount")
    }
}

/// The ledger contract's broker steps, as the private orchestrator calls
/// them.
async fn prepare_attempt(
    pool: &Pool,
    scope: &BrokerScope,
    profile: &str,
    attempt: Uuid,
) {
    let prepared: bool = pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT sequent_backend.operational_initialize_prepare(
                 $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &scope.tenant,
                &scope.event,
                &scope.request,
                &scope.official.to_string(),
                &scope.payload_sha256,
                &profile,
                &attempt,
                &ISSUER,
                &nonce(attempt),
                &binding(attempt),
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(prepared, "the broker prepares the challenge");
}

async fn redeem(pool: &Pool, scope: &BrokerScope, attempt: Uuid) {
    let redeemed: bool = pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT sequent_backend.operational_initialize_take_callback(
                 $1, $2, $3, $4, $5, $6)",
            &[
                &scope.tenant,
                &scope.event,
                &scope.request,
                &scope.official.to_string(),
                &attempt,
                &binding(attempt),
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(redeemed, "the broker redeems the callback");
}

/// Reserves the device use of a redeemed attempt; the fence.
async fn reserve(
    pool: &Pool,
    scope: &BrokerScope,
    profile: &str,
    attempt: Uuid,
) -> Uuid {
    // The proof's authentication time must be after the challenge's second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let fence: Option<Uuid> = pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT sequent_backend.operational_initialize_reserve_own_callback(
                 $1, $2, $3, $4, $5, $6, $7, $8,
                 floor(extract(epoch FROM clock_timestamp()))::bigint,
                 clock_timestamp() + interval '1 minute', $9)",
            &[
                &scope.tenant,
                &scope.event,
                &scope.request,
                &scope.official.to_string(),
                &attempt,
                &binding(attempt),
                &nonce(attempt),
                &ISSUER,
                &profile,
            ],
        )
        .await
        .unwrap()
        .get(0);
    fence.expect("the broker reserves the device use")
}

/// The device claims the reservation and settles its signature of the
/// request's canonical payload.
async fn claim_and_settle(
    pool: &Pool,
    scope: &BrokerScope,
    key: &OfficialKey,
    attempt: Uuid,
    fence: Uuid,
) {
    let client = pool.get().await.unwrap();
    let payload: String = client
        .query_one(
            "SELECT canonical_payload FROM sequent_backend.signing_request
             WHERE id = $1",
            &[&scope.request],
        )
        .await
        .unwrap()
        .get(0);
    let user = scope.official.to_string();
    let claimed: bool = client
        .query_one(
            "SELECT sequent_backend.operational_initialize_claim_current_device(
                 $1, $2, $3, $4, $5, $6, $7, $8,
                 sequent_backend.operational_initialize_ca_version($1, $2, $9, $10))",
            &[
                &scope.tenant,
                &scope.event,
                &scope.request,
                &user,
                &attempt,
                &fence,
                &binding(attempt),
                &key.profile,
                &key.fingerprint,
                &key.spki,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(claimed, "the device claims its reservation");
    let settled: bool = client
        .query_one(
            "SELECT sequent_backend.operational_signing_settle_device_from_reference(
                 $1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &scope.tenant,
                &scope.event,
                &scope.request,
                &user,
                &attempt,
                &fence,
                &binding(attempt),
                &key.signer.sign(payload.as_bytes()),
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(settled, "the device settles its receipt");
}

#[derive(Clone)]
enum Begin {
    /// Prepares a real ledger challenge and returns its PAR redirect.
    Ledger,
    Refuse,
    Unavailable,
    NilAttempt,
    /// Returns this redirect for a new attempt the ledger never saw.
    Redirect(String),
}

#[derive(Clone, Copy)]
enum End {
    /// Redeems, reserves, claims and settles the device receipt.
    Settle,
    Reconcile,
    Refuse,
    Unavailable,
    OtherScope,
    NilChallenge,
}

#[derive(Clone)]
struct Script {
    offer: bool,
    begin: Begin,
    end: End,
}

fn script(offer: bool, begin: Begin, end: End) -> Arc<Mutex<Script>> {
    Arc::new(Mutex::new(Script { offer, begin, end }))
}

/// The authorization endpoint of the tenant's realm.
fn authorize_endpoint(tenant: Uuid) -> reqwest::Url {
    reqwest::Url::parse(&format!(
        "{KEYCLOAK}/realms/tenant-{tenant}/protocol/openid-connect/auth"
    ))
    .unwrap()
}

fn par_redirect(tenant: Uuid, query: &str) -> String {
    format!("{}?{query}", authorize_endpoint(tenant))
}

/// An in-process broker that drives the ledger contract for one official.
struct LedgerBroker {
    pool: Arc<Pool>,
    key: Option<Arc<OfficialKey>>,
    script: Arc<Mutex<Script>>,
    authorize: reqwest::Url,
}

impl LedgerBroker {
    fn new(
        w: &World,
        official: Option<&Official>,
        script: &Arc<Mutex<Script>>,
    ) -> Self {
        LedgerBroker {
            pool: w.pool.clone(),
            key: official.map(|official| official.key.clone()),
            script: script.clone(),
            authorize: authorize_endpoint(w.tenant),
        }
    }

    fn current(&self) -> Script {
        self.script.lock().unwrap().clone()
    }

    fn key(&self) -> &OfficialKey {
        self.key.as_deref().expect("the broker serves an official")
    }
}

impl OperationalInitializeBroker for LedgerBroker {
    fn can_offer<'a>(
        &'a self,
        _prepared: &'a BrokerScope,
    ) -> BrokerFuture<'a, bool> {
        let offer = self.current().offer;
        Box::pin(async move { Ok(offer) })
    }

    fn start<'a>(
        &'a self,
        prepared: &'a BrokerScope,
    ) -> BrokerFuture<'a, BrokerRedirect> {
        let begin = self.current().begin;
        Box::pin(async move {
            let attempt = Uuid::new_v4();
            let location = match begin {
                Begin::Ledger => {
                    prepare_attempt(
                        &self.pool,
                        prepared,
                        self.key().profile,
                        attempt,
                    )
                    .await;
                    par_redirect(
                        prepared.tenant,
                        &format!(
                            "client_id={OIDC_CLIENT}&request_uri={PAR_PREFIX}{}",
                            attempt.simple()
                        ),
                    )
                }
                Begin::Refuse => return Err(BrokerFailure::Refused),
                Begin::Unavailable => return Err(BrokerFailure::Unavailable),
                Begin::NilAttempt => {
                    return Ok(BrokerRedirect {
                        attempt: Uuid::nil(),
                        location: authorize_endpoint(prepared.tenant),
                    })
                }
                Begin::Redirect(location) => location,
            };
            Ok(BrokerRedirect {
                attempt,
                location: reqwest::Url::parse(&location).unwrap(),
            })
        })
    }

    fn finish<'a>(
        &'a self,
        prepared: &'a BrokerScope,
        attempt: Uuid,
        _state: &'a str,
        _code: &'a str,
    ) -> BrokerFuture<'a, Settled> {
        let end = self.current().end;
        Box::pin(async move {
            let settled = |scope: BrokerScope, challenge| {
                Ok(Settled {
                    scope,
                    approval_challenge: challenge,
                })
            };
            match end {
                End::Settle => {
                    let key = self.key();
                    redeem(&self.pool, prepared, attempt).await;
                    let fence =
                        reserve(&self.pool, prepared, key.profile, attempt)
                            .await;
                    claim_and_settle(&self.pool, prepared, key, attempt, fence)
                        .await;
                    settled(prepared.clone(), attempt)
                }
                End::Reconcile => Err(BrokerFailure::Reconcile),
                End::Refuse => Err(BrokerFailure::Refused),
                End::Unavailable => Err(BrokerFailure::Unavailable),
                End::OtherScope => settled(
                    BrokerScope {
                        official: Uuid::new_v4(),
                        ..prepared.clone()
                    },
                    attempt,
                ),
                End::NilChallenge => settled(prepared.clone(), Uuid::nil()),
            }
        })
    }

    fn authorization_endpoint(&self) -> &reqwest::Url {
        &self.authorize
    }

    fn oidc_client_id(&self) -> &str {
        OIDC_CLIENT
    }
}

fn election_page(post: Uuid) -> Value {
    json!({"path": "election-show", "panel": "publish", "election_id": post})
}

fn event_page() -> Value {
    json!({"path": "event-show", "panel": "signatures-requests"})
}

fn callback(request: Uuid, attempt: Uuid) -> Value {
    json!({
        "request_id": request, "attempt": attempt,
        "state": "s".repeat(43), "code": "authorization-code",
    })
}

// --- The portal token guard -------------------------------------------------

/// The setup status route on pools whose connections are refused, so a
/// token the guard accepts reaches the route and fails there.
async fn guarded_client(auth: Option<OperationalPortalAuth>) -> Client {
    let mut rocket = rocket::build()
        .configure(quiet())
        .manage(Services::without_database().harvest_services())
        .mount("/", routes![crate::routes::operational_setup::status]);
    if let Some(auth) = auth {
        rocket = rocket.manage(auth);
    }
    Client::tracked(rocket)
        .await
        .expect("the setup route mounts")
}

#[rocket::async_test]
async fn the_portal_guard_is_installed_only_for_an_exact_https_realm() {
    let tenant = Uuid::new_v4();
    let issuer = signing::issuer_of(&tenant.to_string());
    let build = |tenant: Uuid, issuer: &str, client: &str, audience: &str| {
        OperationalPortalAuth::new(
            tenant,
            issuer.into(),
            client.into(),
            audience.into(),
            realm_keys(),
        )
        .is_some()
    };
    assert!(build(tenant, &issuer, PORTAL, PORTAL));
    let other_realm = signing::issuer_of(&Uuid::new_v4().to_string());
    let http = issuer.replace("https://", "http://");
    let query = format!("{issuer}?realm=1");
    let fragment = format!("{issuer}#realm");
    let user = issuer.replace("https://", "https://user@");
    let password = issuer.replace("https://", "https://:secret@");
    for (tenant, issuer, client, audience) in [
        (Uuid::nil(), issuer.as_str(), PORTAL, PORTAL),
        (tenant, "not a url", PORTAL, PORTAL),
        (tenant, http.as_str(), PORTAL, PORTAL),
        (tenant, other_realm.as_str(), PORTAL, PORTAL),
        (tenant, "https:///realms/tenant-x", PORTAL, PORTAL),
        (tenant, query.as_str(), PORTAL, PORTAL),
        (tenant, fragment.as_str(), PORTAL, PORTAL),
        (tenant, user.as_str(), PORTAL, PORTAL),
        (tenant, password.as_str(), PORTAL, PORTAL),
        (tenant, issuer.as_str(), "", PORTAL),
        (tenant, issuer.as_str(), PORTAL, ""),
    ] {
        assert!(!build(tenant, issuer, client, audience), "{issuer}");
    }
    let mut keys = realm_keys();
    keys.keys.clear();
    assert!(OperationalPortalAuth::new(
        tenant,
        issuer.clone(),
        PORTAL.into(),
        PORTAL.into(),
        keys
    )
    .is_none());
    let mut keys = realm_keys();
    let key = keys.keys[0].clone();
    keys.keys = vec![key; 33];
    assert!(OperationalPortalAuth::new(
        tenant,
        issuer,
        PORTAL.into(),
        PORTAL.into(),
        keys
    )
    .is_none());
}

#[rocket::async_test]
async fn the_portal_guard_accepts_only_a_verified_admin_portal_token() {
    let tenant = Uuid::new_v4();
    let user = Uuid::new_v4().to_string();
    let body = json!({"election_event_id": Uuid::new_v4()});
    let claims = portal_claims(
        tenant,
        &user,
        &[Permissions::SIGNING_CERTIFICATES_READ],
        &[],
    );
    let good = bearer(&claims);

    // Without an installed guard the route is unavailable.
    let client = guarded_client(None).await;
    assert_eq!(
        send(&client, SETUP, &[&good], &body).await.0,
        Status::ServiceUnavailable
    );

    let client = guarded_client(Some(portal_auth(tenant))).await;
    // The guard lets the verified token through; the route then needs its
    // permission and its database.
    assert_eq!(
        send(&client, SETUP, &[&good], &body).await.0,
        Status::InternalServerError
    );
    let unpermitted = bearer(&portal_claims(tenant, &user, &[], &[]));
    let (status, refusal) = send(&client, SETUP, &[&unpermitted], &body).await;
    assert_eq!(status, Status::Forbidden);
    assert!(
        message(&refusal).starts_with("Missing permission"),
        "{refusal}"
    );

    let changed = |change: &dyn Fn(&mut Value)| {
        let mut claims = claims.clone();
        change(&mut claims);
        bearer(&claims)
    };
    let other_tenant = Uuid::new_v4().to_string();
    let rejected = [
        changed(&|c| c["aud"] = json!("account")),
        changed(&|c| {
            c.as_object_mut().unwrap().remove("aud");
        }),
        changed(&|c| c["azp"] = json!("voting-portal")),
        changed(&|c| c["iss"] = json!(signing::issuer_of(&other_tenant))),
        changed(&|c| {
            c["https://hasura.io/jwt/claims"]["x-hasura-tenant-id"] =
                json!(other_tenant)
        }),
        changed(&|c| {
            c["sub"] = json!("officer");
            c["https://hasura.io/jwt/claims"]["x-hasura-user-id"] =
                json!("officer");
        }),
        changed(&|c| {
            c["sub"] = json!(Uuid::nil());
            c["https://hasura.io/jwt/claims"]["x-hasura-user-id"] =
                json!(Uuid::nil());
        }),
        changed(&|c| {
            c["https://hasura.io/jwt/claims"]["x-hasura-user-id"] =
                json!(Uuid::new_v4())
        }),
        changed(&|c| c["typ"] = json!("ID")),
        changed(&|c| c["exp"] = json!(Utc::now().timestamp() - 60)),
        changed(&|c| c["nbf"] = json!(Utc::now().timestamp() + 600)),
        // Signed by the key, but under headers the guard does not accept.
        format!(
            "Bearer {}",
            signing::token_with(&Header::new(Algorithm::RS256), &claims)
        ),
        format!("Bearer {}", {
            let mut header = Header::new(Algorithm::RS256);
            header.kid = Some("another-key".into());
            signing::token_with(&header, &claims)
        }),
        format!("Bearer {}", {
            let mut header = Header::new(Algorithm::RS256);
            header.kid = Some("  ".into());
            signing::token_with(&header, &claims)
        }),
        format!("Bearer {}", {
            let mut header = Header::new(Algorithm::HS256);
            header.kid = Some("test-key".into());
            encode(&header, &claims, &EncodingKey::from_secret(b"shared"))
                .unwrap()
        }),
        "Bearer not-a-token".to_owned(),
        format!("Bearer {}", "a".repeat(40_000)),
        "Bearer ".to_owned(),
        good.replace("Bearer ", "Basic "),
    ];
    for authorization in &rejected {
        assert_eq!(
            send(&client, SETUP, &[authorization], &body).await.0,
            Status::Unauthorized,
            "{}",
            &authorization[..authorization.len().min(60)]
        );
    }
    assert_eq!(
        send(&client, SETUP, &[], &body).await.0,
        Status::Unauthorized
    );
    assert_eq!(
        send(&client, SETUP, &[&good, &good], &body).await.0,
        Status::Unauthorized
    );

    // Two installed keys with the token's key id are ambiguous.
    let mut keys = realm_keys();
    let key = keys.keys[0].clone();
    keys.keys.push(key);
    let ambiguous = OperationalPortalAuth::new(
        tenant,
        signing::issuer_of(&tenant.to_string()),
        PORTAL.into(),
        PORTAL.into(),
        keys,
    );
    let client = guarded_client(ambiguous).await;
    assert_eq!(
        send(&client, SETUP, &[&good], &body).await.0,
        Status::Unauthorized
    );
}

// --- Setup status -----------------------------------------------------------

#[rocket::async_test]
async fn setup_status_reports_only_the_callers_own_phase() {
    let w = World::new(1, true).await;
    let client = w
        .client(LedgerBroker::new(
            &w,
            None,
            &script(false, Begin::Refuse, End::Refuse),
        ))
        .await;
    let mut official = w.official(&Pki::get().jose, "yubihsm").await;
    let official_id = official.id;
    let reader =
        w.token(official_id, &[Permissions::SIGNING_CERTIFICATES_READ]);
    let event = json!({"election_event_id": w.event});
    let (tenant, election_event) = (w.tenant, w.event);
    let phase = |body: Value| {
        assert_eq!(
            (&body["tenant"], &body["event"], &body["official"]),
            (&json!(tenant), &json!(election_event), &json!(official_id)),
        );
        body["phase"].as_str().unwrap().to_owned()
    };

    let (status, body) = send(&client, SETUP, &[&reader], &event).await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(phase(body), "key-not-enrolled");

    w.enroll_key(&mut official).await;
    let (_, body) = send(&client, SETUP, &[&reader], &event).await;
    assert_eq!(phase(body), "certificate-not-attached");

    w.attach(&official).await;
    let (_, body) = send(&client, SETUP, &[&reader], &event).await;
    assert_eq!(phase(body), "certificate-on-file");

    let retired: bool = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT sequent_backend.operational_signing_retire_official_certificate(
                 $1, $2, $3, $4, $5, $6)",
            &[
                &w.tenant,
                &w.event,
                &official.key.user,
                &official.enrollment,
                &official.certificate,
                &official.key.fingerprint,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(retired);
    let (_, body) = send(&client, SETUP, &[&reader], &event).await;
    assert_eq!(phase(body), "certificate-needs-review");

    // No permission, a nil or unknown event: nothing.
    let unpermitted = w.token(official_id, &[]);
    assert_eq!(
        send(&client, SETUP, &[&unpermitted], &event).await.0,
        Status::Forbidden
    );
    for event in [Uuid::nil(), Uuid::new_v4()] {
        let (status, body) = send(
            &client,
            SETUP,
            &[&reader],
            &json!({"election_event_id": event}),
        )
        .await;
        assert_eq!(status, Status::Forbidden);
        assert_eq!(
            message(&body),
            "Signing setup is not available for this event."
        );
    }
    // Only the event id is accepted.
    assert_eq!(
        send(
            &client,
            SETUP,
            &[&reader],
            &json!({"election_event_id": w.event, "official": Uuid::new_v4()}),
        )
        .await
        .0,
        Status::UnprocessableEntity
    );

    // A legacy tenant has no hardware setup.
    let legacy = World::new(1, false).await;
    let client = legacy
        .client(LedgerBroker::new(
            &legacy,
            None,
            &script(false, Begin::Refuse, End::Refuse),
        ))
        .await;
    let reader =
        legacy.token(Uuid::new_v4(), &[Permissions::SIGNING_CERTIFICATES_READ]);
    assert_eq!(
        send(
            &client,
            SETUP,
            &[&reader],
            &json!({"election_event_id": legacy.event})
        )
        .await
        .0,
        Status::Forbidden
    );
}

// --- The Initialize voting ceremony -----------------------------------------

#[rocket::async_test]
async fn the_ceremony_refuses_legacy_tenants_and_unpermitted_callers() {
    let w = World::new(2, false).await;
    let request = w.request().await;
    let official = Uuid::new_v4();
    let client = w
        .client(LedgerBroker::new(
            &w,
            None,
            &script(true, Begin::Refuse, End::Refuse),
        ))
        .await;
    let signer = w.token(official, &[Permissions::SIGN_INITIALIZE_VOTING]);
    let start = json!({"request_id": request});
    for (path, body) in [
        (STATUS, start.clone()),
        (RECONCILE, start.clone()),
        (
            CANCEL,
            json!({"request_id": request, "attempt": Uuid::new_v4()}),
        ),
        (START, json!({"request_id": request, "page": event_page()})),
        (FINISH, callback(request, Uuid::new_v4())),
    ] {
        let (status, body) = send(&client, path, &[&signer], &body).await;
        assert_eq!(status, Status::Forbidden, "{path}");
        assert_eq!(message(&body), UNAVAILABLE, "{path}");
    }
    let unpermitted =
        w.token(official, &[Permissions::SIGNING_CERTIFICATES_READ]);
    let (status, body) = send(&client, STATUS, &[&unpermitted], &start).await;
    assert_eq!(status, Status::Forbidden);
    assert!(message(&body).starts_with("Missing permission"), "{body}");
}

#[rocket::async_test]
async fn status_offers_a_challenge_only_when_the_broker_can() {
    let w = World::new(2, true).await;
    let request = w.request().await;
    let official = w.enrolled(&Pki::get().jose, "yubihsm").await;
    let script = script(false, Begin::Refuse, End::Refuse);
    let client = w
        .client(LedgerBroker::new(&w, Some(&official), &script))
        .await;
    let token = w.signer_token(&official);
    let body = json!({"request_id": request});

    let (status, state) = send(&client, STATUS, &[&token], &body).await;
    assert_eq!(status, Status::Ok, "{state}");
    assert_eq!(
        state,
        json!({
            "scope": {
                "tenant": w.tenant, "event": w.event, "request": request,
                "official": official.id,
            },
            "phase": "unavailable",
            "cancellable_attempt": null,
            "device_profile": "normal",
        })
    );

    script.lock().unwrap().offer = true;
    let (_, state) = send(&client, STATUS, &[&token], &body).await;
    assert_eq!(state["phase"], "challenge-available");

    // An overdue request takes no more approvals.
    w.make_overdue(request).await;
    let (_, state) = send(&client, STATUS, &[&token], &body).await;
    assert_eq!(state["phase"], "unavailable");
}

#[rocket::async_test]
async fn an_unused_attempt_can_be_cancelled_and_a_redeemed_one_cannot() {
    let w = World::new(2, true).await;
    let request = w.request().await;
    let official = w.enrolled(&Pki::get().jose, "yubihsm").await;
    let script = script(true, Begin::Ledger, End::Reconcile);
    let client = w
        .client(LedgerBroker::new(&w, Some(&official), &script))
        .await;
    let token = w.signer_token(&official);
    let status_of = |body: Value| body["phase"].as_str().unwrap().to_owned();
    let request_body = json!({"request_id": request});

    // The return page must be the request's own.
    for page in [
        election_page(Uuid::new_v4()),
        json!({"path": "election-show", "panel": "signatures-requests", "election_id": w.post}),
        json!({"path": "event-show", "panel": "publish", "election_id": w.post}),
    ] {
        let (status, body) = send(
            &client,
            START,
            &[&token],
            &json!({"request_id": request, "page": page}),
        )
        .await;
        assert_eq!(status, Status::Forbidden, "{page}");
        assert_eq!(
            message(&body),
            "The return page does not match this Initialize request."
        );
    }

    let (status, begun) = send(
        &client,
        START,
        &[&token],
        &json!({"request_id": request, "page": event_page()}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{begun}");
    let attempt: Uuid =
        serde_json::from_value(begun["attempt"].clone()).unwrap();
    assert_eq!(
        begun["redirect"],
        par_redirect(
            w.tenant,
            &format!(
                "client_id={OIDC_CLIENT}&request_uri={PAR_PREFIX}{}",
                attempt.simple()
            )
        )
    );
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(status_of(state.clone()), "waiting-for-identity");
    assert_eq!(state["cancellable_attempt"], json!(attempt));

    // A nil attempt or another one is not this official's to cancel.
    for other in [Uuid::nil(), Uuid::new_v4()] {
        assert_eq!(
            send(
                &client,
                CANCEL,
                &[&token],
                &json!({"request_id": request, "attempt": other}),
            )
            .await
            .0,
            Status::Conflict
        );
    }
    let cancel = json!({"request_id": request, "attempt": attempt});
    let (status, outcome) = send(&client, CANCEL, &[&token], &cancel).await;
    assert_eq!(
        (status, outcome),
        (Status::Ok, json!({"phase": "unavailable"}))
    );
    let entries: i64 = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT count(*) FROM sequent_backend.audit_log_outbox
             WHERE tenant_id = $1 AND details::text LIKE '%' || $2 || '%'",
            &[&w.tenant, &attempt.to_string()],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(entries, 2, "the return page and the cancellation");
    let (status, refusal) = send(&client, CANCEL, &[&token], &cancel).await;
    assert_eq!(status, Status::Conflict);
    assert_eq!(
        message(&refusal),
        "Can't cancel an attempt after it could have reached the device."
    );
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(status_of(state), "challenge-available");

    // A redeemed callback could reach the device: pending, not cancellable.
    let (_, begun) = send(
        &client,
        START,
        &[&token],
        &json!({"request_id": request, "page": election_page(w.post)}),
    )
    .await;
    let attempt: Uuid =
        serde_json::from_value(begun["attempt"].clone()).unwrap();
    let scope = w.scope(&official, request).await;
    redeem(&w.pool, &scope, attempt).await;
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(
        (status_of(state.clone()), &state["cancellable_attempt"]),
        ("pending".to_owned(), &Value::Null)
    );
    assert_eq!(
        send(
            &client,
            CANCEL,
            &[&token],
            &json!({"request_id": request, "attempt": attempt}),
        )
        .await
        .0,
        Status::Conflict
    );
    reserve(&w.pool, &scope, "yubihsm", attempt).await;
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(status_of(state), "pending");
    let (_, outcome) = send(&client, RECONCILE, &[&token], &request_body).await;
    assert_eq!(outcome, json!({"phase": "reconcile"}));

    // The broker's lost reply is reconciled, on the attempt's own page.
    let (status, view) =
        send(&client, FINISH, &[&token], &callback(request, attempt)).await;
    assert_eq!(status, Status::Ok, "{view}");
    assert_eq!(
        view,
        json!({
            "tenant": w.tenant, "event": w.event, "request": request,
            "official": official.id, "attempt": attempt, "phase": "reconcile",
            "page": election_page(w.post),
        })
    );
}

/// Starts the same attempt with the broker answering `begin`.
struct Starts<'a> {
    script: &'a Mutex<Script>,
    client: &'a Client,
    token: &'a str,
    body: &'a Value,
}

impl Starts<'_> {
    async fn with(&self, begin: Begin) -> (Status, Value) {
        self.script.lock().unwrap().begin = begin;
        send(self.client, START, &[self.token], self.body).await
    }
}

#[rocket::async_test]
async fn start_refuses_broker_failures_and_foreign_redirects() {
    let w = World::new(2, true).await;
    let request = w.request().await;
    let official = w.enrolled(&Pki::get().juan_b, "yubihsm").await;
    let script = script(true, Begin::Refuse, End::Refuse);
    let client = w
        .client(LedgerBroker::new(&w, Some(&official), &script))
        .await;
    let token = w.signer_token(&official);
    let body = json!({"request_id": request, "page": event_page()});
    let start = Starts {
        script: &script,
        client: &client,
        token: &token,
        body: &body,
    };

    let (status, refusal) = start.with(Begin::Refuse).await;
    assert_eq!(status, Status::Forbidden);
    assert_eq!(message(&refusal), "The device signing attempt was refused.");
    let (status, refusal) = start.with(Begin::Unavailable).await;
    assert_eq!(status, Status::ServiceUnavailable);
    assert_eq!(
        message(&refusal),
        "The private signing service is unavailable."
    );
    assert_eq!(
        start.with(Begin::NilAttempt).await.0,
        Status::ServiceUnavailable
    );

    let suffix = "par-1.a_b";
    let client_id = format!("client_id={OIDC_CLIENT}");
    let request_uri = format!("request_uri={PAR_PREFIX}{suffix}");
    let endpoint = authorize_endpoint(w.tenant).to_string();
    let query = format!("?{client_id}&{request_uri}");
    for location in [
        format!(
            "{}{query}",
            endpoint.replace("identity.invalid", "other.invalid")
        ),
        format!("{}{query}", endpoint.replace("/auth", "/token")),
        format!(
            "{}{query}",
            endpoint.replace("https://", "https://user:pw@")
        ),
        par_redirect(w.tenant, &format!("{client_id}&{request_uri}#fragment")),
        par_redirect(w.tenant, &client_id),
        par_redirect(w.tenant, &request_uri),
        par_redirect(
            w.tenant,
            &format!("{client_id}&{client_id}&{request_uri}"),
        ),
        par_redirect(w.tenant, &format!("client_id=other&{request_uri}")),
        par_redirect(w.tenant, &format!("{client_id}&request_uri={suffix}")),
        par_redirect(
            w.tenant,
            &format!("{client_id}&request_uri={PAR_PREFIX}"),
        ),
        par_redirect(
            w.tenant,
            &format!("{client_id}&request_uri={PAR_PREFIX}a%21b"),
        ),
        par_redirect(
            w.tenant,
            &format!("{client_id}&request_uri={PAR_PREFIX}{}", "a".repeat(257)),
        ),
        par_redirect(
            w.tenant,
            &format!("{client_id}&{request_uri}&prompt=none"),
        ),
    ] {
        let (status, _) = start.with(Begin::Redirect(location.clone())).await;
        assert_eq!(status, Status::ServiceUnavailable, "{location}");
    }

    // A permitted redirect for an attempt the ledger never prepared cannot
    // keep its return page, and is never shown.
    let (status, refusal) = start
        .with(Begin::Redirect(par_redirect(
            w.tenant,
            &format!("{client_id}&{request_uri}"),
        )))
        .await;
    assert_eq!(status, Status::Conflict);
    assert_eq!(
        message(&refusal),
        "This signing attempt could not preserve its return page. Check its status."
    );
}

#[rocket::async_test]
async fn finish_records_the_device_approval_or_leaves_it_to_reconcile() {
    // Three signatures: the request keeps waiting, so no action runs.
    let w = World::new(3, true).await;
    let request = w.request().await;
    let pki = Pki::get();
    let first = w.enrolled(&pki.jose, "yubihsm").await;
    let script_first = script(true, Begin::Ledger, End::Settle);
    let client = w
        .client(LedgerBroker::new(&w, Some(&first), &script_first))
        .await;
    let token = w.signer_token(&first);
    let request_body = json!({"request_id": request});
    let (status, begun) = send(
        &client,
        START,
        &[&token],
        &json!({"request_id": request, "page": election_page(w.post)}),
    )
    .await;
    assert_eq!(status, Status::Ok, "{begun}");
    let attempt: Uuid =
        serde_json::from_value(begun["attempt"].clone()).unwrap();

    // A malformed callback, or one for an attempt that is not the
    // official's own, reaches no broker.
    for (state, code) in [
        ("s".repeat(42), "code".to_owned()),
        (format!("{}!", "s".repeat(42)), "code".to_owned()),
        ("s".repeat(43), String::new()),
        ("s".repeat(43), "a code".to_owned()),
        ("s".repeat(43), "c".repeat(4097)),
    ] {
        let (status, refusal) = send(
            &client,
            FINISH,
            &[&token],
            &json!({"request_id": request, "attempt": attempt, "state": state, "code": code}),
        )
        .await;
        assert_eq!(status, Status::BadRequest);
        assert_eq!(message(&refusal), "Invalid signing callback.");
    }
    for other in [Uuid::nil(), Uuid::new_v4()] {
        let (status, refusal) =
            send(&client, FINISH, &[&token], &callback(request, other)).await;
        assert_eq!(status, Status::Forbidden);
        assert_eq!(
            message(&refusal),
            "No owned signing return for this attempt."
        );
    }

    // Broker refusals and a receipt of another scope are not approvals.
    for (end, expected) in [
        (End::Refuse, Status::Forbidden),
        (End::Unavailable, Status::ServiceUnavailable),
        (End::OtherScope, Status::Forbidden),
        (End::NilChallenge, Status::Forbidden),
    ] {
        script_first.lock().unwrap().end = end;
        assert_eq!(
            send(&client, FINISH, &[&token], &callback(request, attempt))
                .await
                .0,
            expected
        );
    }

    // The settled device receipt is the official's approval.
    script_first.lock().unwrap().end = End::Settle;
    let (status, view) =
        send(&client, FINISH, &[&token], &callback(request, attempt)).await;
    assert_eq!(status, Status::Ok, "{view}");
    assert_eq!(view["phase"], "approval-recorded");
    assert_eq!(view["page"], election_page(w.post));
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(state["phase"], "approval-recorded");
    let (_, outcome) = send(&client, RECONCILE, &[&token], &request_body).await;
    assert_eq!(outcome, json!({"phase": "approval-recorded"}));
    let (status, refusal) = send(
        &client,
        START,
        &[&token],
        &json!({"request_id": request, "page": event_page()}),
    )
    .await;
    assert_eq!(status, Status::Conflict);
    assert_eq!(message(&refusal), "Your approval is already recorded.");
    // A retried return shows the recorded approval, without the device.
    script_first.lock().unwrap().end = End::Refuse;
    let (_, view) =
        send(&client, FINISH, &[&token], &callback(request, attempt)).await;
    assert_eq!(view["phase"], "approval-recorded");

    // A receipt of the test device profile is settled but never approved:
    // it is left to reconcile.
    let second = w.enrolled(&pki.juan_b, "softhsm-test").await;
    let client = w
        .client(LedgerBroker::new(
            &w,
            Some(&second),
            &script(true, Begin::Ledger, End::Settle),
        ))
        .await;
    let token = w.signer_token(&second);
    let (_, begun) = send(
        &client,
        START,
        &[&token],
        &json!({"request_id": request, "page": event_page()}),
    )
    .await;
    let attempt: Uuid =
        serde_json::from_value(begun["attempt"].clone()).unwrap();
    let (status, _) =
        send(&client, FINISH, &[&token], &callback(request, attempt)).await;
    assert_eq!(status, Status::Forbidden);
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(state["phase"], "reconcile");
    let (_, outcome) = send(&client, RECONCILE, &[&token], &request_body).await;
    assert_eq!(outcome, json!({"phase": "reconcile"}));

    // A completed receipt whose approval was never applied is applied by
    // reconciling, once.
    let third = w.enrolled(third_signer(), "yubihsm").await;
    let client = w
        .client(LedgerBroker::new(
            &w,
            Some(&third),
            &script(true, Begin::Refuse, End::Refuse),
        ))
        .await;
    let token = w.signer_token(&third);
    let (_, outcome) = send(&client, RECONCILE, &[&token], &request_body).await;
    assert_eq!(outcome, json!({"phase": "unavailable"}));
    let scope = w.scope(&third, request).await;
    let attempt = Uuid::new_v4();
    prepare_attempt(&w.pool, &scope, "yubihsm", attempt).await;
    redeem(&w.pool, &scope, attempt).await;
    let fence = reserve(&w.pool, &scope, "yubihsm", attempt).await;
    claim_and_settle(&w.pool, &scope, &third.key, attempt, fence).await;
    let (_, state) = send(&client, STATUS, &[&token], &request_body).await;
    assert_eq!(state["phase"], "reconcile");
    let (_, outcome) = send(&client, RECONCILE, &[&token], &request_body).await;
    assert_eq!(outcome, json!({"phase": "approval-recorded"}));
    let approvals: i64 = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT count(*) FROM sequent_backend.signing_approval
             WHERE tenant_id = $1 AND request_id = $2",
            &[&w.tenant, &request],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(approvals, 2);
}

// --- The private broker client ----------------------------------------------

/// A private CA, a server certificate for 127.0.0.1 and two device client
/// certificates, for TLS only. The private CA tests use the second device.
#[allow(dead_code)]
struct TlsPki {
    root: X509,
    server: X509,
    server_key: PKey<Private>,
    device: X509,
    device_key: PKey<Private>,
    stranger: X509,
}

#[derive(Clone, Copy)]
enum TlsRole {
    Ca,
    Server,
    Client,
}

fn tls_certificate(
    common_name: &str,
    key: &PKey<Private>,
    issuer: Option<(&X509, &PKey<Private>)>,
    role: TlsRole,
) -> X509 {
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", common_name).unwrap();
    let name = name.build();
    let mut builder = X509Builder::new().unwrap();
    builder.set_version(2).unwrap();
    let serial = BigNum::from_slice(&Uuid::new_v4().as_bytes()[..15])
        .unwrap()
        .to_asn1_integer()
        .unwrap();
    builder.set_serial_number(&serial).unwrap();
    builder.set_subject_name(&name).unwrap();
    match issuer {
        Some((certificate, _)) => {
            builder.set_issuer_name(certificate.subject_name()).unwrap()
        }
        None => builder.set_issuer_name(&name).unwrap(),
    }
    builder.set_pubkey(key).unwrap();
    let now = Utc::now().timestamp();
    builder
        .set_not_before(&Asn1Time::from_unix(now - 3600).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::from_unix(now + 30 * 86_400).unwrap())
        .unwrap();
    match role {
        TlsRole::Ca => {
            builder
                .append_extension(
                    BasicConstraints::new().critical().ca().build().unwrap(),
                )
                .unwrap();
            builder
                .append_extension(
                    KeyUsage::new()
                        .critical()
                        .key_cert_sign()
                        .crl_sign()
                        .build()
                        .unwrap(),
                )
                .unwrap();
        }
        TlsRole::Server | TlsRole::Client => {
            builder
                .append_extension(BasicConstraints::new().build().unwrap())
                .unwrap();
            builder
                .append_extension(
                    KeyUsage::new()
                        .critical()
                        .digital_signature()
                        .build()
                        .unwrap(),
                )
                .unwrap();
            let mut usage = ExtendedKeyUsage::new();
            if matches!(role, TlsRole::Server) {
                usage.server_auth();
                let san = SubjectAlternativeName::new()
                    .ip("127.0.0.1")
                    .build(&builder.x509v3_context(
                        issuer.map(|(certificate, _)| &**certificate),
                        None,
                    ))
                    .unwrap();
                builder.append_extension(san).unwrap();
            } else {
                usage.client_auth();
            }
            builder.append_extension(usage.build().unwrap()).unwrap();
        }
    }
    let signing_key = issuer.map_or(key, |(_, key)| key);
    builder.sign(signing_key, MessageDigest::sha256()).unwrap();
    builder.build()
}

fn tls_pki() -> &'static TlsPki {
    static TLS: OnceLock<TlsPki> = OnceLock::new();
    TLS.get_or_init(|| {
        let root_key = signing_pki::ec_key();
        let root =
            tls_certificate("Test Private CA", &root_key, None, TlsRole::Ca);
        let issuer = Some((&root, &root_key));
        let server_key = signing_pki::ec_key();
        let server =
            tls_certificate("127.0.0.1", &server_key, issuer, TlsRole::Server);
        let device_key = signing_pki::ec_key();
        let device = tls_certificate(
            "Test device",
            &device_key,
            issuer,
            TlsRole::Client,
        );
        let stranger = tls_certificate(
            "Another device",
            &signing_pki::ec_key(),
            issuer,
            TlsRole::Client,
        );
        TlsPki {
            root,
            server,
            server_key,
            device,
            device_key,
            stranger,
        }
    })
}

fn pem(certificate: &X509) -> Vec<u8> {
    certificate.to_pem().unwrap()
}

fn key_pem(key: &PKey<Private>) -> Vec<u8> {
    key.private_key_to_pem_pkcs8().unwrap()
}

impl TlsPki {
    fn roots(&self) -> Vec<reqwest::Certificate> {
        vec![reqwest::Certificate::from_pem(&pem(&self.root)).unwrap()]
    }

    /// Harvest's per-tenant mTLS identity towards the broker.
    fn identity(&self) -> reqwest::Identity {
        reqwest::Identity::from_pkcs8_pem(
            &pem(&self.device),
            &key_pem(&self.device_key),
        )
        .unwrap()
    }
}

/// A request the HTTPS stand-in received.
struct Seen {
    path: String,
    body: Value,
    client_certificate: bool,
}

/// Answers one connection per reply, in order, over TLS with the test
/// server certificate; records what each request sent.
fn https_stand_in(
    replies: Vec<(u16, String)>,
) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let tls = tls_pki();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!(
        "https://127.0.0.1:{}/",
        listener.local_addr().unwrap().port()
    );
    let mut acceptor =
        SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server()).unwrap();
    acceptor.set_private_key(&tls.server_key).unwrap();
    acceptor.set_certificate(&tls.server).unwrap();
    // Ask for the client's certificate to see that one is sent.
    acceptor.set_verify_callback(SslVerifyMode::PEER, |_, _| true);
    let acceptor = acceptor.build();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    std::thread::spawn(move || {
        for (status, body) in replies {
            let (stream, _) = listener.accept().unwrap();
            let stream = acceptor.accept(stream).unwrap();
            let client_certificate = stream.ssl().peer_certificate().is_some();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap();
                    }
                }
            }
            let mut data = vec![0; length];
            reader.read_exact(&mut data).unwrap();
            record.lock().unwrap().push(Seen {
                path,
                body: serde_json::from_slice(&data).unwrap_or(Value::Null),
                client_certificate,
            });
            let stream = reader.get_mut();
            write!(
                stream,
                "HTTP/1.1 {status} Reply\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            stream.flush().unwrap();
            let _ = stream.shutdown();
        }
    });
    (origin, seen)
}

fn private_broker(
    tenant: Uuid,
    origin: &str,
) -> Result<PrivateInitializeBroker, BrokerFailure> {
    let tls = tls_pki();
    PrivateInitializeBroker::new(
        tenant,
        origin,
        KEYCLOAK,
        OIDC_CLIENT,
        &tls.roots(),
        tls.identity(),
    )
}

fn failure<T>(result: Result<T, BrokerFailure>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(BrokerFailure::Refused) => "refused",
        Err(BrokerFailure::Unavailable) => "unavailable",
        Err(BrokerFailure::Reconcile) => "reconcile",
    }
}

fn broker_scope(tenant: Uuid) -> BrokerScope {
    BrokerScope {
        tenant,
        event: Uuid::new_v4(),
        request: Uuid::new_v4(),
        official: Uuid::new_v4(),
        payload_sha256: "ab".repeat(32),
    }
}

#[rocket::async_test]
async fn the_private_broker_is_provisioned_only_with_fixed_https_endpoints() {
    let tls = tls_pki();
    let tenant = Uuid::new_v4();
    let origin = "https://127.0.0.1:9/";
    let build = |tenant: Uuid,
                 origin: &str,
                 keycloak: &str,
                 client: &str,
                 roots: &[reqwest::Certificate]| {
        failure(PrivateInitializeBroker::new(
            tenant,
            origin,
            keycloak,
            client,
            roots,
            tls.identity(),
        ))
    };
    let roots = tls.roots();
    let long_client = "c".repeat(151);
    for (tenant, origin, keycloak, client, roots) in [
        (Uuid::nil(), origin, KEYCLOAK, OIDC_CLIENT, &roots[..]),
        (tenant, origin, KEYCLOAK, OIDC_CLIENT, &[][..]),
        (tenant, origin, KEYCLOAK, "", &roots[..]),
        (tenant, origin, KEYCLOAK, long_client.as_str(), &roots[..]),
        (tenant, origin, KEYCLOAK, "admin portal", &roots[..]),
        (tenant, "not a url", KEYCLOAK, OIDC_CLIENT, &roots[..]),
        (
            tenant,
            "http://127.0.0.1:9/",
            KEYCLOAK,
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            "https://127.0.0.1:9/b",
            KEYCLOAK,
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            "https://u@127.0.0.1:9/",
            KEYCLOAK,
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            "https://:p@127.0.0.1:9/",
            KEYCLOAK,
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            "https://127.0.0.1:9/?q",
            KEYCLOAK,
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            "https://127.0.0.1:9/#f",
            KEYCLOAK,
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            origin,
            "http://identity.invalid",
            OIDC_CLIENT,
            &roots[..],
        ),
        (
            tenant,
            origin,
            "https://identity.invalid?q",
            OIDC_CLIENT,
            &roots[..],
        ),
    ] {
        assert_eq!(
            build(tenant, origin, keycloak, client, roots),
            "unavailable",
            "{origin} {keycloak} {client}"
        );
    }
    let Ok(broker) = private_broker(tenant, origin) else {
        panic!("a fixed HTTPS broker is provisioned");
    };
    assert_eq!(broker.authorization_endpoint(), &authorize_endpoint(tenant));
    assert_eq!(broker.oidc_client_id(), OIDC_CLIENT);
}

#[rocket::async_test]
async fn the_private_broker_sends_the_exact_scope_and_checks_every_reply() {
    let tenant = Uuid::new_v4();
    let scope = broker_scope(tenant);
    let other = broker_scope(tenant);
    let attempt = Uuid::new_v4();
    let challenge = Uuid::new_v4();
    let redirect = par_redirect(
        tenant,
        &format!("client_id={OIDC_CLIENT}&request_uri={PAR_PREFIX}abc"),
    );
    let oversized = json!({"pad": "a".repeat(17_000)}).to_string();
    let reply = |status: u16, body: Value| (status, body.to_string());
    let (origin, seen) = https_stand_in(vec![
        reply(200, json!({"scope": scope, "challenge_available": true})),
        reply(200, json!({"scope": other, "challenge_available": true})),
        reply(403, json!({})),
        reply(503, json!({})),
        reply(200, json!([])),
        reply(
            200,
            json!({"scope": scope, "challenge_available": true, "extra": 1}),
        ),
        (200, oversized.clone()),
        reply(
            200,
            json!({"scope": scope, "attempt": attempt, "redirect": redirect}),
        ),
        reply(
            200,
            json!({"scope": scope, "attempt": Uuid::nil(), "redirect": redirect}),
        ),
        reply(
            200,
            json!({"scope": other, "attempt": attempt, "redirect": redirect}),
        ),
        reply(
            200,
            json!({"scope": scope, "attempt": attempt, "redirect": "::not a url"}),
        ),
        reply(
            200,
            json!({
                "scope": scope, "attempt": attempt,
                "redirect": format!("{KEYCLOAK}/{}", "a".repeat(2100)),
            }),
        ),
        reply(404, json!({})),
        reply(
            200,
            json!({"scope": scope, "approval_challenge": challenge}),
        ),
        reply(
            200,
            json!({"scope": scope, "approval_challenge": Uuid::nil()}),
        ),
        reply(
            200,
            json!({"scope": other, "approval_challenge": challenge}),
        ),
        reply(403, json!({})),
        (200, oversized),
    ]);
    let Ok(broker) = private_broker(tenant, &origin) else {
        panic!("the broker is provisioned");
    };

    assert!(matches!(broker.can_offer(&scope).await, Ok(true)));
    for expected in [
        "refused",
        "refused",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
    ] {
        assert_eq!(failure(broker.can_offer(&scope).await), expected);
    }
    let Ok(started) = broker.start(&scope).await else {
        panic!("the broker starts the attempt");
    };
    assert_eq!(
        (started.attempt, started.location.to_string()),
        (attempt, redirect.clone())
    );
    for expected in [
        "refused",
        "refused",
        "unavailable",
        "unavailable",
        "refused",
    ] {
        assert_eq!(failure(broker.start(&scope).await), expected);
    }
    let state = "s".repeat(43);
    let Ok(settled) = broker.finish(&scope, attempt, &state, "code").await
    else {
        panic!("the broker settles the attempt");
    };
    assert!(settled.scope == scope && settled.approval_challenge == challenge);
    // After the device could have been used, every failure is reconciled.
    for _ in 0..4 {
        assert_eq!(
            failure(broker.finish(&scope, attempt, &state, "code").await),
            "reconcile"
        );
    }

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 18);
    assert!(seen.iter().all(|request| request.client_certificate));
    let path = |name: &str| {
        format!("/private/v1/operational-signing/initialize/{name}")
    };
    assert_eq!(
        (seen[0].path.clone(), &seen[0].body),
        (path("eligibility"), &json!(scope))
    );
    assert_eq!(
        (seen[7].path.clone(), &seen[7].body),
        (path("start"), &json!(scope))
    );
    assert_eq!(
        (seen[13].path.clone(), &seen[13].body),
        (
            path("finish"),
            &json!({"scope": scope, "attempt": attempt, "state": state, "code": "code"})
        )
    );
}

#[rocket::async_test]
async fn the_private_broker_contacts_nobody_for_another_scope() {
    let tenant = Uuid::new_v4();
    // Nothing listens on the discarded port.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let Ok(broker) =
        private_broker(tenant, &format!("https://127.0.0.1:{port}/"))
    else {
        panic!("the broker is provisioned");
    };
    let scope = broker_scope(tenant);
    assert_eq!(failure(broker.can_offer(&scope).await), "unavailable");
    assert_eq!(failure(broker.start(&scope).await), "unavailable");
    assert_eq!(
        failure(broker.finish(&scope, Uuid::new_v4(), "state", "code").await),
        "reconcile"
    );

    for refused in [
        broker_scope(Uuid::new_v4()),
        BrokerScope {
            event: Uuid::nil(),
            ..broker_scope(tenant)
        },
        BrokerScope {
            official: Uuid::nil(),
            ..broker_scope(tenant)
        },
        BrokerScope {
            payload_sha256: "AB".repeat(32),
            ..broker_scope(tenant)
        },
        BrokerScope {
            payload_sha256: "ab".repeat(31),
            ..broker_scope(tenant)
        },
    ] {
        assert_eq!(failure(broker.can_offer(&refused).await), "refused");
        assert_eq!(failure(broker.start(&refused).await), "refused");
        assert_eq!(
            failure(
                broker
                    .finish(&refused, Uuid::new_v4(), "state", "code")
                    .await
            ),
            "refused"
        );
    }
}

// --- The private CA assessment server ---------------------------------------

#[cfg(feature = "operational-signing-private-ca-candidate")]
mod private_ca {
    use super::*;
    use crate::routes::private_operational_ca::unmounted_authority;
    use deadpool_postgres::Runtime;
    use rocket::{Build, Rocket};
    use std::net::SocketAddr;
    use tokio_postgres::NoTls;
    use windmill::services::database::PgConfig;
    use windmill::services::signing::operational_ca::RequiredCaDevice;

    const ASSESS: &str = "/private/v1/operational-signing/initialize/ca";

    /// The pin of a client certificate: SHA-256 of its public key.
    fn pin(certificate: &X509) -> [u8; 32] {
        Sha256::digest(
            certificate
                .public_key()
                .unwrap()
                .public_key_to_der()
                .unwrap(),
        )
        .into()
    }

    fn unreachable_pool() -> Pool {
        deadpool_postgres::Config {
            host: Some("127.0.0.1".into()),
            port: Some(1),
            user: Some("test".into()),
            dbname: Some("test".into()),
            ..Default::default()
        }
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .unwrap()
    }

    /// A new login with only the CA assessment role, on the test database.
    async fn assessment_login(pool: &Pool) -> Pool {
        let role = format!("os_ca_test_{}", Uuid::new_v4().simple());
        let password = Uuid::new_v4().simple().to_string();
        let client = pool.get().await.unwrap();
        client
            .batch_execute(&format!(
                "CREATE ROLE {role} LOGIN PASSWORD '{password}';
                 GRANT sequent_os_initialize_ca TO {role};"
            ))
            .await
            .unwrap();
        let database: String = client
            .query_one("SELECT current_database()", &[])
            .await
            .unwrap()
            .get(0);
        let mut config = PgConfig::from_env()
            .expect("HASURA_DB__* names the test server")
            .hasura_db;
        config.dbname = Some(database);
        config.user = Some(role);
        config.password = Some(password);
        config.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
    }

    async fn authority(
        tenant: Uuid,
        peer_pin: [u8; 32],
        database: Pool,
        server_chain: &[u8],
        server_key: &[u8],
        client_ca: &[u8],
    ) -> Result<Rocket<Build>, Status> {
        unmounted_authority(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            tenant,
            peer_pin,
            RequiredCaDevice::YubiHsm,
            database,
            server_chain,
            server_key,
            client_ca,
        )
        .await
    }

    #[rocket::async_test]
    async fn the_assessment_server_starts_only_with_its_scoped_login() {
        let tls = tls_pki();
        let (chain, key, ca) =
            (pem(&tls.server), key_pem(&tls.server_key), pem(&tls.root));
        let tenant = Uuid::new_v4();
        let device = pin(&tls.device);
        for (tenant, peer_pin, chain, key, ca) in [
            (Uuid::nil(), device, &chain[..], &key[..], &ca[..]),
            (tenant, [0; 32], &chain[..], &key[..], &ca[..]),
            (tenant, device, &[][..], &key[..], &ca[..]),
            (tenant, device, &chain[..], &[][..], &ca[..]),
            (tenant, device, &chain[..], &key[..], &[][..]),
        ] {
            assert_eq!(
                authority(tenant, peer_pin, unreachable_pool(), chain, key, ca)
                    .await
                    .err(),
                Some(Status::ServiceUnavailable)
            );
        }
        assert_eq!(
            authority(tenant, device, unreachable_pool(), &chain, &key, &ca)
                .await
                .err(),
            Some(Status::ServiceUnavailable)
        );
        // The migration login can do far more than assess.
        let services = Services::on_test_database().await;
        assert_eq!(
            authority(
                tenant,
                device,
                (*services.hasura).clone(),
                &chain,
                &key,
                &ca
            )
            .await
            .err(),
            Some(Status::ServiceUnavailable)
        );
        let login = assessment_login(&services.hasura).await;
        assert!(authority(tenant, device, login, &chain, &key, &ca)
            .await
            .is_ok());
    }

    #[rocket::async_test]
    async fn the_assessment_server_answers_only_its_pinned_device() {
        let tls = tls_pki();
        let w = World::new(2, true).await;
        let request = w.request().await;
        let official = w.enrolled(&Pki::get().jose, "yubihsm").await;
        let scope = w.scope(&official, request).await;
        let attempt = Uuid::new_v4();
        prepare_attempt(&w.pool, &scope, "yubihsm", attempt).await;
        redeem(&w.pool, &scope, attempt).await;
        let fence = reserve(&w.pool, &scope, "yubihsm", attempt).await;

        let login = assessment_login(&w.pool).await;
        let Ok(rocket) = authority(
            w.tenant,
            pin(&tls.device),
            login.clone(),
            &pem(&tls.server),
            &key_pem(&tls.server_key),
            &pem(&tls.root),
        )
        .await
        else {
            panic!("the assessment server starts");
        };
        let client = Client::tracked(rocket).await.unwrap();
        let probe = json!({
            "tenant": w.tenant, "event": w.event, "request": request,
            "official": official.id, "challenge": attempt, "fence": fence,
        });
        let assess = |identity: Option<&X509>, probe: &Value| {
            let mut request = client
                .post(ASSESS)
                .header(ContentType::JSON)
                .body(probe.to_string());
            if let Some(certificate) = identity {
                request = request.identity(&pem(certificate)[..]);
            }
            async move {
                let response = request.dispatch().await;
                let status = response.status();
                let body = response.into_string().await.unwrap_or_default();
                (status, serde_json::from_str(&body).unwrap_or(Value::Null))
            }
        };

        assert_eq!(assess(None, &probe).await.0, Status::Unauthorized);
        assert_eq!(
            assess(Some(&tls.stranger), &probe).await.0,
            Status::Forbidden
        );
        let mut other_tenant = probe.clone();
        other_tenant["tenant"] = json!(Uuid::new_v4());
        assert_eq!(
            assess(Some(&tls.device), &other_tenant).await.0,
            Status::Forbidden
        );
        let mut unknown = probe.clone();
        unknown["challenge"] = json!(Uuid::new_v4());
        assert_eq!(
            assess(Some(&tls.device), &unknown).await.0,
            Status::Forbidden
        );
        let (status, assessment) = assess(Some(&tls.device), &probe).await;
        assert_eq!(status, Status::Ok, "{assessment}");
        assert_eq!(assessment["scope"], probe);
        assert_eq!(
            (
                &assessment["key_id"],
                &assessment["profile"],
                &assessment["certificate_id"],
                &assessment["cert_fingerprint"],
            ),
            (
                &json!(format!("official/{}/{}", w.tenant, official.id)),
                &json!("yubihsm"),
                &json!(official.certificate),
                &json!(official.key.fingerprint),
            )
        );

        // The pin is checked before the database: with the database gone
        // the stranger is still refused and the device is unavailable.
        login.close();
        assert_eq!(
            assess(Some(&tls.stranger), &probe).await.0,
            Status::Forbidden
        );
        assert_eq!(
            assess(Some(&tls.device), &probe).await.0,
            Status::ServiceUnavailable
        );
    }
}
