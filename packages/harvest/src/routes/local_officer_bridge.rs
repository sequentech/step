// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! The actual normal Portal guard authenticates each person separately. No
//! bearer, authorization header, private key, Gold callback or role hint is
//! kept in the rendezvous or copied between their browser sessions.
use super::super::{caller, local::protected, SigningReply};
use super::ingress::SetupSession;
use super::transport::SetupServices;
use super::{HarvestServices, LocalInitializePost, SigningError};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use deadpool_postgres::{Manager, Pool};
use openssl::x509::X509;
use rocket::{serde::json::Json, State};
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_postgres::{NoTls, Row};
use uuid::Uuid;
use windmill::services::signing::local_initialize_attachment::{
    AttachmentEvidence, Binding, CurrentNominee, IssuedLeaf, LocalAttachment,
};

#[path = "local_officer_window.rs"]
mod window;
use window::Window;

const OFFICER: &str = "felix3_local_attachment_officer";
const B: &str =
    "jsonb_populate_record(NULL::felix3_initialize_setup.binding,$1)";
fn refused() -> SigningError {
    SigningError::forbidden(
        "Certificate review is unavailable. Recheck both signed-in sessions and the original request.",
    )
}
fn failed<T>(_: T) -> SigningError {
    refused()
}
fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    installation: Uuid,
    database: String,
    registry_login: String,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct Key {
    request: Uuid,
    official: Uuid,
    digest: String,
}
#[derive(Clone, Eq, PartialEq)]
struct PublicPins {
    attempt: Uuid,
    enrollment: Uuid,
    csr: String,
    spki: String,
}
struct Facts {
    key: Key,
    bound: Binding,
    officer: Uuid,
    nominee_labels: Vec<String>,
    pins: PublicPins,
    gate: LocalAttachment,
}
struct Seen {
    ctx: Arc<Facts>,
    proof: CurrentNominee,
}
type ReviewedKey = (Key, Uuid, String);
pub(crate) struct Bridge {
    authority: Pool,
    registry: Pool,
    registry_login: String,
    people: Mutex<Window<Key, Seen>>,
    reviews: Mutex<Window<ReviewedKey, PublicPins>>,
    presentations: Mutex<Window<(Key, String), (Uuid, PublicPins)>>,
}

async fn qualified(
    pool: &Pool,
    database: &str,
    login: &str,
    peer: &str,
    functions: &[&str],
) -> Option<()> {
    let db = pool.get().await.ok()?;
    let expected: Vec<String> =
        functions.iter().map(|v| v.to_string()).collect();
    let valid: bool = db
        .query_one(
            "SELECT current_user=session_user AND session_user::text=$1 AND current_database()::text=$2
         AND inet_client_addr() IS NULL AND ($3::text='' OR ($3::text='peer' AND system_user LIKE 'peer:%')
             OR system_user=$3::text)
         AND coalesce((SELECT array_agg(p.proname::text ORDER BY p.proname) FROM pg_proc p
            WHERE p.pronamespace='felix3_initialize_bridge'::regnamespace
              AND has_schema_privilege(session_user,p.pronamespace,'USAGE')
              AND has_function_privilege(session_user,p.oid,'EXECUTE')),ARRAY[]::text[])=$4::text[]",
            &[&login, &database, &peer, &expected],
        )
        .await
        .ok()?
        .try_get(0)
        .ok()?;
    valid.then_some(())
}

impl Bridge {
    pub(in crate::routes::operational_initialize) async fn load(
        root: &Path,
        pin: &LocalInitializePost,
        database: &str,
        setup: &SetupServices,
    ) -> Option<Self> {
        let cfg: Registration = serde_json::from_slice(&protected(
            &root.join("harvest/attachment/registration.json"),
            false,
        )?)
        .ok()?;
        if cfg.installation != pin.installation
            || pin.profile != "softhsm-test"
            || cfg.database != database
            || cfg.database.is_empty()
            || cfg.database.len() > 63
            || cfg.registry_login == OFFICER
            || !cfg.registry_login.starts_with("felix3_local_")
            || cfg.registry_login.len() > 63
            || !cfg.registry_login.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
            })
        {
            return None;
        }
        let socket = root.join("postgres");
        if socket.canonicalize().ok()? != socket || !socket.is_dir() {
            return None;
        }
        let connect = |name: &str| -> Option<Pool> {
            let mut pg = tokio_postgres::Config::new();
            pg.host_path(&socket)
                .port(5432)
                .dbname(database)
                .user(name)
                .options("-c search_path=pg_catalog -c lock_timeout=2000 -c statement_timeout=6000")
                .connect_timeout(Duration::from_secs(3));
            Pool::builder(Manager::new(pg, NoTls))
                .max_size(4)
                .build()
                .ok()
        };
        let authority = connect(OFFICER)?;
        let registry = connect(&cfg.registry_login)?;
        qualified(
            &setup.pool,
            database,
            "felix3_local_setup_portal",
            "",
            &["nominee_recovery"],
        )
        .await?;
        qualified(
            &authority,
            database,
            OFFICER,
            "peer:felix3_local_attachment_officer",
            &["issuer_pins", "officer_scope"],
        )
        .await?;
        qualified(&registry, database, &cfg.registry_login, "peer", &[])
            .await?;
        Some(Self {
            authority,
            registry,
            registry_login: cfg.registry_login,
            people: Mutex::new(Window::new(24, Duration::from_secs(20))),
            reviews: Mutex::new(Window::new(64, Duration::from_secs(600))),
            presentations: Mutex::new(Window::new(
                32,
                Duration::from_secs(300),
            )),
        })
    }
}

fn officer(
    claims: &JwtClaims,
    pin: &LocalInitializePost,
) -> Result<(Uuid, DateTime<Utc>), SigningError> {
    let id = Uuid::parse_str(&claims.sub).map_err(failed)?;
    let now = Utc::now();
    let auth = DateTime::<Utc>::from_timestamp(
        claims.auth_time.ok_or_else(refused)?,
        0,
    )
    .ok_or_else(refused)?;
    if id.is_nil()
        || claims.typ != "Bearer"
        || claims.hasura_claims.user_id != id.to_string()
        || claims.hasura_claims.tenant_id != pin.tenant.to_string()
        || claims.iss
            != format!("http://localhost:18090/realms/tenant-{}", pin.tenant)
        || !claims
            .hasura_claims
            .allowed_roles
            .contains(&Permissions::SIGNING_CERTIFICATES_REGISTER.to_string())
        || claims.acr != Permissions::GOLD.to_string()
        || auth > now
        || now >= auth + ChronoDuration::seconds(60)
        || claims.exp <= now.timestamp()
    {
        return Err(refused());
    }
    sequent_core::services::authorization::authorize(
        claims,
        true,
        Some(pin.tenant.to_string()),
        vec![],
    )
    .map_err(failed)?;
    Ok((id, auth))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PresenceInput {
    request_id: Uuid,
    payload_sha256: String,
}
#[derive(Serialize)]
pub(crate) struct PresenceAck {
    active: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventInput {
    election_event_id: Uuid,
}
#[derive(Clone, Serialize)]
pub(crate) struct Reference {
    request_id: Uuid,
    official_id: Uuid,
    election_id: Uuid,
    payload_sha256: String,
}
impl Reference {
    fn new(c: &Facts) -> Self {
        Self {
            request_id: c.key.request,
            official_id: c.key.official,
            election_id: c.bound.post,
            payload_sha256: c.key.digest.clone(),
        }
    }
}
#[derive(Serialize)]
pub(crate) struct Queue {
    items: Vec<Reference>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewInput {
    request_id: Uuid,
    official_id: Uuid,
    payload_sha256: String,
}
fn key(
    request: Uuid,
    official: Uuid,
    sha256: &str,
) -> Result<Key, SigningError> {
    if request.is_nil() || official.is_nil() || !sha(sha256) {
        return Err(refused());
    }
    Ok(Key {
        request,
        official,
        digest: sha256.to_string(),
    })
}
#[derive(Serialize)]
pub(crate) struct PublicReview {
    #[serde(flatten)]
    reference: Reference,
    enrollment: Uuid,
    csr_pem: String,
    csr_sha256: String,
    spki_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttachInput {
    request_id: Uuid,
    official_id: Uuid,
    payload_sha256: String,
    csr_sha256: String,
    leaf_sha256: String,
    leaf_der: String,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum NomineeReviewPhase {
    Unavailable,
    GoldRequired,
    ResumeReady,
    Ready,
}

#[derive(Serialize)]
pub(crate) struct NomineeReviewStatus {
    phase: NomineeReviewPhase,
}

struct NomineeGeneration {
    officer: Uuid,
    pins: PublicPins,
    needs_gold: bool,
    authenticated_after: DateTime<Utc>,
    certificate: Option<Uuid>,
}

fn nomination_binding(
    claims: &JwtClaims,
    pin: &LocalInitializePost,
    input: &PresenceInput,
) -> Result<(Key, Binding), SigningError> {
    let (tenant, _) = caller(claims)?;
    let official = Uuid::parse_str(&claims.sub).map_err(failed)?;
    let k = key(input.request_id, official, &input.payload_sha256)?;
    if pin.profile != "softhsm-test" || tenant != pin.tenant {
        return Err(refused());
    }
    let bound = Binding {
        installation: pin.installation,
        tenant,
        event: pin.event,
        post: pin.post,
        publication: pin.publication,
        request: k.request,
        official,
        payload_sha256: k.digest.clone(),
    };
    Ok((k, bound))
}

async fn nominee_generation(
    setup: &SetupServices,
    bound: &Binding,
    sid: &str,
) -> Result<Option<NomineeGeneration>, SigningError> {
    let wire = serde_json::to_value(bound).map_err(failed)?;
    let sql = format!("SELECT attempt,officer,enrollment,csr_sha256,spki_sha256,needs_gold,
        authenticated_after,certificate FROM felix3_initialize_bridge.nominee_recovery({B},$2)");
    let db = setup.pool.get().await.map_err(failed)?;
    let row = db
        .query_opt(sql.as_str(), &[&wire, &sid])
        .await
        .map_err(failed)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let officer: Uuid = row.try_get("officer").map_err(failed)?;
    if officer.is_nil() || officer == bound.official {
        return Err(refused());
    }
    Ok(Some(NomineeGeneration {
        officer,
        pins: PublicPins {
            attempt: row.try_get("attempt").map_err(failed)?,
            enrollment: row.try_get("enrollment").map_err(failed)?,
            csr: row.try_get("csr_sha256").map_err(failed)?,
            spki: row.try_get("spki_sha256").map_err(failed)?,
        },
        needs_gold: row.try_get("needs_gold").map_err(failed)?,
        authenticated_after: row
            .try_get("authenticated_after")
            .map_err(failed)?,
        certificate: row.try_get("certificate").map_err(failed)?,
    }))
}

fn presentation_is_current(
    bridge: &Bridge,
    k: &Key,
    sid: &str,
    origin: &NomineeGeneration,
) -> Result<bool, SigningError> {
    if !origin.needs_gold {
        return Ok(true);
    }
    Ok(bridge.presentations.lock().map_err(failed)?.matches(
        &(k.clone(), sid.to_owned()),
        Instant::now(),
        |(officer, pins)| *officer == origin.officer && pins == &origin.pins,
    ))
}

// Compare the signed NumericDate at its original seconds resolution. A Gold
// event in the same second as generation or request creation cannot establish
// that it followed them. The already verified Portal JWT remains the only
// source of auth_time, issued time and actor; browser JSON cannot assert Gold.
fn recovery_gold(
    claims: &JwtClaims,
    after: DateTime<Utc>,
    now: DateTime<Utc>,
) -> bool {
    let now = now.timestamp();
    claims.acr == Permissions::GOLD.to_string()
        && claims.auth_time.is_some_and(|auth| {
            after.timestamp() < auth
                && now.saturating_sub(60) < auth
                && auth <= claims.iat
                && claims.iat <= now
                && now < claims.exp
        })
}

#[rocket::post(
    "/operational-signing/initialize/attachment/nominee-review-status",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn nominee_status(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    bridge: &State<Bridge>,
    input: Json<PresenceInput>,
) -> SigningReply<NomineeReviewStatus> {
    let (verified, sid) = session.bridge_identity();
    let (k, bound) = nomination_binding(&verified.0, pin, &input)?;
    super::checked_scope(app, pin, verified, k.request, &k.digest).await?;
    let phase = match nominee_generation(setup, &bound, sid).await? {
        Some(origin) if origin.certificate.is_none() => {
            if presentation_is_current(bridge, &k, sid, &origin)? {
                NomineeReviewPhase::Ready
            } else if recovery_gold(
                &verified.0,
                origin.authenticated_after,
                Utc::now(),
            ) {
                NomineeReviewPhase::ResumeReady
            } else {
                NomineeReviewPhase::GoldRequired
            }
        }
        _ => NomineeReviewPhase::Unavailable,
    };
    Ok(Json(NomineeReviewStatus { phase }))
}

#[rocket::post(
    "/operational-signing/initialize/attachment/nominee-resume",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn nominee_resume(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    bridge: &State<Bridge>,
    input: Json<PresenceInput>,
) -> SigningReply<PresenceAck> {
    let at = Instant::now();
    let (verified, sid) = session.bridge_identity();
    let (k, bound) = nomination_binding(&verified.0, pin, &input)?;
    super::checked_scope(app, pin, verified, k.request, &k.digest).await?;
    let origin = nominee_generation(setup, &bound, sid)
        .await?
        .ok_or_else(refused)?;
    if origin.certificate.is_some()
        || !origin.needs_gold
        || !recovery_gold(&verified.0, origin.authenticated_after, Utc::now())
    {
        return Err(refused());
    }
    // A presentation lease grants no officer operation. Every explicit or
    // renewed presence independently rechecks the current actual request,
    // realm signer, Post, permit and original enrollment and creates a
    // distinct short-lived, move-only continuation.
    if !bridge.presentations.lock().map_err(failed)?.put(
        (k, sid.to_owned()),
        (origin.officer, origin.pins),
        at,
        Instant::now(),
    ) {
        return Err(refused());
    }
    Ok(Json(PresenceAck { active: true }))
}

// The same transaction used to prepare the real action executes the Post and
// live Keycloak check. Then release its locks BEFORE the distinct 2250 role
// performs any officer work. The hash-only lookup never reaches the browser.
#[rocket::post(
    "/operational-signing/initialize/attachment/presence",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn presence(
    session: SetupSession,
    app: &State<HarvestServices>,
    pin: &State<LocalInitializePost>,
    setup: &State<SetupServices>,
    bridge: &State<Bridge>,
    input: Json<PresenceInput>,
) -> SigningReply<PresenceAck> {
    let at = Instant::now();
    let (verified, sid) = session.bridge_identity();
    let claims = &verified.0;
    let (_, actor) = caller(claims)?;
    let (k, bound) = nomination_binding(claims, pin, &input)?;
    let origin = nominee_generation(setup, &bound, sid)
        .await?
        .ok_or_else(refused)?;
    if !presentation_is_current(bridge, &k, sid, &origin)? {
        return Err(refused());
    }
    let responsible = origin.officer;
    let pins = &origin.pins;
    // The shared verified guard already pinned this azp to its immutable
    // ordinary Admin client registration. No per-user client is configured.
    let gate = LocalAttachment::from_installation(
        "http://localhost:3002",
        pin.profile,
        bound.clone(),
        pins.attempt,
        pins.enrollment,
        pins.csr.clone(),
        pins.spki.clone(),
        claims.azp.clone(),
        bridge.registry_login.clone(),
        bridge.authority.clone(),
        bridge.registry.clone(),
    )
    .map_err(failed)?;
    let mut backend =
        app.databases.hasura().await.get().await.map_err(failed)?;
    let tx = backend.transaction().await.map_err(failed)?;
    let txref = &tx;
    let actorref = &actor;
    let proof = gate
        .current_nominee(&tx, claims, |scope| async move {
            pin.verify(txref, app, actorref, &scope).await.map_err(|_| {
                anyhow::anyhow!("current Post eligibility unavailable")
            })
        })
        .await
        .map_err(failed)?;
    tx.commit().await.map_err(failed)?;
    if !presentation_is_current(bridge, &k, sid, &origin)? {
        return Err(refused());
    }
    let ctx = Arc::new(Facts {
        key: k.clone(),
        bound,
        officer: responsible,
        nominee_labels: decode_permission_labels(claims),
        pins: origin.pins,
        gate,
    });
    if !bridge.people.lock().map_err(failed)?.put(
        k,
        Seen { ctx, proof },
        at,
        Instant::now(),
    ) {
        return Err(refused());
    }
    Ok(Json(PresenceAck { active: true }))
}

async fn scoped_row(
    bridge: &Bridge,
    ctx: &Facts,
    claims: &JwtClaims,
    who: Uuid,
    authenticated: DateTime<Utc>,
    trust: bool,
) -> Result<Option<Row>, SigningError> {
    if who != ctx.officer || who == ctx.bound.official {
        return Err(refused());
    }
    let wire = serde_json::to_value(&ctx.bound).map_err(failed)?;
    let role_labels = decode_permission_labels(claims);
    let call = if trust {
        "issuer_pins"
    } else {
        "officer_scope"
    }; // closed, never supplied by the client
    let sql = format!(
        "SELECT * FROM felix3_initialize_bridge.{call}({B},$2,$3,$4,$5,$6)"
    );
    let db = bridge.authority.get().await.map_err(failed)?;
    db.query_opt(
        sql.as_str(),
        &[
            &wire,
            &ctx.pins.attempt,
            &who,
            &role_labels,
            &ctx.nominee_labels,
            &authenticated,
        ],
    )
    .await
    .map_err(failed)
}

#[rocket::post(
    "/operational-signing/initialize/attachment/pending",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn pending(
    session: SetupSession,
    pin: &State<LocalInitializePost>,
    bridge: &State<Bridge>,
    input: Json<EventInput>,
) -> SigningReply<Queue> {
    let (verified, _) = session.bridge_identity();
    let claims = &verified.0;
    let (who, authenticated) = officer(claims, pin)?;
    if input.election_event_id != pin.event {
        return Err(refused());
    }
    let candidates =
        bridge
            .people
            .lock()
            .map_err(failed)?
            .visible(Instant::now(), |s| {
                (s.ctx.officer == who && s.ctx.bound.event == pin.event)
                    .then(|| s.ctx.clone())
            });
    let mut items = Vec::new();
    for ctx in candidates {
        if let Some(row) =
            scoped_row(bridge, &ctx, claims, who, authenticated, false).await?
        {
            if row
                .try_get::<_, Option<Uuid>>("certificate")
                .map_err(failed)?
                .is_none()
                && row.try_get::<_, Uuid>("request_id").map_err(failed)?
                    == ctx.key.request
                && row.try_get::<_, Uuid>("official_id").map_err(failed)?
                    == ctx.key.official
                && row.try_get::<_, Uuid>("election_id").map_err(failed)?
                    == pin.post
                && row.try_get::<_, String>("payload_sha256").map_err(failed)?
                    == ctx.key.digest
            {
                items.push(Reference::new(&ctx));
            }
        }
    }
    officer(claims, pin)?;
    // Queue reads take SQL locks and can outlive their initial snapshot.
    // Remove entries whose presence expired or was consumed in the interim.
    let now = Instant::now();
    let mut current = bridge.people.lock().map_err(failed)?;
    items.retain(|r| {
        let k = Key {
            request: r.request_id,
            official: r.official_id,
            digest: r.payload_sha256.clone(),
        };
        current.matches(&k, now, |s| {
            s.ctx.officer == who && s.ctx.bound.post == r.election_id
        })
    });
    items.sort_by_key(|r| (r.request_id, r.official_id));
    Ok(Json(Queue { items }))
}

fn consume(
    bridge: &Bridge,
    pin: &LocalInitializePost,
    k: &Key,
    who: Uuid,
) -> Result<Seen, SigningError> {
    bridge
        .people
        .lock()
        .map_err(failed)?
        .take(k, Instant::now(), |s| {
            s.ctx.officer == who
                && s.ctx.bound.official != who
                && s.ctx.bound.installation == pin.installation
                && s.ctx.bound.tenant == pin.tenant
                && s.ctx.bound.event == pin.event
                && s.ctx.bound.post == pin.post
                && s.ctx.bound.publication == pin.publication
        })
        .ok_or_else(refused)
}

#[rocket::post(
    "/operational-signing/initialize/attachment/review",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn review(
    session: SetupSession,
    pin: &State<LocalInitializePost>,
    bridge: &State<Bridge>,
    input: Json<ReviewInput>,
) -> SigningReply<PublicReview> {
    let (verified, sid) = session.bridge_identity();
    let claims = &verified.0;
    let (who, _) = officer(claims, pin)?;
    let k = key(input.request_id, input.official_id, &input.payload_sha256)?;
    let Seen { ctx, proof } = consume(bridge, pin, &k, who)?;
    let csr = ctx.gate.review_csr(claims, proof).await.map_err(failed)?;
    if digest(csr.as_bytes()) != ctx.pins.csr {
        return Err(refused());
    }
    officer(claims, pin)?;
    let at = Instant::now();
    if !bridge.reviews.lock().map_err(failed)?.put(
        (k, who, sid.to_owned()),
        ctx.pins.clone(),
        at,
        at,
    ) {
        return Err(refused());
    }
    Ok(Json(PublicReview {
        reference: Reference::new(&ctx),
        enrollment: ctx.pins.enrollment,
        csr_pem: csr,
        csr_sha256: ctx.pins.csr.clone(),
        spki_sha256: ctx.pins.spki.clone(),
    }))
}

fn single_public_leaf(
    text: &str,
    reviewed: &str,
) -> Result<Vec<u8>, SigningError> {
    if !sha(reviewed) || text.len() > 43692 || text.is_empty() {
        return Err(refused());
    }
    let der = STANDARD.decode(text).map_err(failed)?;
    if !(1..=32768).contains(&der.len())
        || STANDARD.encode(&der) != text
        || digest(&der) != reviewed
    {
        return Err(refused());
    }
    let cert = X509::from_der(&der).map_err(failed)?;
    if cert.to_der().map_err(failed)? != der {
        return Err(refused());
    }
    Ok(der)
}

#[rocket::post(
    "/operational-signing/initialize/attachment/attach",
    format = "json",
    data = "<input>"
)]
pub(crate) async fn attach(
    session: SetupSession,
    pin: &State<LocalInitializePost>,
    bridge: &State<Bridge>,
    input: Json<AttachInput>,
) -> SigningReply<AttachmentEvidence> {
    let (verified, sid) = session.bridge_identity();
    let claims = &verified.0;
    let (who, authenticated) = officer(claims, pin)?;
    let k = key(input.request_id, input.official_id, &input.payload_sha256)?;
    if !sha(&input.csr_sha256) {
        return Err(refused());
    }
    let der = single_public_leaf(&input.leaf_der, &input.leaf_sha256)?;
    let Seen { ctx, proof } = consume(bridge, pin, &k, who)?;
    let rk = (k, who, sid.to_owned());
    if input.csr_sha256 != ctx.pins.csr
        || !bridge.reviews.lock().map_err(failed)?.matches(
            &rk,
            Instant::now(),
            |p| p == &ctx.pins,
        )
    {
        return Err(refused());
    }
    let trust = scoped_row(bridge, &ctx, claims, who, authenticated, true)
        .await?
        .ok_or_else(refused)?;
    let issued = IssuedLeaf {
        der,
        leaf_sha256: input.leaf_sha256.clone(),
        issuer: trust.try_get("issuer").map_err(failed)?,
        root_sha256: trust.try_get("root_sha256").map_err(failed)?,
        crl_sha256: trust.try_get("crl_sha256").map_err(failed)?,
    };
    let receipt = ctx
        .gate
        .register_and_attach(
            claims,
            proof,
            &issued,
            &input.csr_sha256,
            &input.leaf_sha256,
        )
        .await
        .map_err(failed)?;
    Ok(Json(receipt))
}
