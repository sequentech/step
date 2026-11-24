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

    use crate::route_services::{rows, service_tokens, Services};
    use crate::services::operational_portal_auth::OperationalPortalAuth;
    use crate::test_claims::Claims;
    use chrono::Duration as ChronoDuration;
    use openssl::{
        asn1::Asn1Time,
        hash::MessageDigest,
        pkey::PKey,
        rsa::Rsa,
        x509::{X509NameBuilder, X509},
    };
    use rocket::{
        http::{ContentType, Header, Status},
        local::asynchronous::Client as LocalClient,
    };
    use sequent_core::signing::{
        payload_sha256, RequesterSigning, SigningAction, SigningPayload,
        SigningPayloadFields, SigningRequirement, SigningRule,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::net::TcpListener;

    const LABEL: &str = "initialize-fixture-post";
    const STATUS: &str = "/operational-signing/initialize/setup/status";
    const START: &str = "/operational-signing/initialize/setup";

    // A real TLS client configuration without persisted private material.
    // The socket peer deliberately resets, including on redispatch.
    fn local_identity() -> (Certificate, Identity) {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "synthetic setup client")
            .unwrap();
        let name = name.build();
        let mut x = X509::builder().unwrap();
        x.set_version(2).unwrap();
        x.set_subject_name(&name).unwrap();
        x.set_issuer_name(&name).unwrap();
        x.set_pubkey(&key).unwrap();
        let from = Asn1Time::days_from_now(0).unwrap();
        let until = Asn1Time::days_from_now(1).unwrap();
        x.set_not_before(&from).unwrap();
        x.set_not_after(&until).unwrap();
        x.sign(&key, MessageDigest::sha256()).unwrap();
        let certificate = x.build().to_pem().unwrap();
        let mut combined = certificate.clone();
        combined.extend(key.private_key_to_pem_pkcs8().unwrap());
        (
            Certificate::from_pem(&certificate).unwrap(),
            Identity::from_pem(&combined).unwrap(),
        )
    }

    async fn pending_request(
        pool: &deadpool_postgres::Pool,
        tenant: Uuid,
        event: Uuid,
        post: Uuid,
        requester: Uuid,
    ) -> Uuid {
        let request = Uuid::new_v4();
        let now = Utc::now();
        let expires = now + ChronoDuration::hours(1);
        let subject = json!({"publication_id": Uuid::new_v4()});
        let rule = SigningRule {
            action: SigningAction::InitializeVoting,
            requirement: SigningRequirement::Required,
            signatures: 2,
            requester_signing: RequesterSigning::NotAllowed,
            expires_minutes: Some(60),
            revision: 1,
        };
        // Seed only a pending request, not a real publication or tally effect.
        let payload = SigningPayload::new(SigningPayloadFields {
            tenant_id: tenant.to_string(),
            election_event_id: event.to_string(),
            request_id: request,
            action: SigningAction::InitializeVoting,
            election_id: Some(post.to_string()),
            area_id: None,
            subject: subject.clone(),
            config_revision: None,
            rule_revision: rule.revision,
            requested_by: requester.to_string(),
            created_at: now,
            expires_at: Some(expires),
        })
        .unwrap();
        let canonical = payload.canonical().unwrap();
        rows::execute(pool, "INSERT INTO sequent_backend.signing_request
            (id,tenant_id,election_event_id,action,election_id,scope_key,subject,
             canonical_payload,payload_sha256,code,rule_revision,rule_snapshot,required,
             requested_by,requested_by_username,expires_at,created_at,permission_label)
            VALUES ($1,$2,$3,'initialize-voting',$4,$5,$6,$7,$8,$9,1,$10,2,$11,$11,$12,$13,$14)",
            &[&request,&tenant,&event,&post,&format!("{post}|||"),&subject,&canonical,
              &payload_sha256(&canonical),&payload.code(),&serde_json::to_value(&rule).unwrap(),
              &requester.to_string(),&expires,&now,&LABEL]).await;
        request
    }

    struct Portal<'a> {
        client: &'a LocalClient,
        tenant: Uuid,
        issuer: String,
    }

    impl Portal<'_> {
        fn claims(
            &self,
            official: Uuid,
            gold: bool,
            labels: &[&str],
        ) -> serde_json::Value {
            let tenant = self.tenant.to_string();
            let claims = Claims::new(&tenant, &official.to_string())
                .azp("admin-portal")
                .roles([Permissions::SIGN_INITIALIZE_VOTING])
                .permission_labels(labels)
                .acr(if gold { "gold" } else { "1" })
                .auth_time(Utc::now().timestamp());
            let mut value = service_tokens::issued(&claims, &tenant);
            value["iss"] = json!(self.issuer);
            value["aud"] = json!("admin-portal");
            value
        }

        async fn request(
            &self,
            route: &str,
            body: serde_json::Value,
            claim: &serde_json::Value,
        ) -> (Status, serde_json::Value) {
            let response = self
                .client
                .post(route)
                .header(ContentType::JSON)
                .header(Header::new(
                    "Authorization",
                    service_tokens::RealmSigner::shared().sign_payload(claim),
                ))
                .body(body.to_string())
                .dispatch()
                .await;
            let status = response.status();
            (
                status,
                response
                    .into_json()
                    .await
                    .unwrap_or(serde_json::Value::Null),
            )
        }

        async fn phase(
            &self,
            route: &str,
            request: Uuid,
            claim: &serde_json::Value,
            event: Uuid,
            official: Uuid,
            expected: &str,
        ) {
            let (status, body) = self
                .request(route, json!({"request_id": request}), claim)
                .await;
            assert_eq!(status, Status::Ok, "{body}");
            assert_eq!(
                body,
                json!({
                    "scope":{"tenant": self.tenant, "event": event,"request": request,"official": official},
                    "phase": expected
                })
            );
        }
    }

    #[rocket::async_test]
    async fn setup_http_reconciles_only_the_same_unclaimed_attempt() {
        let services = Services::on_test_database().await;
        let pool = &services.hasura;
        let event_row = rows::event(pool).await;
        let tenant = event_row.tenant_id.parse::<Uuid>().unwrap();
        let event = event_row.election_event_id.parse::<Uuid>().unwrap();
        let post = event_row
            .labelled_election(pool, LABEL)
            .await
            .parse::<Uuid>()
            .unwrap();
        let other_post = event_row
            .labelled_election(pool, LABEL)
            .await
            .parse::<Uuid>()
            .unwrap();
        let official = Uuid::new_v4();
        let officer = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        rows::execute(pool, "INSERT INTO sequent_backend.operational_signing_required_tenant VALUES ($1)", &[&tenant]).await;
        rows::execute(
            pool,
            "INSERT INTO sequent_backend.operational_setup_installation
            (tenant_id,device_profile) VALUES ($1,'yubihsm')",
            &[&tenant],
        )
        .await;
        rows::execute(
            pool,
            "INSERT INTO sequent_backend.operational_initialize_setup_roster
            (tenant_id,event_id,official,officer,eligible_until)
            VALUES ($1,$2,$3,$4,now()+interval '1 hour')",
            &[&tenant, &event, &official, &officer],
        )
        .await;
        let request = pending_request(pool, tenant, event, post, officer).await;
        let unrelated =
            pending_request(pool, tenant, event, other_post, officer).await;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("https://{}", listener.local_addr().unwrap());
        let deliveries = Arc::new(AtomicUsize::new(0));
        let observed = deliveries.clone();
        let peer = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                observed.fetch_add(1, Ordering::SeqCst);
                drop(socket);
            }
        });
        let (root, identity) = local_identity();
        let dispatcher =
            SetupDispatcher::new(tenant, event, &origin, root, identity)
                .unwrap();
        let issuer =
            format!("https://portal-test.invalid/realms/tenant-{tenant}");
        let verifier = OperationalPortalAuth::new(
            tenant,
            issuer.clone(),
            "admin-portal".into(),
            "admin-portal".into(),
            service_tokens::RealmSigner::shared().jwks(),
        )
        .unwrap();
        let client = LocalClient::tracked(
            rocket::build()
                .manage(services.harvest_services())
                .manage(dispatcher)
                .manage(verifier)
                .manage(
                    crate::routes::operational_initialize::DemoSigningScope {
                        tenant,
                        event,
                        run: Uuid::new_v4(),
                        profile: "yubihsm",
                    },
                )
                .mount(
                    "/",
                    rocket::routes![
                        status,
                        start,
                        crate::routes::operational_initialize::status
                    ],
                ),
        )
        .await
        .unwrap();
        let api = Portal {
            client: &client,
            tenant,
            issuer,
        };
        let normal = api.claims(official, false, &[LABEL]);
        let gold = api.claims(official, true, &[LABEL]);
        let other = api.claims(outsider, true, &[LABEL]);
        let wrong_post = api.claims(official, true, &["some-other-post"]);
        // The common profile cannot install the reserved demo database;
        // ordinary databases must refuse this route even with a matching
        // tenant/Post and a genuine independently signed token.
        let (ordinary, _) = api
            .request(
                "/operational-signing/initialize/status",
                json!({"request_id": request}),
                &gold,
            )
            .await;
        assert_eq!(ordinary, Status::Forbidden);
        let mut wrong_audience = gold.clone();
        wrong_audience["aud"] = json!("some-other-portal");
        assert_eq!(
            api.request(START, json!({"request_id":request}), &wrong_audience)
                .await
                .0,
            Status::Unauthorized
        );
        assert_eq!(
            api.request(
                START,
                json!({"request_id":request,"official":officer}),
                &gold
            )
            .await
            .0,
            Status::UnprocessableEntity
        );
        assert_eq!(
            api.request(START, json!({"request_id":request}), &wrong_post)
                .await
                .0,
            Status::Forbidden
        );
        api.phase(STATUS, request, &normal, event, official, "setup-available")
            .await;
        let (status, body) = api
            .request(START, json!({"request_id":request}), &normal)
            .await;
        assert_eq!(status, Status::Forbidden);
        assert_eq!(body["extensions"]["gold_required"], true);
        assert_eq!(deliveries.load(Ordering::SeqCst), 0);

        api.phase(
            START,
            request,
            &gold,
            event,
            official,
            "setup-retry-available",
        )
        .await;
        assert_eq!(deliveries.load(Ordering::SeqCst), 1);
        let row = rows::query(
            pool,
            "SELECT g.attempt, g.claim_deadline
            FROM sequent_backend.operational_setup_generation g
            WHERE g.tenant_id=$1 AND g.official=$2",
            &[&tenant, &official],
        )
        .await;
        let attempt: Uuid = row[0].get(0);
        let deadline: DateTime<Utc> = row[0].get(1);
        api.phase(START, request, &other, event, outsider, "unavailable")
            .await;
        api.phase(START, unrelated, &gold, event, official, "unavailable")
            .await;
        let mut stale = gold.clone();
        stale["auth_time"] = json!(Utc::now().timestamp() - 61);
        assert_eq!(
            api.request(START, json!({"request_id":request}), &stale)
                .await
                .0,
            Status::Forbidden
        );
        assert_eq!(deliveries.load(Ordering::SeqCst), 1);
        api.phase(
            START,
            request,
            &gold,
            event,
            official,
            "setup-retry-available",
        )
        .await;
        assert_eq!(deliveries.load(Ordering::SeqCst), 2);
        let row = rows::query(
            pool,
            "SELECT g.attempt, g.claim_deadline
            FROM sequent_backend.operational_setup_generation g
            WHERE g.tenant_id=$1 AND g.official=$2",
            &[&tenant, &official],
        )
        .await;
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].get::<_, Uuid>(0), attempt);
        assert_eq!(row[0].get::<_, DateTime<Utc>>(1), deadline);

        // Device CAS is granted to a dedicated login in the real runtime.
        // Call the same DB function once to simulate a claimed invocation;
        // HTTP work must not redeliver it or convert it into an approval.
        let claimed = rows::query(pool,"SELECT fence FROM sequent_backend.operational_initialize_claim_self_setup($1,$2,$3,$4)",
            &[&tenant,&event,&official,&attempt]).await;
        assert_eq!(claimed.len(), 1);
        api.phase(START, request, &gold, event, official, "setup-pending")
            .await;
        assert_eq!(deliveries.load(Ordering::SeqCst), 2);
        let rows = rows::query(pool,"SELECT count(*) FROM sequent_backend.signing_approval WHERE request_id=$1", &[&request]).await;
        assert_eq!(rows[0].get::<_, i64>(0), 0);
        peer.abort();
    }

    #[test]
    fn private_dispatcher_rejects_unsafe_origins_without_a_fallback() {
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        for (tenant, event, origin) in [
            (Uuid::nil(), event, "https://127.0.0.1:20443"),
            (tenant, Uuid::nil(), "https://127.0.0.1:20443"),
            (tenant, event, "http://127.0.0.1:20443"),
            (tenant, event, "https://user:password@127.0.0.1:20443"),
            (tenant, event, "https://127.0.0.1:20443/?elsewhere=1"),
            (tenant, event, "https://127.0.0.1:20443/#elsewhere"),
            (tenant, event, "https://127.0.0.1:20443/a"),
            (tenant, event, "not a URL"),
        ] {
            let (root, identity) = local_identity();
            assert!(SetupDispatcher::new(
                tenant, event, origin, root, identity
            )
            .is_none());
        }
    }
}
