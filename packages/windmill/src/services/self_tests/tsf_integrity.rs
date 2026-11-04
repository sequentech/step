// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! On-demand comparison of the running TOE images with the accepted delivery.
//! Separate from the self-test suite: its result never counts for voter access.
use super::checks;
use crate::services::{
    database::get_hasura_pool, election_lifecycle, electoral_log_audit_deadline::AuditDeadline,
};
use anyhow::{ensure, Context, Result};
use chrono::Utc;
use deadpool_postgres::Transaction;
use sequent_core::election_lifecycle::SafetyState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Read, path::Path, time::Duration};

const IMAGE_STATUS: &str = "/run/sequent-images/status.json";
const MAX_STATUS_BYTES: u64 = 1024 * 1024;
const FRESHNESS_SECONDS: i64 = 180;
const AUDIT_SECONDS: u64 = 60;
const PARTS: [&str; 6] = [
    "b4",
    "ballot-verifier",
    "braid",
    "harvest",
    "keycloak",
    "windmill",
];
const DIFFERENT_REASON: &str = "TOE software differs from the accepted delivery";
const AUDIT_REASON: &str = "The electoral-log record of this check could not be verified";

/// Facts published by the control-plane host. A difference in this contract
/// reads as "could not verify", never as a match.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    sampled_at: i64,
    error: Option<String>,
    digest_list_sha256: Option<String>,
    toe_reference: Option<String>,
    tag: Option<String>,
    commit: Option<String>,
    parts: BTreeMap<String, PartFacts>,
    containers: Vec<ContainerFacts>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PartFacts {
    accepted_digest: Option<String>,
    expected_config: Option<String>,
    error: Option<String>,
    content_mismatch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerFacts {
    namespace: String,
    pod: String,
    container: String,
    part: String,
    image: String,
    image_id: Option<String>,
    observed_config: Option<String>,
    running: bool,
    ready: bool,
    phase: Option<String>,
    error: Option<String>,
    content_mismatch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Verified,
    Different,
    Unverifiable,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Verified => "verified",
            Outcome::Different => "different",
            Outcome::Unverifiable => "unverifiable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartResult {
    part: String,
    accepted_digest: Option<String>,
    expected_config: Option<String>,
    running: usize,
    required: usize,
    outcome: Outcome,
    detail: Option<String>,
    containers: Vec<ContainerFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Comparison {
    outcome: Outcome,
    reason: Option<String>,
    different_parts: Vec<String>,
    toe_reference: Option<String>,
    tag: Option<String>,
    commit: Option<String>,
    digest_list_sha256: Option<String>,
    sampled_at: Option<i64>,
    parts: Vec<PartResult>,
}

impl Comparison {
    fn unverifiable(reason: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::Unverifiable,
            reason: Some(reason.into()),
            different_parts: vec![],
            toe_reference: None,
            tag: None,
            commit: None,
            digest_list_sha256: None,
            sampled_at: None,
            parts: vec![],
        }
    }

    /// A difference stands without its log record; a match does not.
    fn without_audit_record(mut self) -> Self {
        if self.outcome == Outcome::Verified {
            self.outcome = Outcome::Unverifiable;
        }
        self.reason.get_or_insert_with(|| AUDIT_REASON.into());
        self
    }
}

fn judge(part: &str, facts: &PartFacts, containers: &[ContainerFacts]) -> PartResult {
    let own: Vec<ContainerFacts> = containers
        .iter()
        .filter(|c| c.part == part)
        .cloned()
        .collect();
    let image = facts
        .accepted_digest
        .as_ref()
        .map(|digest| format!("sequentech.local/{part}@{digest}"));
    let config = facts.expected_config.as_ref();
    let required = if matches!(part, "braid" | "windmill") {
        2
    } else {
        1
    };
    let ready = own
        .iter()
        .filter(|c| {
            c.running
                && c.ready
                && image.as_ref() == Some(&c.image)
                && config.is_some()
                && c.observed_config.as_ref() == config
        })
        .count();
    let (outcome, detail) = if facts.content_mismatch || own.iter().any(|c| c.content_mismatch) {
        (
            Outcome::Different,
            Some("Stored image content differs from its digest".to_string()),
        )
    } else if image.is_some() && own.iter().any(|c| image.as_ref() != Some(&c.image)) {
        (
            Outcome::Different,
            Some("A container uses an image other than the accepted one".to_string()),
        )
    } else if config.is_some()
        && own.iter().any(|c| {
            c.running && c.observed_config.is_some() && c.observed_config.as_ref() != config
        })
    {
        (
            Outcome::Different,
            Some("A running container has a configuration other than the accepted one".to_string()),
        )
    } else if image.is_none() || config.is_none() || facts.error.is_some() {
        (
            Outcome::Unverifiable,
            Some(
                facts
                    .error
                    .clone()
                    .unwrap_or_else(|| "The accepted image could not be resolved".to_string()),
            ),
        )
    } else if own
        .iter()
        .any(|c| c.running && (c.error.is_some() || c.observed_config.is_none()))
    {
        (
            Outcome::Unverifiable,
            Some("The image of a running container could not be read".to_string()),
        )
    } else if ready < required {
        // An outage or a part not yet commissioned is not different software.
        (
            Outcome::Unverifiable,
            Some(format!(
                "{ready} of the {required} required containers are ready"
            )),
        )
    } else {
        (Outcome::Verified, None)
    };
    PartResult {
        part: part.to_string(),
        accepted_digest: facts.accepted_digest.clone(),
        expected_config: facts.expected_config.clone(),
        running: own.iter().filter(|c| c.running).count(),
        required,
        outcome,
        detail,
        containers: own,
    }
}

/// Pure decision over one observation. Facts that are stale or incomplete say
/// nothing about the running software, whatever their content.
pub fn compare(observation: &Observation, now: i64) -> Comparison {
    let mut comparison = Comparison {
        toe_reference: observation.toe_reference.clone(),
        tag: observation.tag.clone(),
        commit: observation.commit.clone(),
        digest_list_sha256: observation.digest_list_sha256.clone(),
        sampled_at: Some(observation.sampled_at),
        ..Comparison::unverifiable("")
    };
    let refusal = if let Some(error) = &observation.error {
        Some(error.as_str())
    } else if observation.sampled_at > now {
        Some("The host image observation is dated in the future")
    } else if now - observation.sampled_at > FRESHNESS_SECONDS {
        Some("The host image observation is stale")
    } else if !observation.parts.keys().map(String::as_str).eq(PARTS)
        || observation
            .containers
            .iter()
            .any(|c| !PARTS.contains(&c.part.as_str()))
    {
        Some("The host image observation does not cover exactly the six TOE parts")
    } else {
        None
    };
    if let Some(reason) = refusal {
        comparison.reason = Some(reason.to_string());
        return comparison;
    }
    comparison.parts = PARTS
        .iter()
        .map(|part| judge(part, &observation.parts[*part], &observation.containers))
        .collect();
    comparison.different_parts = comparison
        .parts
        .iter()
        .filter(|p| p.outcome == Outcome::Different)
        .map(|p| p.part.clone())
        .collect();
    if !comparison.different_parts.is_empty() {
        comparison.outcome = Outcome::Different;
        comparison.reason = None;
    } else if comparison
        .parts
        .iter()
        .any(|p| p.outcome == Outcome::Unverifiable)
    {
        comparison.reason = Some("Not every TOE part could be verified".to_string());
    } else {
        comparison.outcome = Outcome::Verified;
        comparison.reason = None;
    }
    comparison
}

fn read_observation(path: &Path) -> Result<Observation> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .context("The host image observation is unavailable")?
        .take(MAX_STATUS_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_STATUS_BYTES,
        "Invalid host image observation size"
    );
    Ok(serde_json::from_slice(&bytes)?)
}

/// Queuing never waits for the comparison; each event has one check at a time.
pub async fn enqueue(tenant: &str, event: &str, requested_by: &str) -> Result<String> {
    AuditDeadline::database()
        .run(enqueue_inner(tenant, event, requested_by))
        .await?
}

async fn enqueue_inner(tenant: &str, event: &str, requested_by: &str) -> Result<String> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    tx.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        &[&format!("tsf-integrity:enqueue:{tenant}:{event}")],
    )
    .await?;
    if let Some(row)=tx.query_opt("SELECT id::text FROM sequent_backend.tsf_integrity_check WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND finished_at IS NULL",&[&tenant,&event]).await? {
        let id:String=row.try_get(0)?;tx.commit().await?;return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    tx.execute("INSERT INTO sequent_backend.tsf_integrity_check(id,tenant_id,election_event_id,requested_by) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4)",&[&id,&tenant,&event,&requested_by]).await?;
    tx.commit().await?;
    drop(c);
    let sent = crate::services::celery_app::get_celery_app()
        .await
        .send_task(crate::tasks::self_tests::run_tsf_integrity_check::new(
            tenant.into(),
            event.into(),
            id.clone(),
        ))
        .await;
    if sent.is_err() {
        finish(
            tenant,
            event,
            &id,
            &Comparison::unverifiable("The check could not be queued"),
        )
        .await?;
        anyhow::bail!("Unable to queue the integrity check; no match was recorded");
    }
    Ok(id)
}

/// A late worker never replaces the sweep's closing result.
async fn finish(tenant: &str, event: &str, id: &str, comparison: &Comparison) -> Result<bool> {
    let outcome = comparison.outcome.as_str();
    let result = serde_json::to_value(comparison)?;
    let safety_pending = comparison.outcome == Outcome::Different;
    let c = get_hasura_pool().await.get().await?;
    let closed=c.execute("UPDATE sequent_backend.tsf_integrity_check SET finished_at=now(),outcome=$4,result=$5,safety_pending=$6 WHERE id=$1::text::uuid AND tenant_id=$2::text::uuid AND election_event_id=$3::text::uuid AND finished_at IS NULL",&[&id,&tenant,&event,&outcome,&result,&safety_pending]).await?;
    Ok(closed == 1)
}

async fn enter_maintenance(tenant: &str, event: &str, id: &str) -> Result<()> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    let Some(row)=tx.query_opt("SELECT result FROM sequent_backend.tsf_integrity_check WHERE id=$1::text::uuid AND safety_pending FOR UPDATE SKIP LOCKED",&[&id]).await? else{return Ok(())};
    let result: Value = row.try_get(0)?;
    election_lifecycle::enter_failure(
        &tx,
        tenant,
        event,
        SafetyState::Recoverable,
        DIFFERENT_REASON,
        json!({"check_id":id,"different_parts":result["different_parts"],"digest_list_sha256":result["digest_list_sha256"]}),
    )
    .await?;
    tx.execute(
        "UPDATE sequent_backend.tsf_integrity_check SET safety_pending=false WHERE id=$1::text::uuid",
        &[&id],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn execute(tenant: &str, event: &str, id: &str) -> Result<()> {
    let setup = AuditDeadline::database();
    let mut c = setup
        .run(async { get_hasura_pool().await.get().await })
        .await??;
    let lock = setup.run(c.transaction()).await??;
    let acquired: bool = setup
        .run(lock.query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1,0))",
            &[&format!("tsf-integrity:execute:{event}")],
        ))
        .await??
        .try_get(0)?;
    if !acquired {
        return Ok(());
    }
    let Some(row)=setup.run(lock.query_opt("SELECT requested_by FROM sequent_backend.tsf_integrity_check WHERE id=$1::text::uuid AND tenant_id=$2::text::uuid AND election_event_id=$3::text::uuid AND finished_at IS NULL",&[&id,&tenant,&event])).await?? else{return Ok(())};
    let requested_by: String = row.try_get(0)?;
    let mut comparison = match read_observation(Path::new(IMAGE_STATUS)) {
        Ok(observation) => compare(&observation, Utc::now().timestamp()),
        Err(error) => {
            tracing::warn!(%event,check_id=id,"Host image observation refused: {error:#}");
            Comparison::unverifiable("The host image observation is unavailable or invalid")
        }
    };
    let audited = AuditDeadline::after(Duration::from_secs(AUDIT_SECONDS))
        .run(checks::audit_record(
            tenant,
            event,
            id,
            "tsf-integrity",
            &json!({"check_id":id,"requested_by":requested_by,"result":comparison}),
        ))
        .await;
    if !matches!(audited, Ok(Ok(()))) {
        tracing::warn!(%event, check_id = id, "Integrity check audit record not verified");
        comparison = comparison.without_audit_record();
    }
    // First commit the evidence. A busy safety writer cannot discard it; the sweep
    // retries the maintenance entry.
    let closed = AuditDeadline::database()
        .run(finish(tenant, event, id, &comparison))
        .await??;
    if closed && comparison.outcome == Outcome::Different {
        let entered = AuditDeadline::database()
            .run(enter_maintenance(tenant, event, id))
            .await;
        if !matches!(entered, Ok(Ok(()))) {
            tracing::warn!(%event, check_id = id, "Maintenance entry deferred to the sweep");
        }
    }
    lock.commit().await?;
    Ok(())
}

/// Close checks whose worker is gone and retry deferred maintenance entries.
pub async fn sweep() -> Result<()> {
    let abandoned = serde_json::to_value(Comparison::unverifiable("The check did not complete"))?;
    let c = get_hasura_pool().await.get().await?;
    c.execute("UPDATE sequent_backend.tsf_integrity_check SET finished_at=now(),outcome='unverifiable',result=$1 WHERE finished_at IS NULL AND started_at<now()-interval '300 seconds'",&[&abandoned]).await?;
    let pending=c.query("SELECT tenant_id::text,election_event_id::text,id::text FROM sequent_backend.tsf_integrity_check WHERE safety_pending ORDER BY started_at LIMIT 100",&[]).await?;
    drop(c);
    for row in pending {
        let event: String = row.try_get(1)?;
        let id: String = row.try_get(2)?;
        if let Err(error) = enter_maintenance(&row.try_get::<_, String>(0)?, &event, &id).await {
            tracing::warn!(%event,check_id=id,"Maintenance entry deferred: {error:#}");
        }
    }
    Ok(())
}

pub async fn latest(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<Option<Value>> {
    let row=tx.query_opt("SELECT jsonb_build_object('id',id,'requested_by',requested_by,'started_at',started_at,'finished_at',finished_at,'outcome',outcome,'result',result) FROM sequent_backend.tsf_integrity_check WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY started_at DESC LIMIT 1",&[&tenant,&event]).await?;
    Ok(row.map(|r| r.try_get(0)).transpose()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const NOW: i64 = 1_000_000;

    fn digest(part: &str) -> String {
        format!("sha256:{:0>64x}", part.len())
    }

    fn config(part: &str) -> String {
        format!("sha256:{:f>64x}", part.len())
    }

    fn container(part: &str, name: &str) -> ContainerFacts {
        ContainerFacts {
            namespace: "step-apps".into(),
            pod: format!("{name}-1"),
            container: name.into(),
            part: part.into(),
            image: format!("sequentech.local/{part}@{}", digest(part)),
            image_id: Some(format!("sequentech.local/{part}@{}", digest(part))),
            observed_config: Some(config(part)),
            running: true,
            ready: true,
            phase: Some("Running".into()),
            error: None,
            content_mismatch: false,
        }
    }

    fn matching() -> Observation {
        Observation {
            sampled_at: NOW - 30,
            error: None,
            digest_list_sha256: Some("a".repeat(64)),
            toe_reference: Some("Sequent Voting Platform".into()),
            tag: Some("v10.0.0".into()),
            commit: Some("abc".into()),
            parts: PARTS
                .iter()
                .map(|part| {
                    (
                        part.to_string(),
                        PartFacts {
                            accepted_digest: Some(digest(part)),
                            expected_config: Some(config(part)),
                            error: None,
                            content_mismatch: false,
                        },
                    )
                })
                .collect(),
            containers: vec![
                container("b4", "b4"),
                container("ballot-verifier", "ballot-verifier"),
                container("braid", "trustee1"),
                container("braid", "trustee2"),
                container("harvest", "harvest"),
                container("keycloak", "keycloak"),
                container("windmill", "windmill"),
                container("windmill", "beat"),
            ],
        }
    }

    fn with(part: &str, change: impl Fn(&mut ContainerFacts)) -> Observation {
        let mut observation = matching();
        observation
            .containers
            .iter_mut()
            .filter(|c| c.part == part)
            .for_each(change);
        observation
    }

    fn assert_unverifiable(observation: &Observation, reason: &str) {
        let comparison = compare(observation, NOW);
        assert_eq!(comparison.outcome, Outcome::Unverifiable);
        assert_eq!(comparison.reason.as_deref(), Some(reason));
        assert!(comparison.different_parts.is_empty());
    }

    fn assert_part_unverifiable(observation: &Observation, part: &str, detail: &str) {
        let comparison = compare(observation, NOW);
        assert_eq!(comparison.outcome, Outcome::Unverifiable);
        assert_eq!(
            comparison.reason.as_deref(),
            Some("Not every TOE part could be verified")
        );
        let judged = comparison.parts.iter().find(|p| p.part == part).unwrap();
        assert_eq!(judged.outcome, Outcome::Unverifiable);
        assert_eq!(judged.detail.as_deref(), Some(detail));
    }

    fn assert_different(observation: &Observation, part: &str, detail: &str) {
        let comparison = compare(observation, NOW);
        assert_eq!(comparison.outcome, Outcome::Different);
        assert_eq!(comparison.different_parts, [part]);
        assert_eq!(comparison.reason, None);
        let judged = comparison.parts.iter().find(|p| p.part == part).unwrap();
        assert_eq!(judged.detail.as_deref(), Some(detail));
    }

    #[test]
    fn matching_observation_is_verified_with_its_evidence() {
        let comparison = compare(&matching(), NOW);
        assert_eq!(comparison.outcome, Outcome::Verified);
        assert_eq!(comparison.reason, None);
        assert_eq!(
            comparison
                .parts
                .iter()
                .map(|p| (p.part.as_str(), p.running, p.required, p.outcome))
                .collect::<Vec<_>>(),
            vec![
                ("b4", 1, 1, Outcome::Verified),
                ("ballot-verifier", 1, 1, Outcome::Verified),
                ("braid", 2, 2, Outcome::Verified),
                ("harvest", 1, 1, Outcome::Verified),
                ("keycloak", 1, 1, Outcome::Verified),
                ("windmill", 2, 2, Outcome::Verified),
            ]
        );
        let stored = serde_json::to_value(&comparison).unwrap();
        assert_eq!(stored["outcome"], "verified");
        assert_eq!(stored["toe_reference"], "Sequent Voting Platform");
        assert_eq!(stored["digest_list_sha256"], "a".repeat(64));
        assert_eq!(stored["parts"][0]["outcome"], "verified");
        assert_eq!(stored["parts"][0]["containers"][0]["pod"], "b4-1");
    }

    #[test]
    fn observation_age_is_bounded_on_both_sides() {
        let mut observation = matching();
        observation.sampled_at = NOW - FRESHNESS_SECONDS;
        assert_eq!(compare(&observation, NOW).outcome, Outcome::Verified);
        observation.sampled_at = NOW;
        assert_eq!(compare(&observation, NOW).outcome, Outcome::Verified);
        observation.sampled_at = NOW - FRESHNESS_SECONDS - 1;
        assert_unverifiable(&observation, "The host image observation is stale");
        observation.sampled_at = NOW + 1;
        assert_unverifiable(
            &observation,
            "The host image observation is dated in the future",
        );
    }

    #[test]
    fn stale_facts_cannot_report_a_difference() {
        let mut observation = with("harvest", |c| c.image = "sequentech.local/other:1".into());
        observation.sampled_at = NOW - FRESHNESS_SECONDS - 1;
        assert_unverifiable(&observation, "The host image observation is stale");
    }

    #[test]
    fn observer_error_is_the_reason_and_keeps_the_delivery_identity() {
        let mut observation = matching();
        observation.error = Some("The running containers cannot be read".into());
        assert_unverifiable(&observation, "The running containers cannot be read");
        let comparison = compare(&observation, NOW);
        assert_eq!(
            comparison.toe_reference.as_deref(),
            Some("Sequent Voting Platform")
        );
        assert_eq!(comparison.digest_list_sha256, Some("a".repeat(64)));
        assert!(comparison.parts.is_empty());
    }

    #[test]
    fn exactly_the_six_parts_must_be_present() {
        let coverage = "The host image observation does not cover exactly the six TOE parts";
        let mut missing = matching();
        missing.parts.remove("braid");
        assert_unverifiable(&missing, coverage);
        let mut extra = matching();
        let facts = extra.parts["b4"].clone();
        extra.parts.insert("election-verifier".into(), facts);
        assert_unverifiable(&extra, coverage);
        let mut foreign = matching();
        foreign.containers.push(container("other", "other"));
        assert_unverifiable(&foreign, coverage);
    }

    #[test]
    fn running_image_must_be_pinned_to_the_accepted_digest() {
        let detail = "A container uses an image other than the accepted one";
        for image in [
            "sequentech.local/harvest:latest".to_string(),
            format!("sequentech.local/harvest@{}", digest("other-image")),
            format!("sequentech.local/keycloak@{}", digest("harvest")),
            "example.org/harvest@sha256:0".to_string(),
        ] {
            assert_different(
                &with("harvest", |c| c.image = image.clone()),
                "harvest",
                detail,
            );
        }
    }

    #[test]
    fn observed_configuration_must_equal_the_expected_one() {
        assert_different(
            &with("keycloak", |c| {
                c.observed_config = Some(config("other-image"))
            }),
            "keycloak",
            "A running container has a configuration other than the accepted one",
        );
    }

    #[test]
    fn altered_content_is_a_difference() {
        let detail = "Stored image content differs from its digest";
        let mut accepted = matching();
        let facts = accepted.parts.get_mut("b4").unwrap();
        facts.expected_config = None;
        facts.content_mismatch = true;
        assert_different(&accepted, "b4", detail);
        assert_different(
            &with("b4", |c| {
                c.observed_config = None;
                c.content_mismatch = true;
            }),
            "b4",
            detail,
        );
    }

    #[test]
    fn missing_ready_containers_are_not_different_software() {
        for (part, detail) in [
            ("b4", "0 of the 1 required containers are ready"),
            (
                "ballot-verifier",
                "0 of the 1 required containers are ready",
            ),
            ("harvest", "0 of the 1 required containers are ready"),
            ("keycloak", "0 of the 1 required containers are ready"),
            ("braid", "0 of the 2 required containers are ready"),
            ("windmill", "0 of the 2 required containers are ready"),
        ] {
            let mut absent = matching();
            absent.containers.retain(|c| c.part != part);
            assert_part_unverifiable(&absent, part, detail);
        }
        for part in ["braid", "windmill"] {
            let mut short = matching();
            let position = short
                .containers
                .iter()
                .position(|c| c.part == part)
                .unwrap();
            short.containers.remove(position);
            assert_part_unverifiable(&short, part, "1 of the 2 required containers are ready");
        }
    }

    #[test]
    fn not_ready_containers_do_not_count_and_are_not_a_difference() {
        assert_part_unverifiable(
            &with("harvest", |c| c.ready = false),
            "harvest",
            "0 of the 1 required containers are ready",
        );
        let comparison = compare(&with("harvest", |c| c.ready = false), NOW);
        let harvest = comparison
            .parts
            .iter()
            .find(|p| p.part == "harvest")
            .unwrap();
        assert_eq!(harvest.running, 1);
    }

    #[test]
    fn finished_containers_only_need_the_accepted_image() {
        let mut job = container("windmill", "setup-queue-database");
        job.running = false;
        job.ready = false;
        job.phase = Some("Succeeded".into());
        job.image_id = None;
        job.observed_config = None;
        job.error = Some("The content store cannot provide it".into());
        let mut observation = matching();
        observation.containers.push(job.clone());
        let comparison = compare(&observation, NOW);
        assert_eq!(comparison.outcome, Outcome::Verified);
        let windmill = comparison
            .parts
            .iter()
            .find(|p| p.part == "windmill")
            .unwrap();
        assert_eq!((windmill.running, windmill.containers.len()), (2, 3));
        // It cannot make up the minimum.
        observation.containers.retain(|c| c.container != "beat");
        assert_part_unverifiable(
            &observation,
            "windmill",
            "1 of the 2 required containers are ready",
        );
        job.image = "sequentech.local/windmill:latest".into();
        let mut foreign = matching();
        foreign.containers.push(job);
        assert_different(
            &foreign,
            "windmill",
            "A container uses an image other than the accepted one",
        );
    }

    #[test]
    fn unresolved_running_image_cannot_be_verified() {
        let detail = "The image of a running container could not be read";
        assert_part_unverifiable(
            &with("harvest", |c| {
                c.observed_config = None;
                c.error = Some("The content store cannot provide it".into());
            }),
            "harvest",
            detail,
        );
        // Running with no image identity yet.
        assert_part_unverifiable(
            &with("harvest", |c| {
                c.image_id = None;
                c.observed_config = None;
            }),
            "harvest",
            detail,
        );
    }

    #[test]
    fn unresolved_accepted_image_cannot_be_verified() {
        let mut errored = matching();
        let facts = errored.parts.get_mut("braid").unwrap();
        facts.expected_config = None;
        facts.error = Some("The content store cannot provide it".into());
        assert_part_unverifiable(&errored, "braid", "The content store cannot provide it");
        let mut unlisted = matching();
        let facts = unlisted.parts.get_mut("braid").unwrap();
        facts.accepted_digest = None;
        facts.expected_config = None;
        assert_part_unverifiable(
            &unlisted,
            "braid",
            "The accepted image could not be resolved",
        );
        let mut flagged = matching();
        flagged.parts.get_mut("braid").unwrap().error = Some("Unexpected".into());
        assert_part_unverifiable(&flagged, "braid", "Unexpected");
    }

    #[test]
    fn a_difference_outranks_a_part_that_could_not_be_verified() {
        let mut observation = with("harvest", |c| c.image = "sequentech.local/other:1".into());
        observation.containers.retain(|c| c.part != "braid");
        let comparison = compare(&observation, NOW);
        assert_eq!(comparison.outcome, Outcome::Different);
        assert_eq!(comparison.different_parts, ["harvest"]);
        let braid = comparison.parts.iter().find(|p| p.part == "braid").unwrap();
        assert_eq!(braid.outcome, Outcome::Unverifiable);
    }

    #[test]
    fn a_match_needs_its_audit_record_but_a_difference_stands_without_it() {
        let verified = compare(&matching(), NOW).without_audit_record();
        assert_eq!(verified.outcome, Outcome::Unverifiable);
        assert_eq!(verified.reason.as_deref(), Some(AUDIT_REASON));
        let different = compare(
            &with("harvest", |c| c.image = "sequentech.local/other:1".into()),
            NOW,
        )
        .without_audit_record();
        assert_eq!(different.outcome, Outcome::Different);
        assert_eq!(different.reason.as_deref(), Some(AUDIT_REASON));
        let unverifiable =
            Comparison::unverifiable("The host image observation is stale").without_audit_record();
        assert_eq!(
            unverifiable.reason.as_deref(),
            Some("The host image observation is stale")
        );
    }

    fn published() -> serde_json::Value {
        json!({
            "sampled_at": NOW,
            "error": null,
            "digest_list_sha256": "a".repeat(64),
            "toe_reference": "Sequent Voting Platform",
            "tag": "v10.0.0",
            "commit": "abc",
            "parts": {"harvest": {"accepted_digest": digest("harvest"), "expected_config": config("harvest"), "error": null, "content_mismatch": false}},
            "containers": [{"namespace": "step-apps", "pod": "harvest-1", "container": "harvest", "part": "harvest", "image": "sequentech.local/harvest@sha256:0", "image_id": null, "observed_config": null, "running": false, "ready": false, "phase": "Pending", "error": null, "content_mismatch": false}],
        })
    }

    fn file_with(content: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content).unwrap();
        file
    }

    #[test]
    fn the_host_contract_is_strict_and_the_read_is_bounded() {
        let file = file_with(published().to_string().as_bytes());
        let observation = read_observation(file.path()).unwrap();
        assert_eq!(observation.sampled_at, NOW);
        assert_eq!(observation.containers[0].pod, "harvest-1");

        let mut unknown = published();
        unknown["extra"] = json!(true);
        assert!(read_observation(file_with(unknown.to_string().as_bytes()).path()).is_err());
        let mut unknown_container = published();
        unknown_container["containers"][0]["extra"] = json!(true);
        assert!(
            read_observation(file_with(unknown_container.to_string().as_bytes()).path()).is_err()
        );
        let mut missing = published();
        missing.as_object_mut().unwrap().remove("parts");
        assert!(read_observation(file_with(missing.to_string().as_bytes()).path()).is_err());

        let mut padded = published().to_string().into_bytes();
        padded.resize(MAX_STATUS_BYTES as usize + 1, b' ');
        let error = read_observation(file_with(&padded).path()).unwrap_err();
        assert_eq!(error.to_string(), "Invalid host image observation size");
        assert!(read_observation(Path::new("/nonexistent/status.json")).is_err());
    }
}
