// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot-store views and separately authorized private investigation actions.
//! Public responses never contain a voter identity or personalized cache data.
use crate::routes::ballot_checks::ensure_checks_open_with_pool;
use crate::services::authorization::{
    authorize, authorize_voter_event, cast_vote_read_elections,
};
use crate::services::dependencies::HarvestServices;
use anyhow::Result;
use base64::{
    engine::general_purpose::STANDARD,
    engine::general_purpose::URL_SAFE_NO_PAD, Engine,
};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::adapters::privacy::{BallotIdentity, PrivacyBallotQuery};
use electoral_log::domain::{Filter, LogQuery, SqlCompOperators, TextColumn};
use electoral_log::messages::{
    message::{Message, SigningData},
    privacy::{ForensicRead, ForensicReadOperation, PseudonymResolution},
};
use electoral_log::ports::ElectoralLogStore;
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::ballot::ShowCastVoteLogs;
use sequent_core::ballot_privacy::{BallotPrivacyProof, BallotPrivacyTrust};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use sequent_core::types::vote_privacy::{
    NetworkDataPolicy, PseudonymResolutionReason,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use windmill::services::{
    election_event_board::get_election_event_board,
    electoral_log_board::{get_board_signing_key, get_event_store},
};

type ApiError = (Status, String);
fn unavailable(_: impl std::fmt::Display) -> ApiError {
    (
        Status::InternalServerError,
        "Unable to read ballot privacy data".into(),
    )
}
fn bad(message: &str) -> ApiError {
    (Status::BadRequest, message.into())
}
fn forbidden() -> ApiError {
    (
        Status::Forbidden,
        "Not authorized for this privacy operation".into(),
    )
}

#[derive(Serialize)]
pub struct PrivacyPayload {
    pub payload: Value,
}
fn payload(value: impl Serialize) -> Result<Json<PrivacyPayload>, ApiError> {
    Ok(Json(PrivacyPayload {
        payload: serde_json::to_value(value).map_err(unavailable)?,
    }))
}

#[derive(Deserialize)]
pub struct BallotLogInput {
    election_event_id: String,
    election_id: Option<String>,
    prefix: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}
#[derive(Deserialize)]
pub struct BallotEntryInput {
    election_event_id: String,
    election_id: String,
    ballot_id: String,
}
#[derive(Deserialize)]
pub struct BallotKeysInput {
    election_event_id: String,
    election_id: String,
    ballot_id: Option<String>,
}

async fn event(
    services: &HarvestServices,
    claims: &JwtClaims,
    id: &str,
) -> Result<ElectionEvent, ApiError> {
    uuid::Uuid::parse_str(id).map_err(|_| bad("Invalid event"))?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    windmill::postgres::election_event::get_election_event_by_id_if_exist(
        &tx,
        &claims.hasura_claims.tenant_id,
        id,
    )
    .await
    .map_err(unavailable)?
    .ok_or_else(|| (Status::NotFound, "Event not found".into()))
}

fn voter_token_scope(
    claims: &JwtClaims,
    event: &str,
    election: Option<&str>,
) -> Result<(String, Vec<String>), ApiError> {
    let (area, mut elections) = authorize_voter_event(
        claims,
        vec![VoterPermissions::CAST_VOTE],
        event,
    )?;
    if let Some(election) = election {
        if !elections.iter().any(|allowed| allowed == election) {
            return Err(forbidden());
        }
        elections.retain(|allowed| allowed == election);
    }
    elections.sort();
    elections.dedup();
    Ok((area, elections))
}

fn proof_token_scope(
    claims: &JwtClaims,
    event: &str,
    election: &str,
) -> Result<(), ApiError> {
    if claims.hasura_claims.election_event_id.is_some() {
        voter_token_scope(claims, event, Some(election))?;
    } else {
        authorize(
            claims,
            false,
            Some(claims.hasura_claims.tenant_id.clone()),
            vec![Permissions::LOGS_READ],
        )?;
    }
    Ok(())
}

async fn voter_scope(
    services: &HarvestServices,
    claims: &JwtClaims,
    event: &ElectionEvent,
    election: Option<&str>,
) -> Result<(String, Vec<String>), ApiError> {
    if claims.hasura_claims.tenant_id != event.tenant_id {
        return Err(forbidden());
    }
    let (area, elections) = voter_token_scope(claims, &event.id, election)?;
    let presentation = event
        .get_presentation()
        .map_err(unavailable)?
        .unwrap_or_default();
    if presentation.show_cast_vote_logs.unwrap_or_default()
        != ShowCastVoteLogs::ShowLogsTab
    {
        return Err(forbidden());
    }
    let pool = services.databases.hasura().await;
    ensure_checks_open_with_pool(
        &pool,
        &claims.hasura_claims.tenant_id,
        &event.id,
    )
    .await?;
    Ok((area, elections))
}

/// The encoded cursor carries its exact query scope, so changing filters cannot
/// silently continue a previous page. It grants no authority; every read rechecks claims.
fn scope_key(
    event: &str,
    area: &str,
    elections: &[String],
    prefix: &str,
) -> String {
    json!([event, area, elections, prefix]).to_string()
}
fn encode_cursor(scope: &str, seq: i64) -> String {
    URL_SAFE_NO_PAD.encode(
        json!([hex::encode(Sha256::digest(scope.as_bytes())), seq]).to_string(),
    )
}
fn decode_cursor(
    scope: &str,
    cursor: Option<&str>,
) -> Result<Option<i64>, ApiError> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };
    if cursor.len() > 4096 {
        return Err(bad("Invalid cursor"));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| bad("Invalid cursor"))?;
    let (saved, seq): (String, i64) =
        serde_json::from_slice(&bytes).map_err(|_| bad("Invalid cursor"))?;
    if saved != hex::encode(Sha256::digest(scope.as_bytes())) || seq < 1 {
        return Err(bad("Cursor does not belong to this query"));
    }
    Ok(Some(seq))
}
fn prefix(input: Option<&str>) -> Result<String, ApiError> {
    let code = input
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase()
        .replace('-', "");
    if code.len() > 8
        || !code
            .bytes()
            .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c))
    {
        return Err(bad("Invalid Ballot ID or pseudonym prefix"));
    }
    Ok(code)
}

type CachedTotal = Arc<Mutex<Option<(Instant, i64)>>>;
static TOTALS: OnceLock<Mutex<HashMap<String, CachedTotal>>> = OnceLock::new();
async fn cached_total(
    store: &PostgresStore,
    tenant: &str,
    event: &str,
    area: &str,
    election: Option<&str>,
    elections: &[String],
) -> Result<i64> {
    let key = page_cache_key(tenant, event, area, elections, 0);
    let entry = {
        let mut entries = TOTALS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .await;
        if entries.len() >= 2048 && !entries.contains_key(&key) {
            entries.clear();
        }
        entries
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    };
    let mut cached = entry.lock().await;
    if let Some((when, count)) = *cached {
        if when.elapsed() < Duration::from_secs(5) {
            return Ok(count);
        }
    }
    let count = store
        .privacy_ballot_count(event, area, election, Some(elections))
        .await?;
    *cached = Some((Instant::now(), count));
    Ok(count)
}
type CachedPage = Arc<Mutex<Option<(Instant, Value)>>>;
static PAGES: OnceLock<Mutex<HashMap<String, CachedPage>>> = OnceLock::new();
fn page_cache_key(
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
    limit: i64,
) -> String {
    json!([tenant, event, area, elections, limit]).to_string()
}
async fn page_cache_entry(key: String) -> CachedPage {
    let mut entries = PAGES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .await;
    if entries.len() >= 2048 && !entries.contains_key(&key) {
        entries.clear();
    }
    entries
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(None)))
        .clone()
}
async fn limit_search(
    claims: &JwtClaims,
    store: &PostgresStore,
) -> Result<(), ApiError> {
    let key = crate::services::ballot_log_cache::search_session(claims);
    let client = store.client().await.map_err(unavailable)?;
    let accepted = client.query_opt(
        "WITH expired AS (
            DELETE FROM sequent_backend.ballot_log_search_limit WHERE session_key IN
              (SELECT session_key FROM sequent_backend.ballot_log_search_limit
               WHERE searched_at < clock_timestamp() - interval '1 day' AND session_key <> $1 LIMIT 100)
         )
         INSERT INTO sequent_backend.ballot_log_search_limit (session_key, searched_at)
         VALUES ($1,clock_timestamp()) ON CONFLICT (session_key) DO UPDATE
         SET searched_at=EXCLUDED.searched_at
         WHERE ballot_log_search_limit.searched_at <= EXCLUDED.searched_at - interval '200 milliseconds'
         RETURNING 1", &[&key]).await.map_err(unavailable)?;
    if accepted.is_none() {
        return Err((
            Status::TooManyRequests,
            "Wait before searching again".into(),
        ));
    }
    Ok(())
}

#[derive(rocket::Responder)]
#[response(status = 204)]
pub struct BallotCacheAuthorization {
    body: (),
    scope: rocket::http::Header<'static>,
    cache_control: rocket::http::Header<'static>,
}

/// Called by the internal proxy on every request, including cache hits. No
/// request body, voter identity or token is included in the shared cache key.
#[get("/ballot-box-log-authorize")]
pub async fn ballot_box_log_authorize(
    services: &State<HarvestServices>,
    claims: JwtClaims,
) -> Result<BallotCacheAuthorization, ApiError> {
    let event_id = claims
        .hasura_claims
        .election_event_id
        .as_deref()
        .ok_or_else(forbidden)?;
    voter_token_scope(&claims, event_id, None)?;
    let event = event(services, &claims, event_id).await?;
    let (area, elections) =
        voter_scope(services, &claims, &event, None).await?;
    if event
        .get_presentation()
        .map_err(unavailable)?
        .unwrap_or_default()
        .ballot_identity
        != Some(sequent_core::types::vote_privacy::BallotIdentity::Pseudonym)
    {
        return Err(forbidden());
    }
    Ok(BallotCacheAuthorization {
        body: (),
        scope: rocket::http::Header::new(
            "X-Ballot-Scope",
            crate::services::ballot_log_cache::scope(
                &event.tenant_id,
                &event.id,
                &area,
                &elections,
            ),
        ),
        cache_control: rocket::http::Header::new("Cache-Control", "no-store"),
    })
}

#[derive(Clone, Copy)]
enum BallotPageCache {
    Shared,
    Bypass,
}

#[derive(rocket::Responder)]
pub struct BallotPageResponse {
    body: Json<PrivacyPayload>,
    cacheable: rocket::http::Header<'static>,
    cache_control: rocket::http::Header<'static>,
}

fn page_payload(
    value: Value,
    cache: BallotPageCache,
) -> Result<BallotPageResponse, ApiError> {
    Ok(BallotPageResponse {
        body: payload(value)?,
        cacheable: rocket::http::Header::new(
            "X-Ballot-Cacheable",
            match cache {
                BallotPageCache::Shared => "1",
                BallotPageCache::Bypass => "0",
            },
        ),
        // Only the internal proxy may override this for its authorized cache.
        cache_control: rocket::http::Header::new(
            "Cache-Control",
            "private, no-store",
        ),
    })
}

#[post("/ballot-box-log", format = "json", data = "<body>")]
pub async fn ballot_box_log(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<BallotLogInput>,
) -> Result<BallotPageResponse, ApiError> {
    voter_token_scope(
        &claims,
        &body.election_event_id,
        body.election_id.as_deref(),
    )?;
    let event = event(services, &claims, &body.election_event_id).await?;
    let (area, elections) =
        voter_scope(services, &claims, &event, body.election_id.as_deref())
            .await?;
    let prefix = prefix(body.prefix.as_deref())?;
    let scope = scope_key(&event.id, &area, &elections, &prefix);
    let before = decode_cursor(&scope, body.cursor.as_deref())?;
    let limit = body.limit.unwrap_or(25);
    if !(1..=100).contains(&limit) {
        return Err(bad("Page size must be between 1 and 100"));
    }
    let store = get_event_store(&event.id).await.map_err(unavailable)?;
    if !prefix.is_empty() {
        limit_search(&claims, &store).await?;
    }
    let cached = if prefix.is_empty() && before.is_none() {
        Some(
            page_cache_entry(page_cache_key(
                &event.tenant_id,
                &event.id,
                &area,
                &elections,
                limit,
            ))
            .await,
        )
    } else {
        None
    };
    // Hold the per-scope lock through the query so concurrent voters share one fill.
    let mut cached = match &cached {
        Some(entry) => Some(entry.lock().await),
        None => None,
    };
    if let Some(entry) = &cached {
        if let Some((when, value)) = &**entry {
            if when.elapsed() < Duration::from_secs(3) {
                return page_payload(value.clone(), BallotPageCache::Shared);
            }
        }
    }
    let page = store
        .privacy_ballot_page(&PrivacyBallotQuery {
            event: &event.id,
            area: &area,
            election: body.election_id.as_deref(),
            elections: Some(&elections),
            prefix: Some(&prefix),
            before,
            limit,
        })
        .await
        .map_err(unavailable)?;
    let total = cached_total(
        &store,
        &event.tenant_id,
        &event.id,
        &area,
        body.election_id.as_deref(),
        &elections,
    )
    .await
    .map_err(unavailable)?;
    let response = json!({"ballots": page.ballots, "next_cursor": page.next_cursor.map(|seq| encode_cursor(&scope, seq)),
        "total":total, "match_count": page.match_count.unwrap_or(total), "match_count_capped":page.match_count_capped});
    if let Some(entry) = &mut cached {
        **entry = Some((Instant::now(), response.clone()));
    }
    page_payload(
        response,
        if prefix.is_empty() && before.is_none() {
            BallotPageCache::Shared
        } else {
            BallotPageCache::Bypass
        },
    )
}

#[post("/ballot-box-own", format = "json", data = "<body>")]
pub async fn ballot_box_own(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<BallotLogInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    voter_token_scope(
        &claims,
        &body.election_event_id,
        body.election_id.as_deref(),
    )?;
    let event = event(services, &claims, &body.election_event_id).await?;
    let (area, elections) =
        voter_scope(services, &claims, &event, body.election_id.as_deref())
            .await?;
    let store = get_event_store(&event.id).await.map_err(unavailable)?;
    if store
        .ballot_identity(&event.id)
        .await
        .map_err(unavailable)?
        != BallotIdentity::Pseudonym
    {
        return Err(forbidden());
    }
    let mapping = store.client().await.map_err(unavailable)?.query_opt(
        "SELECT i.mapping_purged_at IS NOT NULL, m.pseudonym FROM sequent_backend.ballot_identity i \
         LEFT JOIN sequent_backend.voter_pseudonym m ON m.election_event_id=i.election_event_id AND m.voter_id=$2 \
         WHERE i.election_event_id=$1::text::uuid",
        &[&event.id, &claims.hasura_claims.user_id],
    ).await.map_err(unavailable)?.ok_or_else(|| unavailable("Identity mode unavailable"))?;
    if mapping.try_get::<_, bool>(0).map_err(unavailable)? {
        return payload(json!({"ballots":[]}));
    }
    let identity: String = mapping
        .try_get::<_, Option<String>>(1)
        .map_err(unavailable)?
        .ok_or_else(|| {
            unavailable("Voter privacy registration is not ready")
        })?;
    let rows = store.client().await.map_err(unavailable)?.query(
        "SELECT r.ballot_id,r.voter_id,b.accepted_at,b.voting_channel,b.election_id::text FROM ballot_box_ballot b \
         JOIN ballot_box_received r ON r.election_event_id=b.election_event_id AND r.election_id=b.election_id AND r.voter_id=b.voter_id AND r.ballot_hash=b.ballot_id \
         WHERE b.election_event_id=$1::text::uuid AND b.area_id=$2::text::uuid AND b.voter_id=$3 \
         AND b.election_id::text=ANY($4) AND b.status='valid' ORDER BY b.seq DESC LIMIT 100", &[&event.id,&area,&identity,&elections]).await.map_err(unavailable)?;
    let ballots = rows.iter().map(|r| Ok(json!({"ballot_id":r.try_get::<_,String>(0)?,"pseudonym":r.try_get::<_,String>(1)?,
        "cast_at":r.try_get::<_,chrono::DateTime<chrono::Utc>>(2)?.to_rfc3339(),"voting_channel":r.try_get::<_,String>(3)?,"election_id":r.try_get::<_,String>(4)?})))
        .collect::<Result<Vec<_>,tokio_postgres::Error>>().map_err(unavailable)?;
    payload(json!({"ballots":ballots}))
}

async fn proof_scope(
    services: &HarvestServices,
    claims: &JwtClaims,
    event: &ElectionEvent,
    election: &str,
) -> Result<Option<String>, ApiError> {
    if claims.hasura_claims.election_event_id.is_some() {
        let (area, _) =
            voter_scope(services, claims, event, Some(election)).await?;
        return Ok(Some(area));
    }
    authorize(
        claims,
        false,
        Some(event.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    cast_vote_read_elections(&tx, claims, &event.id, Some(election)).await?;
    Ok(None)
}

fn ensure_proof_receipt_visible(
    voter: bool,
    status: electoral_log::adapters::ballot_box_received::ReceivedStatus,
) -> Result<(), ApiError> {
    if voter && status != electoral_log::adapters::ballot_box_received::ReceivedStatus::Cast {
        return Err((Status::NotFound,"Ballot not found".into()));
    }
    Ok(())
}
fn proof_channel(
    status: electoral_log::adapters::ballot_box_received::ReceivedStatus,
    channel: Option<String>,
) -> Result<String, ApiError> {
    match channel {
        Some(channel) => Ok(channel),
        None if status != electoral_log::adapters::ballot_box_received::ReceivedStatus::Cast => Ok(String::new()),
        None => Err(unavailable("Cast ballot channel missing")),
    }
}

#[post("/ballot-box-entry", format = "json", data = "<body>")]
pub async fn ballot_box_entry(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<BallotEntryInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    proof_token_scope(&claims, &body.election_event_id, &body.election_id)?;
    let event = event(services, &claims, &body.election_event_id).await?;
    let allowed_area =
        proof_scope(services, &claims, &event, &body.election_id).await?;
    let store = get_event_store(&event.id).await.map_err(unavailable)?;
    let code = electoral_log::adapters::privacy::normalize_privacy_code(
        &body.ballot_id,
    )
    .ok_or_else(|| bad("Invalid Ballot ID"))?;
    if store
        .ballot_identity(&event.id)
        .await
        .map_err(unavailable)?
        != BallotIdentity::Pseudonym
    {
        return Err(forbidden());
    }
    let row=store.client().await.map_err(unavailable)?.query_opt(
        "SELECT area_id::text,voter_id FROM ballot_box_received WHERE election_event_id=$1::text::uuid AND election_id=$2::text::uuid AND ballot_id=$3 AND ($4::text IS NULL OR area_id=$4::text::uuid)",
        &[&event.id,&body.election_id,&code,&allowed_area]).await.map_err(unavailable)?
        .ok_or_else(|| (Status::NotFound,"Ballot not found".into()))?;
    let area: String = row.try_get(0).map_err(unavailable)?;
    let pseudonym: String = row.try_get(1).map_err(unavailable)?;
    let received = store
        .received_ballot_by_id(&event.id, &body.election_id, &pseudonym, &code)
        .await
        .map_err(unavailable)?
        .ok_or_else(|| (Status::NotFound, "Ballot not found".into()))?;
    ensure_proof_receipt_visible(allowed_area.is_some(), received.status)?;
    let channel:Option<String>=store.client().await.map_err(unavailable)?.query_opt(
        "SELECT voting_channel FROM ballot_box_ballot WHERE election_event_id=$1::text::uuid AND election_id=$2::text::uuid AND ballot_id=$3 AND voter_id=$4 AND area_id=$5::text::uuid AND status='valid'",
        &[&event.id,&body.election_id,&received.ballot_hash,&received.voter_id,&area]).await.map_err(unavailable)?
        .map(|row|row.try_get(0)).transpose().map_err(unavailable)?;
    let channel = proof_channel(received.status, channel)?;
    let board = get_election_event_board(event.bulletin_board_reference)
        .ok_or_else(|| unavailable("Event log unavailable"))?;
    let entries = store
        .query(
            &board,
            &LogQuery {
                filters: vec![
                    Filter::Text(
                        TextColumn::BallotId,
                        SqlCompOperators::Equal,
                        code.clone(),
                    ),
                    Filter::Text(
                        TextColumn::ElectionId,
                        SqlCompOperators::Equal,
                        body.election_id.clone(),
                    ),
                ],
                limit: 10,
                ..Default::default()
            },
        )
        .await
        .map_err(unavailable)?;
    let find = |kind: &str| {
        entries
            .iter()
            .find(|entry| entry.statement_kind == kind)
            .map(|entry| STANDARD.encode(&entry.message))
    };
    payload(BallotPrivacyProof {
        ballot_id: code,
        pseudonym: received.voter_id,
        voting_channel: channel,
        received_entry: find("BallotReceived"),
        cast_entry: find("CastVote"),
    })
}

#[post("/ballot-box-keys", format = "json", data = "<body>")]
pub async fn ballot_box_keys(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<BallotKeysInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    proof_token_scope(&claims, &body.election_event_id, &body.election_id)?;
    let event = event(services, &claims, &body.election_event_id).await?;
    let allowed_area =
        proof_scope(services, &claims, &event, &body.election_id).await?;
    let store = get_event_store(&event.id).await.map_err(unavailable)?;
    let key_id = if let Some(ballot_id) = body.ballot_id.as_deref() {
        let code =
            electoral_log::adapters::privacy::normalize_privacy_code(ballot_id)
                .ok_or_else(|| bad("Invalid Ballot ID"))?;
        let row = store.client().await.map_err(unavailable)?.query_opt(
            "SELECT key_id,status FROM ballot_box_received WHERE election_event_id=$1::text::uuid AND election_id=$2::text::uuid AND ballot_id=$3 AND ($4::text IS NULL OR area_id=$4::text::uuid)",
            &[&event.id,&body.election_id,&code,&allowed_area],
        ).await.map_err(unavailable)?.ok_or_else(||(Status::NotFound,"Ballot not found".into()))?;
        let status: String = row.try_get(1).map_err(unavailable)?;
        ensure_proof_receipt_visible(
            allowed_area.is_some(),
            status.parse().map_err(unavailable)?,
        )?;
        Some(row.try_get::<_, String>(0).map_err(unavailable)?)
    } else {
        None
    };
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let board =
        get_election_event_board(event.bulletin_board_reference.clone())
            .ok_or_else(|| unavailable("Event log unavailable"))?;
    let checkpoint = store
        .journal()
        .checkpoint(&board)
        .await
        .map_err(unavailable)?;
    let mut keys = windmill::services::public_ballot_keys::list(
        &tx,
        &event.tenant_id,
        &event.id,
    )
    .await
    .map_err(unavailable)?
    .into_iter()
    .filter(|key| {
        key.election_event_id == event.id
            && key.board_log_uid == checkpoint.log_uid
            && key.board_name == board
            && key_id
                .as_ref()
                .is_none_or(|id| id == &key.ballot_box_key_id)
    });
    let key = keys.next().ok_or_else(|| {
        (
            Status::NotFound,
            "Published verification keys unavailable; backfill required".into(),
        )
    })?;
    if keys.next().is_some() {
        return Err(bad("Give a Ballot ID to select its published key"));
    }
    payload(BallotPrivacyTrust {
        tenant_id: event.tenant_id,
        election_event_id: event.id,
        election_id: body.election_id.clone(),
        system_public_key: key.system_public_key,
        ballot_box_key: sequent_core::ballot::BallotBoxKey {
            key_id: key.ballot_box_key_id,
            public_key: key.ballot_box_public_key,
        },
    })
}

#[derive(Deserialize)]
pub struct ResolveInput {
    election_event_id: String,
    pseudonym: String,
    reason: String,
}
async fn audit_signing(
    services: &HarvestServices,
    claims: &JwtClaims,
    event: &ElectionEvent,
) -> Result<SigningData, ApiError> {
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let board =
        get_election_event_board(event.bulletin_board_reference.clone())
            .ok_or_else(|| unavailable("Event log unavailable"))?;
    let key =
        get_board_signing_key(&tx, &event.tenant_id, Some(&event.id), &board)
            .await
            .map_err(unavailable)?;
    Ok(SigningData::new(
        key.clone(),
        &claims.hasura_claims.user_id,
        key,
    ))
}
#[post("/resolve-voter-pseudonym", format = "json", data = "<body>")]
pub async fn resolve_voter_pseudonym(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<ResolveInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    authorize(
        &claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::VOTER_PSEUDONYM_RESOLVE],
    )?;
    let event = event(services, &claims, &body.election_event_id).await?;
    let code = electoral_log::adapters::privacy::normalize_privacy_code(
        &body.pseudonym,
    )
    .ok_or_else(|| bad("Invalid pseudonym"))?;
    let reason = body
        .reason
        .parse::<PseudonymResolutionReason>()
        .map_err(bad)?;
    let store = get_event_store(&event.id).await.map_err(unavailable)?;
    let voter = store
        .resolve_pseudonym(&event.id, &code)
        .await
        .map_err(unavailable)?
        .ok_or_else(|| {
            (
                Status::NotFound,
                "Pseudonym not found or no longer retained".into(),
            )
        })?;
    let signing = audit_signing(services, &claims, &event).await?;
    let message = Message::pseudonym_resolved_message(
        &event.id,
        PseudonymResolution {
            tenant_id: event.tenant_id.clone(),
            pseudonym: code,
            actor_id: claims.hasura_claims.user_id.clone(),
            reason: reason.as_str().into(),
        },
        &signing,
    )
    .map_err(unavailable)?;
    windmill::services::vote_privacy::queue_message(
        &event.id,
        &format!("pseudonym-resolution:{}", uuid::Uuid::new_v4()),
        &message,
    )
    .await
    .map_err(unavailable)?;
    payload(json!({"voter_id":voter}))
}

#[derive(Deserialize)]
pub struct ForensicInput {
    election_event_id: String,
    election_id: Option<String>,
    ip: Option<String>,
    country: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    cursor: Option<String>,
    operation: Option<ForensicReadOperation>,
}
async fn forensic_scope(
    services: &HarvestServices,
    claims: &JwtClaims,
    input: &ForensicInput,
    operation: ForensicReadOperation,
) -> Result<(ElectionEvent, PostgresStore, Option<Vec<String>>), ApiError> {
    authorize(
        claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::FORENSIC_READ],
    )?;
    let event = event(services, claims, &input.election_event_id).await?;
    if event
        .get_presentation()
        .map_err(unavailable)?
        .unwrap_or_default()
        .network_data_policy
        .unwrap_or_default()
        != NetworkDataPolicy::ForensicStore
    {
        return Err(forbidden());
    }
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let elections = cast_vote_read_elections(
        &tx,
        claims,
        &event.id,
        input.election_id.as_deref(),
    )
    .await?;
    drop(tx);
    drop(client);
    let signing = audit_signing(services, claims, &event).await?;
    let scope_id = uuid::Uuid::new_v4().to_string();
    let mut message = Message::forensic_read_message(
        &event.id,
        ForensicRead {
            tenant_id: event.tenant_id.clone(),
            actor_id: claims.hasura_claims.user_id.clone(),
            scope_id: scope_id.clone(),
            operation,
        },
        &signing,
    )
    .map_err(unavailable)?;
    message.username = claims.preferred_username.clone();
    windmill::services::vote_privacy::queue_message(
        &event.id,
        &format!("forensic-read:{scope_id}"),
        &message,
    )
    .await
    .map_err(unavailable)?;
    let store = get_event_store(&event.id).await.map_err(unavailable)?;
    Ok((event, store, elections))
}

#[post("/forensic-network-summary", format = "json", data = "<body>")]
pub async fn forensic_network_summary(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<ForensicInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    let limit = body.limit.unwrap_or(25);
    let offset = body.offset.unwrap_or(0);
    if !(1..=100).contains(&limit) || offset < 0 {
        return Err(bad("Invalid page"));
    }
    let operation = match body.operation.as_ref() {
        None | Some(ForensicReadOperation::List) => ForensicReadOperation::List,
        Some(ForensicReadOperation::Export) => ForensicReadOperation::Export,
        _ => return Err(bad("Invalid summary operation")),
    };
    let (event, store, elections) =
        forensic_scope(services, &claims, &body, operation).await?;
    let client = store.client().await.map_err(unavailable)?;
    let predicate="s.election_event_id=$1::text::uuid AND ($2::text[] IS NULL OR s.election_id::text=ANY($2)) AND ($3::text IS NULL OR s.ip LIKE $3 || '%') AND ($4::text IS NULL OR s.country=$4)";
    let query=format!("SELECT s.ip,min(s.country),sum(s.records)::bigint,min(s.first_at),max(s.last_at), \
        ARRAY(SELECT DISTINCT p.pseudonym FROM sequent_backend.forensic_network_pseudonym p WHERE p.election_event_id=$1::text::uuid AND p.ip=s.ip AND ($2::text[] IS NULL OR p.election_id::text=ANY($2)) ORDER BY p.pseudonym LIMIT 6), \
        (SELECT count(DISTINCT p.pseudonym) FROM sequent_backend.forensic_network_pseudonym p WHERE p.election_event_id=$1::text::uuid AND p.ip=s.ip AND ($2::text[] IS NULL OR p.election_id::text=ANY($2))) \
        FROM sequent_backend.forensic_network_summary s WHERE {predicate} GROUP BY s.ip ORDER BY sum(s.records) DESC,s.ip LIMIT $5 OFFSET $6");
    let rows = client
        .query(
            &query,
            &[
                &event.id,
                &elections,
                &body.ip,
                &body.country,
                &limit,
                &offset,
            ],
        )
        .await
        .map_err(unavailable)?;
    let election_presentation = if let Some(election) =
        body.election_id.as_deref()
    {
        let client = services
            .databases
            .hasura()
            .await
            .get()
            .await
            .map_err(unavailable)?;
        client.query_opt("SELECT presentation FROM sequent_backend.election WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND id=$3::text::uuid", &[&event.tenant_id,&event.id,&election])
            .await.map_err(unavailable)?.and_then(|row| row.get::<_,Option<Value>>(0))
    } else {
        None
    };
    let items=rows.iter().map(|row| -> Result<Value> {let ip:String=row.try_get(0)?;let pseudonyms:Vec<String>=row.try_get(5)?;
        Ok(json!({"id":ip,"ip":ip,"election_presentation":election_presentation,"country":row.try_get::<_,Option<String>>(1)?,"vote_count":row.try_get::<_,i64>(2)?,
            "first_ballot":row.try_get::<_,chrono::DateTime<chrono::Utc>>(3)?.to_rfc3339(),"last_ballot":row.try_get::<_,chrono::DateTime<chrono::Utc>>(4)?.to_rfc3339(),
            "shared":pseudonyms.len()>1,"pseudonyms":pseudonyms,"pseudonym_count":row.try_get::<_,i64>(6)?}))}).collect::<Result<Vec<_>>>().map_err(unavailable)?;
    let total:i64=client.query_one(&format!("SELECT count(DISTINCT s.ip) FROM sequent_backend.forensic_network_summary s WHERE {predicate}"),&[&event.id,&elections,&body.ip,&body.country]).await.map_err(unavailable)?.try_get(0).map_err(unavailable)?;
    payload(json!({"items":items,"total":total}))
}

#[post("/forensic-network-detail", format = "json", data = "<body>")]
pub async fn forensic_network_detail(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<ForensicInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    let ip = body
        .ip
        .as_deref()
        .filter(|ip| ip.parse::<std::net::IpAddr>().is_ok())
        .ok_or_else(|| bad("Give an IP address"))?;
    let limit = body.limit.unwrap_or(25);
    if !(1..=100).contains(&limit) {
        return Err(bad("Invalid page"));
    }
    let (event, store, elections) =
        forensic_scope(services, &claims, &body, ForensicReadOperation::Detail)
            .await?;
    let scope = json!([event.id, elections, ip]).to_string();
    let before = decode_cursor(&scope, body.cursor.as_deref())?;
    let client = store.client().await.map_err(unavailable)?;
    let rows=client.query("SELECT id,ballot_id,pseudonym,recorded_at FROM sequent_backend.forensic_network_record WHERE election_event_id=$1::text::uuid \
        AND ($2::text[] IS NULL OR election_id::text=ANY($2)) AND ip=$3 AND event_kind='cast' AND ($4::bigint IS NULL OR id<$4) ORDER BY id DESC LIMIT $5",
        &[&event.id,&elections,&ip,&before,&(limit+1)]).await.map_err(unavailable)?;
    let next = if rows.len() > limit as usize {
        Some(encode_cursor(
            &scope,
            rows[limit as usize - 1].try_get(0).map_err(unavailable)?,
        ))
    } else {
        None
    };
    let ballots=rows.iter().take(limit as usize).map(|r| -> Result<Value> {Ok(json!({"ballot_id":r.try_get::<_,Option<String>>(1)?,"pseudonym":r.try_get::<_,String>(2)?,"cast_at":r.try_get::<_,chrono::DateTime<chrono::Utc>>(3)?.to_rfc3339()}))}).collect::<Result<Vec<_>>>().map_err(unavailable)?;
    let total=client.query_one("SELECT min(country),COALESCE(sum(records),0)::bigint,min(first_at),max(last_at) FROM sequent_backend.forensic_network_summary \
        WHERE election_event_id=$1::text::uuid AND ($2::text[] IS NULL OR election_id::text=ANY($2)) AND ip=$3",&[&event.id,&elections,&ip]).await.map_err(unavailable)?;
    payload(
        json!({"ip":ip,"country":total.try_get::<_,Option<String>>(0).map_err(unavailable)?,"vote_count":total.try_get::<_,i64>(1).map_err(unavailable)?,
        "first_ballot":total.try_get::<_,Option<chrono::DateTime<chrono::Utc>>>(2).map_err(unavailable)?.map(|v|v.to_rfc3339()),
        "last_ballot":total.try_get::<_,Option<chrono::DateTime<chrono::Utc>>>(3).map_err(unavailable)?.map(|v|v.to_rfc3339()),"ballots":ballots,"next_cursor":next}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_is_bound_to_area_elections_and_filter() {
        let scope = scope_key("event", "area-a", &["e1".into()], "7Q");
        let cursor = encode_cursor(&scope, 42);
        assert_eq!(decode_cursor(&scope, Some(&cursor)).unwrap(), Some(42));
        for other in [
            scope_key("event", "area-b", &["e1".into()], "7Q"),
            scope_key("event", "area-a", &["e2".into()], "7Q"),
            scope_key("event", "area-a", &["e1".into()], "8"),
        ] {
            assert!(decode_cursor(&other, Some(&cursor)).is_err());
        }
        assert!(decode_cursor(&scope, Some("not a cursor")).is_err());
    }
    #[test]
    fn prefixes_are_codes_not_sql_patterns() {
        assert_eq!(prefix(Some("7qkm-2")).unwrap(), "7QKM2");
        for value in ["%", "_", "7QKI", "123456789"] {
            assert!(prefix(Some(value)).is_err());
        }
    }

    #[test]
    fn pagination_cursor_stays_bounded_for_104_authorized_elections() {
        let elections = (0..104)
            .map(|_| uuid::Uuid::new_v4().to_string())
            .collect::<Vec<_>>();
        let scope = scope_key("event", "area", &elections, "");
        let cursor = encode_cursor(&scope, 42);
        assert!(cursor.len() < 200);
        assert_eq!(decode_cursor(&scope, Some(&cursor)).unwrap(), Some(42));
    }
}

#[derive(Deserialize)]
pub struct PrivacySettingsInput {
    election_event_id: String,
}

#[post("/ballot-privacy-settings", format = "json", data = "<body>")]
pub async fn ballot_privacy_settings(
    services: &State<HarvestServices>,
    claims: JwtClaims,
    body: Json<PrivacySettingsInput>,
) -> Result<Json<PrivacyPayload>, ApiError> {
    authorize(
        &claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_READ],
    )?;
    let event = event(services, &claims, &body.election_event_id).await?;
    let realm =
        sequent_core::services::keycloak::get_tenant_realm(&event.tenant_id);
    let client = sequent_core::services::keycloak::KeycloakAdminClient::new()
        .await
        .map_err(unavailable)?;
    let (roles, _) = client
        .list_roles(&realm, None, None, None)
        .await
        .map_err(unavailable)?;
    let names = |permission: &str| {
        roles
            .iter()
            .filter(|role| {
                role.permissions
                    .as_ref()
                    .is_some_and(|p| p.iter().any(|p| p == permission))
            })
            .filter_map(|role| role.name.clone())
            .collect::<Vec<_>>()
    };
    let system_public_key = if event.bulletin_board_reference.is_some() {
        let mut client = services
            .databases
            .hasura()
            .await
            .get()
            .await
            .map_err(unavailable)?;
        let tx = client.transaction().await.map_err(unavailable)?;
        let board =
            get_election_event_board(event.bulletin_board_reference.clone())
                .ok_or_else(|| unavailable("Event log unavailable"))?;
        let keys = windmill::services::public_ballot_keys::list(
            &tx,
            &event.tenant_id,
            &event.id,
        )
        .await
        .map_err(unavailable)?;
        let mut public = None;
        for key in keys.into_iter().filter(|key| {
            key.election_event_id == event.id && key.board_name == board
        }) {
            if public
                .as_ref()
                .is_some_and(|previous| previous != &key.system_public_key)
            {
                return Err(unavailable(
                    "Conflicting published authority keys",
                ));
            }
            public = Some(key.system_public_key);
        }
        public
    } else {
        None
    };
    payload(json!({"system_public_key":system_public_key,"roles":{
        "forensic_read":names("forensic-read"),"pseudonym_resolve":names("voter-pseudonym-resolve")}}))
}

#[cfg(test)]
#[path = "../../tests/support/ballot_privacy_routes.rs"]
mod route_tests;
