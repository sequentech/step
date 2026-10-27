// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! FAU_SAA.3: durable activity, bounded comparisons, deduplicated detections.
pub mod policy;
use crate::services::database::get_hasura_pool;
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthFailure {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub realm: String,
    pub account: Option<String>,
    pub event_time_ms: i64,
    pub failure_limit: i32,
    pub window_seconds: i32,
}
pub async fn record_auth(input: &AuthFailure) -> Result<()> {
    let expected = match input.election_event_id {
        Some(event) => format!("tenant-{}-event-{event}", input.tenant_id),
        None => format!("tenant-{}", input.tenant_id),
    };
    ensure!(
        input.realm == expected,
        "Authentication activity realm does not match tenant/event"
    );
    ensure!(
        (1..=100000).contains(&input.failure_limit)
            && (1..=2592000).contains(&input.window_seconds),
        "Invalid authentication failure policy"
    );
    ensure!(
        input
            .account
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.len() <= 512),
        "Invalid account reference"
    );
    let account = input
        .account
        .as_ref()
        .map(|s| hex::encode(Sha256::digest(format!("{}:{s}", input.realm).as_bytes())));
    let time = DateTime::from_timestamp_millis(input.event_time_ms)
        .context("Invalid authentication activity time")?;
    let db = get_hasura_pool().await.get().await?;
    db.query_one(
        "SELECT sequent_backend.record_attack_auth_activity($1,$2,$3,$4,$5,$6,$7,$8)",
        &[
            &input.id,
            &input.tenant_id,
            &input.election_event_id,
            &input.realm,
            &account,
            &time,
            &input.failure_limit,
            &input.window_seconds,
        ],
    )
    .await?;
    Ok(())
}
pub async fn settings(tenant: &str) -> Result<Value> {
    let db = get_hasura_pool().await.get().await?;
    let row=db.query_one("SELECT COALESCE(s.failed_logins_per_minute,20),COALESCE(s.requests_per_minute,2000) FROM sequent_backend.tenant t LEFT JOIN sequent_backend.attack_detection_settings s ON s.tenant_id=t.id WHERE t.id=$1::text::uuid", &[&tenant]).await?;
    Ok(
        json!({"failed_logins_per_minute":row.try_get::<_,i64>(0)?,"requests_per_minute":row.try_get::<_,i64>(1)?}),
    )
}
pub fn rates(value: &Value) -> Result<(i64, i64)> {
    let object = value.as_object().context("Settings must be an object")?;
    ensure!(object.len() == 2, "Unexpected attack detection settings");
    let failures = value["failed_logins_per_minute"]
        .as_i64()
        .context("Failed login rate must be an integer")?;
    let requests = value["requests_per_minute"]
        .as_i64()
        .context("Request rate must be an integer")?;
    ensure!(
        policy::valid_rates(failures, requests),
        "Attack rates must be positive and within the documented limits"
    );
    Ok((failures, requests))
}
pub async fn update_settings(tenant: &str, actor: &str, value: &Value) -> Result<()> {
    let (failures, requests) = rates(value)?;
    let mut db = get_hasura_pool().await.get().await?;
    let tx = db.transaction().await?;
    tx.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1,848528))",
        &[&tenant],
    )
    .await?;
    // Opening holds the same event lock, so it cannot race a threshold change.
    // Fail fast on a live operation; waiting could create a reversed lock order.
    let events=tx.query("SELECT id::text FROM sequent_backend.election_event WHERE tenant_id=$1::text::uuid ORDER BY id", &[&tenant]).await?;
    for event in events {
        let event: String = event.try_get(0)?;
        let held: bool = tx
            .query_one(
                "SELECT pg_try_advisory_xact_lock(hashtextextended($1||':'||$2,848527))",
                &[&tenant, &event],
            )
            .await?
            .try_get(0)?;
        ensure!(
            held,
            "An election operation is in progress; retry changing thresholds"
        );
    }
    let active:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE tenant_id=$1::text::uuid AND phase='execution')", &[&tenant]).await?.get(0);
    ensure!(
        !active,
        "Attack thresholds cannot change during election execution; configure before approval"
    );
    tx.execute("INSERT INTO sequent_backend.attack_detection_settings(tenant_id,failed_logins_per_minute,requests_per_minute,updated_by) VALUES($1::text::uuid,$2,$3,$4) ON CONFLICT(tenant_id) DO UPDATE SET failed_logins_per_minute=$2,requests_per_minute=$3,updated_by=$4,updated_at=clock_timestamp()", &[&tenant,&failures,&requests,&actor]).await?;
    let details = json!({"actor":actor,"settings":value});
    tx.execute("INSERT INTO sequent_backend.lifecycle_notice(tenant_id,election_event_id,kind,details) SELECT tenant_id,id,'AttackDetectionSettingsChanged',$2 FROM sequent_backend.election_event WHERE tenant_id=$1::text::uuid", &[&tenant,&details]).await?;
    tx.commit().await?;
    Ok(())
}
#[derive(Debug, Deserialize)]
pub struct RequestBucket {
    pub minute_start_ms: i64,
    pub host: String,
    pub count: i64,
}
#[derive(Debug, Deserialize)]
pub struct Activity {
    #[serde(rename = "schema")]
    pub schema_version: u32,
    pub generation: String,
    pub updated_at_ms: i64,
    pub source: String,
    pub scope: String,
    pub status: String,
    pub buckets: Vec<RequestBucket>,
}
pub fn validate_activity(feed: &Activity, now: i64) -> Result<()> {
    ensure!(
        feed.schema_version == 1 && feed.source == "external-nginx" && feed.scope == "cluster",
        "Unexpected request activity source"
    );
    Uuid::parse_str(&feed.generation).context("Invalid request activity generation")?;
    ensure!(
        feed.status == "available" && policy::fresh_source(feed.updated_at_ms, now),
        "External request activity unavailable or stale"
    );
    ensure!(feed.buckets.len() <= 1024, "Request activity exceeds bound");
    let mut keys = std::collections::HashSet::new();
    for row in &feed.buckets {
        ensure!(
            policy::valid_bucket_time(row.minute_start_ms, feed.updated_at_ms, now),
            "Invalid request bucket time"
        );
        ensure!(
            (0..=1_000_000_000).contains(&row.count)
                && !row.host.is_empty()
                && row.host.len() <= 253
                && row
                    .host
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-_:".contains(&c)),
            "Invalid request bucket"
        );
        ensure!(
            keys.insert((row.minute_start_ms, row.host.as_str())),
            "Duplicate request bucket"
        );
    }
    Ok(())
}
async fn collect_requests() -> Result<()> {
    let path = std::env::var("WINDMILL_ATTACK_ACTIVITY_STATUS")
        .unwrap_or_else(|_| "/run/sequent-security/activity.json".into());
    let meta = tokio::fs::metadata(&path).await?;
    ensure!(meta.len() <= 256 * 1024, "Request activity exceeds bound");
    let bytes = tokio::fs::read(path).await?;
    ensure!(bytes.len() <= 256 * 1024, "Request activity exceeds bound");
    let feed: Activity = serde_json::from_slice(&bytes)?;
    let now = Utc::now();
    validate_activity(&feed, now.timestamp_millis())?;
    let mut db = get_hasura_pool().await.get().await?;
    let tx = db.transaction().await?;
    let changed:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.attack_request_bucket WHERE generation<>$1)", &[&feed.generation]).await?.get(0);
    ensure!(
        !changed,
        "Request activity generation changed; continuity requires investigation"
    );
    for bucket in feed.buckets {
        let minute = DateTime::from_timestamp_millis(bucket.minute_start_ms)
            .context("Invalid bucket time")?;
        let stored=tx.query_opt("SELECT requests FROM sequent_backend.attack_request_bucket WHERE generation=$1 AND minute_start=$2 AND host=$3", &[&feed.generation,&minute,&bucket.host]).await?;
        if let Some(row) = stored {
            ensure!(
                row.try_get::<_, i64>(0)? <= bucket.count,
                "Request activity counter moved backwards"
            );
        }
        tx.execute("INSERT INTO sequent_backend.attack_request_bucket(generation,minute_start,host,requests,observed_at) VALUES($1,$2,$3,$4,$5) ON CONFLICT(generation,minute_start,host) DO UPDATE SET requests=EXCLUDED.requests,observed_at=EXCLUDED.observed_at", &[&feed.generation,&minute,&bucket.host,&bucket.count,&now]).await?;
    }
    tx.execute(
        "SELECT sequent_backend.compare_attack_request_activity()",
        &[],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
/// The external feed has a controlled status; driver paths/errors are not exposed.
pub async fn reconcile() -> Result<()> {
    let collected = collect_requests().await;
    let available = collected.is_ok();
    if let Err(ref error) = collected {
        tracing::warn!("Attack request feed unavailable: {error:#}");
    }
    let mut db = get_hasura_pool().await.get().await?;
    let tx = db.transaction().await?;
    let reason = if available {
        "External request log is current"
    } else {
        "External request log unavailable, stale or discontinuous; investigate before relying on detection"
    };
    tx.execute("INSERT INTO sequent_backend.attack_activity_health(source,available,checked_at,reason) VALUES('external-nginx',$1,clock_timestamp(),$2) ON CONFLICT(source) DO UPDATE SET available=$1,checked_at=clock_timestamp(),reason=$2", &[&available,&reason]).await?;
    // Each detection is copied once into each applicable signed electoral-log outbox.
    let rows=tx.query("SELECT d.id,d.tenant_id,e.id,d.category,d.window_start,d.window_end,d.observed_count,d.threshold,d.details FROM sequent_backend.attack_detection d JOIN sequent_backend.election_event e ON e.tenant_id=d.tenant_id AND (d.election_event_id IS NULL OR d.election_event_id=e.id) WHERE NOT EXISTS(SELECT 1 FROM sequent_backend.attack_detection_audit a WHERE a.detection_id=d.id AND a.election_event_id=e.id) ORDER BY d.detected_at LIMIT 100", &[]).await?;
    for row in rows {
        let id: Uuid = row.get(0);
        let tenant: Uuid = row.get(1);
        let event: Uuid = row.get(2);
        let notice = Uuid::new_v4();
        let details = json!({"detection_id":id,"category":row.get::<_,String>(3),"window_start":row.get::<_,DateTime<Utc>>(4),"window_end":row.get::<_,DateTime<Utc>>(5),"observed_count":row.get::<_,i64>(6),"threshold":row.get::<_,i64>(7),"activity":row.get::<_,Value>(8)});
        let added=tx.execute("INSERT INTO sequent_backend.attack_detection_audit(detection_id,election_event_id,notice_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING", &[&id,&event,&notice]).await?;
        if added == 1 {
            tx.execute("INSERT INTO sequent_backend.lifecycle_notice(id,tenant_id,election_event_id,kind,details) VALUES($1,$2,$3,'PotentialAttackDetected',$4)", &[&notice,&tenant,&event,&details]).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
pub async fn detections(tenant: &str, event: &str) -> Result<Value> {
    let db = get_hasura_pool().await.get().await?;
    let rows=db.query("SELECT jsonb_build_object('id',id,'category',category,'detected_at',detected_at,'window_start',window_start,'window_end',window_end,'observed_count',observed_count,'threshold',threshold,'details',details) FROM sequent_backend.attack_detection WHERE tenant_id=$1::text::uuid AND (election_event_id IS NULL OR election_event_id=$2::text::uuid) ORDER BY detected_at DESC LIMIT 100", &[&tenant,&event]).await?;
    let health=db.query_opt("SELECT jsonb_build_object('available',available AND checked_at>clock_timestamp()-interval '30 seconds','checked_at',checked_at,'reason',reason) FROM sequent_backend.attack_activity_health WHERE source='external-nginx'", &[]).await?;
    Ok(
        json!({"detections":rows.iter().map(|r|r.get::<_,Value>(0)).collect::<Vec<_>>(),"request_source":health.map(|r|r.get::<_,Value>(0)).unwrap_or(json!({"available":false,"reason":"Request activity has not yet been observed"}))}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn activity() -> Activity {
        serde_json::from_value(json!({"schema":1,"generation":"11111111-1111-4111-8111-111111111111","updated_at_ms":120000,"source":"external-nginx","scope":"cluster","status":"available","buckets":[{"minute_start_ms":120000,"host":"vote.example","count":4}]})).unwrap()
    }
    #[test]
    fn request_feed_rejects_missing_freshness_continuity_and_malformed_counters() {
        let mut feed = activity();
        assert!(validate_activity(&feed, 120000).is_ok());
        assert!(validate_activity(&feed, 150001).is_err());
        assert!(validate_activity(&feed, 119999).is_err());
        feed.status = "unavailable".into();
        assert!(validate_activity(&feed, 120000).is_err());
        feed = activity();
        feed.generation = "restarted-without-state".into();
        assert!(validate_activity(&feed, 120000).is_err());
        feed = activity();
        feed.buckets[0].count = -1;
        assert!(validate_activity(&feed, 120000).is_err());
        feed = activity();
        feed.buckets[0].host = "vote.example/token?secret".into();
        assert!(validate_activity(&feed, 120000).is_err());
        feed = activity();
        feed.buckets.push(RequestBucket {
            minute_start_ms: 120000,
            host: "vote.example".into(),
            count: 4,
        });
        assert!(validate_activity(&feed, 120000).is_err());
    }
    #[test]
    fn threshold_settings_are_explicit_positive_bounded_integers() {
        assert_eq!(
            rates(&json!({"failed_logins_per_minute":20,"requests_per_minute":2000})).unwrap(),
            (20, 2000)
        );
        for bad in [
            json!({}),
            json!({"failed_logins_per_minute":0,"requests_per_minute":2000}),
            json!({"failed_logins_per_minute":20.5,"requests_per_minute":2000}),
            json!({"failed_logins_per_minute":20,"requests_per_minute":10000001}),
            json!({"failed_logins_per_minute":20,"requests_per_minute":2000,"enabled":false}),
        ] {
            assert!(rates(&bad).is_err());
        }
    }
}
