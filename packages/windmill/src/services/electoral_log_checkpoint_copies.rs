// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Write-once copies of published electoral-log checkpoints.
//!
//! Every publication is also written to an S3 bucket with Object Lock, where nobody,
//! the backend included, can change or delete it before its retention ends. The audit
//! compares these copies with the checkpoints published to the Hasura database, so a
//! deleted or changed checkpoint row is detected.

use crate::postgres::electoral_log_checkpoint::PublishedCheckpoint;
use anyhow::{anyhow, ensure, Context, Result};
use aws_sdk_s3 as s3;
use s3::primitives::ByteStream;
use s3::types::{
    BucketLocationConstraint, CreateBucketConfiguration, ObjectLockEnabled, ObjectLockMode,
};
use sequent_core::services::s3::{get_shared_s3_client, S3Endpoint};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;

/// Environment variable with the copy policy: `off`, `best-effort` or `required`.
pub const COPY_POLICY_ENV: &str = "ELECTORAL_LOG_CHECKPOINT_COPY";
/// Environment variable with the bucket of the copies.
pub const COPY_BUCKET_ENV: &str = "ELECTORAL_LOG_CHECKPOINT_BUCKET";
/// Environment variable with the Object Lock mode: `compliance` or `governance`.
pub const COPY_LOCK_MODE_ENV: &str = "ELECTORAL_LOG_CHECKPOINT_LOCK_MODE";
/// Environment variable with the days each copy is locked.
pub const COPY_RETENTION_DAYS_ENV: &str = "ELECTORAL_LOG_CHECKPOINT_RETENTION_DAYS";
/// Bucket used when the variable is unset or empty.
pub const DEFAULT_COPY_BUCKET: &str = "electoral-log-checkpoints";
/// Retention used when the variable is unset or empty.
pub const DEFAULT_COPY_RETENTION_DAYS: u32 = 3650;
const SECONDS_PER_DAY: u64 = 86_400;
const DEFAULT_S3_REGION: &str = "us-east-1";

/// Whether published checkpoints are copied to the write-once bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum CheckpointCopyPolicy {
    /// No copies are written, and audits do not look for them.
    Off,
    /// A failed copy is logged and the publication goes ahead; the audit then
    /// reports the missing copy.
    BestEffort,
    /// A failed copy fails the publication before anything is stored.
    Required,
}

/// Object Lock mode of the copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum CheckpointLockMode {
    /// Nobody can delete a copy or shorten its retention before it ends.
    Compliance,
    /// Users allowed to bypass governance retention can delete copies. Meant for
    /// development and testing.
    Governance,
}

impl From<CheckpointLockMode> for ObjectLockMode {
    fn from(mode: CheckpointLockMode) -> Self {
        match mode {
            CheckpointLockMode::Compliance => ObjectLockMode::Compliance,
            CheckpointLockMode::Governance => ObjectLockMode::Governance,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointCopyConfig {
    pub policy: CheckpointCopyPolicy,
    pub bucket: String,
    pub lock_mode: CheckpointLockMode,
    pub retention_days: u32,
}

impl CheckpointCopyConfig {
    /// Read the configuration; unset or empty variables take their defaults.
    pub fn from_env() -> Result<Self> {
        let var = |name: &str| std::env::var(name).ok();
        Self::from_values(
            var(COPY_POLICY_ENV).as_deref(),
            var(COPY_BUCKET_ENV).as_deref(),
            var(COPY_LOCK_MODE_ENV).as_deref(),
            var(COPY_RETENTION_DAYS_ENV).as_deref(),
        )
    }

    fn from_values(
        policy: Option<&str>,
        bucket: Option<&str>,
        lock_mode: Option<&str>,
        retention_days: Option<&str>,
    ) -> Result<Self> {
        let retention_days = match non_empty(retention_days) {
            None => DEFAULT_COPY_RETENTION_DAYS,
            Some(value) => value
                .parse::<u32>()
                .ok()
                .filter(|days| *days > 0)
                .with_context(|| {
                    format!("{COPY_RETENTION_DAYS_ENV} must be a positive number of days, got {value:?}")
                })?,
        };
        Ok(Self {
            policy: parse_or(COPY_POLICY_ENV, policy, CheckpointCopyPolicy::Off)?,
            bucket: non_empty(bucket).unwrap_or(DEFAULT_COPY_BUCKET).to_string(),
            lock_mode: parse_or(
                COPY_LOCK_MODE_ENV,
                lock_mode,
                CheckpointLockMode::Compliance,
            )?,
            retention_days,
        })
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn parse_or<T: FromStr>(name: &str, value: Option<&str>, default: T) -> Result<T> {
    match non_empty(value) {
        None => Ok(default),
        Some(value) => value
            .parse()
            .map_err(|_| anyhow!("{name} has an unknown value {value:?}")),
    }
}

fn copy_prefix(tenant_id: &str, election_event_id: &str) -> String {
    format!("tenant-{tenant_id}/event-{election_event_id}/")
}

/// Object key of the copy of a checkpoint. Sizes are zero-padded so that copies
/// list in log order.
pub fn copy_key(
    tenant_id: &str,
    election_event_id: &str,
    log_uid: &Uuid,
    tree_size: i64,
) -> String {
    format!(
        "{}log-{log_uid}/size-{tree_size:020}.json",
        copy_prefix(tenant_id, election_event_id)
    )
}

/// Write the copy of a published checkpoint, following the configured policy.
#[instrument(skip(config, checkpoint), err)]
pub async fn store_checkpoint_copy(
    config: &CheckpointCopyConfig,
    tenant_id: &str,
    election_event_id: &str,
    checkpoint: &PublishedCheckpoint,
) -> Result<()> {
    if config.policy == CheckpointCopyPolicy::Off {
        return Ok(());
    }
    match put_copy(config, tenant_id, election_event_id, checkpoint).await {
        Ok(()) => Ok(()),
        Err(error) if config.policy == CheckpointCopyPolicy::BestEffort => {
            tracing::error!(
                "Could not write the write-once copy of the electoral-log checkpoint at size {}: {error:?}",
                checkpoint.tree_size
            );
            Ok(())
        }
        Err(error) => {
            Err(error
                .context("Could not write the write-once copy of the electoral-log checkpoint"))
        }
    }
}

async fn put_copy(
    config: &CheckpointCopyConfig,
    tenant_id: &str,
    election_event_id: &str,
    checkpoint: &PublishedCheckpoint,
) -> Result<()> {
    let client = get_shared_s3_client(S3Endpoint::Server).await?;
    ensure_write_once_bucket(&client, &config.bucket).await?;
    let retain_until =
        SystemTime::now() + Duration::from_secs(u64::from(config.retention_days) * SECONDS_PER_DAY);
    let key = copy_key(
        tenant_id,
        election_event_id,
        &checkpoint.log_uid,
        checkpoint.tree_size,
    );
    client
        .put_object()
        .bucket(&config.bucket)
        .key(&key)
        .content_type("application/json")
        .body(ByteStream::from(serde_json::to_vec(checkpoint)?))
        .object_lock_mode(config.lock_mode.into())
        .object_lock_retain_until_date(retain_until.into())
        .send()
        .await
        .with_context(|| format!("Error writing {key} to bucket {}", config.bucket))?;
    Ok(())
}

/// Create the bucket with Object Lock if it does not exist, and refuse a bucket
/// without it, where copies could be changed or deleted.
async fn ensure_write_once_bucket(client: &s3::Client, bucket: &str) -> Result<()> {
    if client.head_bucket().bucket(bucket).send().await.is_err() {
        let region = client
            .config()
            .region()
            .context("The S3 client has no region")?
            .to_string();
        let mut request = client
            .create_bucket()
            .bucket(bucket)
            .object_lock_enabled_for_bucket(true);
        // S3 refuses an explicit location constraint for its default region.
        if region != DEFAULT_S3_REGION {
            request = request.create_bucket_configuration(
                CreateBucketConfiguration::builder()
                    .location_constraint(BucketLocationConstraint::from(region.as_str()))
                    .build(),
            );
        }
        request
            .send()
            .await
            .with_context(|| format!("Error creating bucket {bucket} with Object Lock"))?;
    }
    let configuration = client
        .get_object_lock_configuration()
        .bucket(bucket)
        .send()
        .await
        .with_context(|| {
            format!("Error reading the Object Lock configuration of bucket {bucket}")
        })?;
    let enabled = configuration
        .object_lock_configuration()
        .and_then(|configuration| configuration.object_lock_enabled())
        == Some(&ObjectLockEnabled::Enabled);
    ensure!(
        enabled,
        "Bucket {bucket} does not have Object Lock enabled, so its copies would not be write-once"
    );
    Ok(())
}

/// The write-once copies of an election event's checkpoints, every stored version
/// of each, and what is wrong with the bucket's contents.
#[derive(Debug, Default)]
pub struct CheckpointCopies {
    pub copies: Vec<PublishedCheckpoint>,
    pub findings: Vec<String>,
}

/// Read every version of every copy of an election event's checkpoints.
#[instrument(skip(config), err)]
pub async fn read_checkpoint_copies(
    config: &CheckpointCopyConfig,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<CheckpointCopies> {
    let client = get_shared_s3_client(S3Endpoint::Server).await?;
    let prefix = copy_prefix(tenant_id, election_event_id);
    let mut result = CheckpointCopies::default();
    let mut key_marker: Option<String> = None;
    let mut version_marker: Option<String> = None;
    loop {
        let page = client
            .list_object_versions()
            .bucket(&config.bucket)
            .prefix(&prefix)
            .set_key_marker(key_marker.clone())
            .set_version_id_marker(version_marker.clone())
            .send()
            .await
            .with_context(|| {
                format!(
                    "Error listing the checkpoint copies in bucket {}",
                    config.bucket
                )
            })?;
        for marker in page.delete_markers() {
            result.findings.push(format!(
                "The write-once copy {} has a delete marker: someone tried to delete it",
                marker.key().unwrap_or_default()
            ));
        }
        for version in page.versions() {
            let key = version.key().context("A listed copy has no key")?;
            let object = client
                .get_object()
                .bucket(&config.bucket)
                .key(key)
                .set_version_id(version.version_id().map(str::to_string))
                .send()
                .await
                .with_context(|| format!("Error reading the checkpoint copy {key}"))?;
            let bytes = object
                .body
                .collect()
                .await
                .with_context(|| format!("Error reading the checkpoint copy {key}"))?
                .into_bytes();
            match serde_json::from_slice::<PublishedCheckpoint>(&bytes) {
                Ok(copy)
                    if key
                        == copy_key(
                            tenant_id,
                            election_event_id,
                            &copy.log_uid,
                            copy.tree_size,
                        ) =>
                {
                    result.copies.push(copy)
                }
                Ok(_) => result.findings.push(format!(
                    "The write-once copy {key} holds a checkpoint of another size"
                )),
                Err(error) => result
                    .findings
                    .push(format!("The write-once copy {key} is malformed: {error}")),
            }
        }
        if page.is_truncated() != Some(true) {
            break;
        }
        key_marker = page.next_key_marker().map(str::to_string);
        version_marker = page.next_version_id_marker().map(str::to_string);
    }
    Ok(result)
}

/// Compare the checkpoints published to the Hasura database with their write-once
/// copies.
pub fn cross_check(
    published: &[PublishedCheckpoint],
    copies: &[PublishedCheckpoint],
) -> Vec<String> {
    let rows: BTreeMap<(Uuid, i64), &PublishedCheckpoint> = published
        .iter()
        .map(|row| ((row.log_uid, row.tree_size), row))
        .collect();
    let mut copied: BTreeMap<(Uuid, i64), BTreeSet<&str>> = BTreeMap::new();
    for copy in copies {
        copied
            .entry((copy.log_uid, copy.tree_size))
            .or_default()
            .insert(copy.root.as_str());
    }
    let mut findings = Vec::new();
    for ((log_uid, size), roots) in &copied {
        if roots.len() > 1 {
            findings.push(format!(
                "The write-once copy of the checkpoint at size {size} of log {log_uid} was overwritten with a different root"
            ));
        }
        match rows.get(&(*log_uid, *size)) {
            None => findings.push(format!(
                "The checkpoint at size {size} of log {log_uid} has a write-once copy but no published row: the row was deleted"
            )),
            Some(row) if !roots.contains(row.root.as_str()) => findings.push(format!(
                "The published checkpoint at size {size} of log {log_uid} differs from its write-once copy"
            )),
            Some(_) => {}
        }
    }
    for (log_uid, size) in rows.keys() {
        if !copied.contains_key(&(*log_uid, *size)) {
            findings.push(format!(
                "The published checkpoint at size {size} of log {log_uid} has no write-once copy"
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint(log: u128, tree_size: i64, root: &str) -> PublishedCheckpoint {
        PublishedCheckpoint {
            board_name: "board".to_string(),
            log_uid: Uuid::from_u128(log),
            tree_size,
            root: root.repeat(64),
            reason: "PERIODIC".to_string(),
            signer_pk: "signer".to_string(),
            signature: "signature".to_string(),
        }
    }

    #[test]
    fn unset_values_take_the_defaults() {
        assert_eq!(
            CheckpointCopyConfig::from_values(None, Some(" "), None, Some("")).unwrap(),
            CheckpointCopyConfig {
                policy: CheckpointCopyPolicy::Off,
                bucket: DEFAULT_COPY_BUCKET.to_string(),
                lock_mode: CheckpointLockMode::Compliance,
                retention_days: DEFAULT_COPY_RETENTION_DAYS,
            }
        );
        assert_eq!(
            CheckpointCopyConfig::from_values(
                Some("best-effort"),
                Some("copies"),
                Some("governance"),
                Some("1")
            )
            .unwrap(),
            CheckpointCopyConfig {
                policy: CheckpointCopyPolicy::BestEffort,
                bucket: "copies".to_string(),
                lock_mode: CheckpointLockMode::Governance,
                retention_days: 1,
            }
        );
    }

    #[test]
    fn invalid_values_name_their_variable() {
        for (values, name) in [
            ((Some("always"), None, None, None), COPY_POLICY_ENV),
            ((None, None, Some("strict"), None), COPY_LOCK_MODE_ENV),
            ((None, None, None, Some("0")), COPY_RETENTION_DAYS_ENV),
            ((None, None, None, Some("ten")), COPY_RETENTION_DAYS_ENV),
        ] {
            let (policy, bucket, mode, days) = values;
            let error = CheckpointCopyConfig::from_values(policy, bucket, mode, days)
                .unwrap_err()
                .to_string();
            assert!(error.contains(name), "{error}");
        }
    }

    #[test]
    fn copy_keys_list_in_log_order() {
        let log = Uuid::from_u128(1);
        let small = copy_key("t", "e", &log, 9);
        let large = copy_key("t", "e", &log, 10);
        assert_eq!(
            small,
            "tenant-t/event-e/log-00000000-0000-0000-0000-000000000001/size-00000000000000000009.json"
        );
        assert!(small < large);
    }

    #[test]
    fn matching_copies_have_no_findings() {
        let published = [checkpoint(1, 5, "a"), checkpoint(1, 9, "b")];
        assert!(cross_check(&published, &published).is_empty());
    }

    #[test]
    fn deleted_changed_uncopied_and_overwritten_checkpoints_are_found() {
        let published = [
            checkpoint(1, 5, "a"),
            checkpoint(1, 9, "c"),
            checkpoint(1, 12, "d"),
        ];
        let copies = [
            checkpoint(1, 3, "e"),
            checkpoint(1, 5, "a"),
            checkpoint(1, 5, "f"),
            checkpoint(1, 9, "b"),
        ];
        let findings = cross_check(&published, &copies);
        assert_eq!(findings.len(), 4, "{findings:?}");
        assert!(findings[0].contains("size 3") && findings[0].contains("was deleted"));
        assert!(findings[1].contains("size 5") && findings[1].contains("overwritten"));
        assert!(findings[2].contains("size 9") && findings[2].contains("differs"));
        assert!(findings[3].contains("size 12") && findings[3].contains("no write-once copy"));
    }

    /// Runs against the S3 endpoint configured for Windmill, such as the development
    /// MinIO, in a bucket of its own that it deletes at the end.
    #[tokio::test]
    #[ignore = "needs an S3 endpoint with Object Lock, such as the development MinIO"]
    async fn copies_are_write_once_and_tampering_is_reported() {
        let config = CheckpointCopyConfig {
            policy: CheckpointCopyPolicy::Required,
            bucket: format!("elog-copies-test-{}", uuid::Uuid::new_v4().simple()),
            lock_mode: CheckpointLockMode::Governance,
            retention_days: 1,
        };
        let tenant = uuid::Uuid::new_v4().to_string();
        let event = uuid::Uuid::new_v4().to_string();
        let original = checkpoint(1, 5, "a");
        store_checkpoint_copy(&config, &tenant, &event, &original)
            .await
            .unwrap();
        let read = read_checkpoint_copies(&config, &tenant, &event)
            .await
            .unwrap();
        assert_eq!(read.copies, vec![original.clone()]);
        assert!(read.findings.is_empty(), "{:?}", read.findings);
        assert!(cross_check(&[original.clone()], &read.copies).is_empty());

        let client = get_shared_s3_client(S3Endpoint::Server).await.unwrap();
        let key = copy_key(&tenant, &event, &Uuid::from_u128(1), 5);
        let versions = client
            .list_object_versions()
            .bucket(&config.bucket)
            .prefix(&key)
            .send()
            .await
            .unwrap();
        let version = versions.versions()[0].version_id().unwrap().to_string();
        let deleted = client
            .delete_object()
            .bucket(&config.bucket)
            .key(&key)
            .version_id(&version)
            .send()
            .await;
        assert!(deleted.is_err(), "a locked copy was deleted");

        let mut forged = original.clone();
        forged.root = "f".repeat(64);
        store_checkpoint_copy(&config, &tenant, &event, &forged)
            .await
            .unwrap();
        client
            .delete_object()
            .bucket(&config.bucket)
            .key(&key)
            .send()
            .await
            .unwrap();
        let read = read_checkpoint_copies(&config, &tenant, &event)
            .await
            .unwrap();
        assert_eq!(read.copies.len(), 2);
        assert!(
            read.findings
                .iter()
                .any(|finding| finding.contains("delete marker")),
            "{:?}",
            read.findings
        );
        let findings = cross_check(&[original], &read.copies);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("overwritten")),
            "{findings:?}"
        );

        let all = client
            .list_object_versions()
            .bucket(&config.bucket)
            .send()
            .await
            .unwrap();
        let entries = all
            .versions()
            .iter()
            .map(|version| (version.key(), version.version_id()))
            .chain(
                all.delete_markers()
                    .iter()
                    .map(|marker| (marker.key(), marker.version_id())),
            );
        for (key, version) in entries {
            client
                .delete_object()
                .bucket(&config.bucket)
                .key(key.unwrap())
                .set_version_id(version.map(str::to_string))
                .bypass_governance_retention(true)
                .send()
                .await
                .unwrap();
        }
        client
            .delete_bucket()
            .bucket(&config.bucket)
            .send()
            .await
            .unwrap();
    }
}
