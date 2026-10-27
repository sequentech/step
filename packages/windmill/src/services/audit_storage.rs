// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Observe the audit database filesystem and refuse voters when evidence is unhealthy.

use super::audit_storage_policy::{parse_df, Space};
use super::database::try_get_queue_pool;
use anyhow::{bail, ensure, Context, Result};
use serde_json::json;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

/// Deployment policy. Required in the evaluated installation; explicit enum preserves
/// deployments that have not provisioned the database filesystem observer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoragePolicy {
    Disabled,
    Required,
}

impl StoragePolicy {
    pub fn from_env() -> Result<Self> {
        match std::env::var("AUDIT_STORAGE_POLICY").as_deref() {
            Ok("required") => Ok(Self::Required),
            Ok("disabled") | Err(std::env::VarError::NotPresent) => Ok(Self::Disabled),
            _ => bail!("AUDIT_STORAGE_POLICY must be required or disabled"),
        }
    }
}

/// Read exactly the mounted audit database filesystem, never the container overlay.
pub async fn measure(path: &Path) -> Result<Space> {
    ensure!(path.is_absolute(), "Audit storage path must be absolute");
    let output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("/usr/bin/df")
            .args(["--output=size,used,avail", "-B1", "--"])
            .arg(path)
            .env("LC_ALL", "C")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("Audit storage observation timed out")??;
    ensure!(output.status.success(), "Audit storage observation failed");
    parse_df(std::str::from_utf8(&output.stdout)?).map_err(anyhow::Error::msg)
}

/// The sidecar's only duty: persist a short-lived measurement in the environment's
/// queue database. It mounts the very same PVC as PostgreSQL, read-only.
pub async fn monitor() -> Result<()> {
    ensure!(
        StoragePolicy::from_env()? == StoragePolicy::Required,
        "Audit observer requires the required policy"
    );
    let path = std::env::var("AUDIT_STORAGE_PATH").context("AUDIT_STORAGE_PATH is required")?;
    ensure!(
        Path::new(&path).is_absolute(),
        "Audit storage path must be absolute"
    );
    loop {
        let observation = measure(Path::new(&path)).await;
        let (state, used, available) = match observation {
            Ok(space) => (
                if space.exceeded() {
                    "threshold-exceeded"
                } else {
                    "available"
                },
                Some(i64::try_from(space.used)?),
                Some(i64::try_from(space.available)?),
            ),
            Err(error) => {
                tracing::warn!("Audit storage observation unavailable: {error:#}");
                ("unavailable", None, None)
            }
        };
        let recorded = tokio::time::timeout(Duration::from_secs(5), async {
            let db = try_get_queue_pool().await?.get().await?;
            db.execute(
                "INSERT INTO step_queue.audit_storage_health(singleton,observed_at,state,used_bytes,available_bytes,filesystem_path) VALUES(true,clock_timestamp(),$1,$2,$3,$4)
                 ON CONFLICT(singleton) DO UPDATE SET observed_at=clock_timestamp(),state=EXCLUDED.state,used_bytes=EXCLUDED.used_bytes,available_bytes=EXCLUDED.available_bytes,filesystem_path=EXCLUDED.filesystem_path,
                 generation=CASE WHEN audit_storage_health.state IS DISTINCT FROM EXCLUDED.state OR audit_storage_health.observed_at<clock_timestamp()-interval '90 seconds' THEN audit_storage_health.generation+1 ELSE audit_storage_health.generation END",
                &[&state, &used, &available, &path],
            ).await?;
            Ok::<_, anyhow::Error>(())
        }).await;
        if !matches!(recorded, Ok(Ok(()))) {
            // Readers independently fail closed when the last successful observation ages.
            tracing::error!("Could not persist the audit-storage observation");
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

/// Copy measured health to the GraphQL boundary and durably record each state
/// transition once per event. Email and audit delivery happen outside this transaction.
pub async fn reconcile_alerts() -> Result<()> {
    let (state, queue_generation) = if StoragePolicy::from_env()? == StoragePolicy::Disabled {
        ("disabled".to_string(), None)
    } else {
        let observation = tokio::time::timeout(Duration::from_secs(3), async {
            let db = try_get_queue_pool().await?.get().await?;
            let row = db.query_opt("SELECT CASE WHEN observed_at>clock_timestamp() OR observed_at<=clock_timestamp()-interval '90 seconds' THEN 'stale' ELSE state END, generation FROM step_queue.audit_storage_health WHERE singleton", &[]).await?;
            match row {
                Some(row) => Ok::<_,anyhow::Error>((row.try_get::<_,String>(0)?,Some(row.try_get::<_,i64>(1)?))),
                None => Ok(("unavailable".into(),None)),
            }
        }).await;
        observation
            .ok()
            .and_then(Result::ok)
            .unwrap_or(("unavailable".into(), None))
    };
    let mut db = super::database::get_hasura_pool().await.get().await?;
    let tx = db.transaction().await?;
    tx.batch_execute("SET LOCAL statement_timeout='5000ms'")
        .await?;
    let row=tx.query_one("INSERT INTO sequent_backend.audit_storage_status(singleton,state,queue_generation) VALUES(true,$1,$2) ON CONFLICT(singleton) DO UPDATE SET state=EXCLUDED.state,queue_generation=EXCLUDED.queue_generation,updated_at=clock_timestamp(),generation=CASE WHEN audit_storage_status.state IS DISTINCT FROM EXCLUDED.state OR audit_storage_status.queue_generation IS DISTINCT FROM EXCLUDED.queue_generation THEN audit_storage_status.generation+1 ELSE audit_storage_status.generation END RETURNING generation", &[&state,&queue_generation]).await?;
    let generation: i64 = row.try_get(0)?;
    if !matches!(state.as_str(), "available" | "disabled") {
        let events=tx.query("SELECT tenant_id::text,election_event_id::text FROM sequent_backend.election_lifecycle l WHERE phase IN ('preparation','execution','evaluation') AND NOT EXISTS(SELECT 1 FROM sequent_backend.audit_storage_notice n WHERE n.tenant_id=l.tenant_id AND n.election_event_id=l.election_event_id AND n.generation=$1) ORDER BY tenant_id,election_event_id LIMIT 100",&[&generation]).await?;
        for event in events {
            let tenant: String = event.try_get(0)?;
            let event: String = event.try_get(1)?;
            let inserted=tx.execute("INSERT INTO sequent_backend.audit_storage_notice(tenant_id,election_event_id,generation) VALUES($1::text::uuid,$2::text::uuid,$3) ON CONFLICT DO NOTHING",&[&tenant,&event,&generation]).await?;
            if inserted == 1 {
                super::election_lifecycle::notice(&tx,&tenant,&event,
                    if state=="threshold-exceeded" {"AuditStorageThresholdExceeded"} else {"AuditStorageObservationUnavailable"},
                    json!({"state":state,"generation":generation,"limit_percent":80,"voter_channel":"denied"})).await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

/// A fresh positive observation is mandatory whenever storage protection is required.
/// Database errors and expired/future-dated measurements are refusals, not exemptions.
pub async fn require_voter_storage() -> Result<()> {
    if StoragePolicy::from_env()? == StoragePolicy::Disabled {
        return Ok(());
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        let db = try_get_queue_pool().await?.get().await?;
        let healthy: bool = db.query_one(
            "SELECT EXISTS(SELECT 1 FROM step_queue.audit_storage_health WHERE singleton AND state='available' AND observed_at<=clock_timestamp() AND observed_at>clock_timestamp()-interval '90 seconds')", &[],
        ).await?.try_get(0)?;
        ensure!(healthy, "AuditStorageThresholdExceeded");
        Ok::<_, anyhow::Error>(())
    }).await.context("Audit storage status timed out")?
}
