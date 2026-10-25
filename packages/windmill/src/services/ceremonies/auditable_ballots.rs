// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballots a tally leaves out but counts as auditable: discarded ballots
//! and ballots without an eligible voter. The extraction writes them, still
//! encrypted, to a private document of the tally session contest, and the
//! tally results archive carries them in `auditable-ballots/`.

use crate::postgres::document::get_document;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::join::{AuditableBallot, AuditableBallotReason};
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, HashableBallot};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use sequent_core::types::vote_privacy::BallotIdentity;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use tracing::warn;

pub(crate) const AUDITABLE_BALLOTS_FILE: &str = "encrypted-ballots.jsonl";
pub(crate) const AUDITABLE_BALLOTS_FOLDER: &str = "auditable-ballots";
const AUDITABLE_BALLOTS_MEDIA_TYPE: &str = "application/x-ndjson";
/// The `format_version` of `summary.json`. Fields are only ever added.
const SUMMARY_FORMAT_VERSION: u32 = 1;

/// How the records of an archive identify voters, as `summary.json` states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditIdentityMode {
    /// `voter_id` is the voter's id.
    Identity,
    /// `voter_id` is null and `voter_pseudonym` is the voter's pseudonym.
    Pseudonym,
}

impl From<BallotIdentity> for AuditIdentityMode {
    fn from(identity: BallotIdentity) -> Self {
        match identity {
            BallotIdentity::VoterId => AuditIdentityMode::Identity,
            BallotIdentity::Pseudonym => AuditIdentityMode::Pseudonym,
        }
    }
}

#[derive(serde::Serialize)]
struct AuditRecord<'a> {
    cast_vote_id: &'a Option<String>,
    ballot_id: &'a Option<String>,
    voter_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voter_pseudonym: Option<&'a str>,
    reason: &'a AuditableBallotReason,
    #[serde(flatten)]
    ballot: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload_error: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_sha256: Option<String>,
}

/// Writes one JSON line for `record`. The ballot keeps its ciphertexts but
/// not the voter's signing key and signature; it is never decrypted.
pub(crate) fn write_ballot(
    mut writer: impl Write,
    record: &AuditableBallot,
    policy: &ContestEncryptionPolicy,
    identity: AuditIdentityMode,
) -> Result<()> {
    // The unsigned types omit the voter signing key and signature stored
    // with cast ballots.
    let ballot = match policy {
        ContestEncryptionPolicy::SINGLE_CONTEST => {
            serde_json::from_str::<HashableBallot>(&record.content).and_then(serde_json::to_value)
        }
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            serde_json::from_str::<HashableMultiBallot>(&record.content)
                .and_then(serde_json::to_value)
        }
    };
    // Malformed excluded contents must not block tallying eligible ballots.
    // Do not copy raw contents or parser errors: they may expose signing
    // fields. Keep an explicit record and a digest of the stored bytes.
    let (ballot, payload_error, content_sha256) = match ballot {
        Ok(ballot) => (Some(ballot), None, None),
        Err(_) => (
            None,
            Some("invalid_ballot_format"),
            Some(format!("{:x}", Sha256::digest(record.content.as_bytes()))),
        ),
    };
    let (voter_id, voter_pseudonym) = match identity {
        AuditIdentityMode::Identity => (Some(record.voter_id.as_str()), None),
        AuditIdentityMode::Pseudonym => (None, Some(record.voter_id.as_str())),
    };
    serde_json::to_writer(
        &mut writer,
        &AuditRecord {
            cast_vote_id: &record.cast_vote_id,
            ballot_id: &record.ballot_id,
            voter_id,
            voter_pseudonym,
            reason: &record.reason,
            ballot,
            payload_error,
            content_sha256,
        },
    )?;
    writeln!(writer)?;
    Ok(())
}

/// The auditable ballots of one contest area, written as the tally finds
/// them: an empty census can make every ballot auditable, so they are
/// streamed to a file rather than kept in memory.
pub struct AuditableBallotsWriter {
    writer: BufWriter<NamedTempFile>,
    policy: ContestEncryptionPolicy,
    identity: AuditIdentityMode,
    count: u64,
}

/// The finished file of [`AuditableBallotsWriter`].
#[derive(Debug)]
pub struct AuditableBallotsFile {
    pub file: NamedTempFile,
    pub count: u64,
}

impl AuditableBallotsWriter {
    pub fn new(policy: ContestEncryptionPolicy, identity: AuditIdentityMode) -> Result<Self> {
        Ok(AuditableBallotsWriter {
            writer: BufWriter::new(NamedTempFile::new()?),
            policy,
            identity,
            count: 0,
        })
    }

    pub fn write(&mut self, ballot: AuditableBallot) -> Result<()> {
        write_ballot(&mut self.writer, &ballot, &self.policy, self.identity)?;
        self.count += 1;
        Ok(())
    }

    pub fn finish(self) -> Result<AuditableBallotsFile> {
        let count = self.count;
        let file = self
            .writer
            .into_inner()
            .map_err(|error| error.into_error())?;
        Ok(AuditableBallotsFile { file, count })
    }
}

/// The document id of a tally session contest's auditable ballots: fixed per
/// contest area, so a retried extraction replaces its own upload instead of
/// adding another. Version-4 shaped, as the document table requires.
pub fn auditable_ballots_document_id(tally_session_contest_id: &str) -> String {
    let digest = Sha256::digest(format!("auditable-ballots:{tally_session_contest_id}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    uuid::Builder::from_random_bytes(bytes)
        .into_uuid()
        .to_string()
}

/// Stores the auditable ballots of `contest` as the private document its
/// annotations name, in the transaction that records its annotations. The
/// file must hold exactly the ballots the annotations count.
pub async fn save_auditable_ballots(
    transaction: &Transaction<'_>,
    contest: &TallySessionContest,
    ballots: &AuditableBallotsFile,
) -> Result<()> {
    let annotations: TallySessionContestAnnotations = serde_json::from_value(
        contest
            .annotations
            .clone()
            .context("The tally session contest has no annotations")?,
    )?;
    ensure!(
        ballots.count == annotations.ballots_without_voter,
        "The auditable ballots file of tally session contest {} holds {} ballots, not {}",
        contest.id,
        ballots.count,
        annotations.ballots_without_voter
    );
    let Some(document_id) = annotations.auditable_ballots_document_id else {
        ensure!(
            ballots.count == 0,
            "The annotations of tally session contest {} name no auditable ballots document",
            contest.id
        );
        return Ok(());
    };
    upload_and_return_document(
        transaction,
        ballots
            .file
            .path()
            .to_str()
            .context("Invalid audit file path")?,
        ballots.file.as_file().metadata()?.len(),
        AUDITABLE_BALLOTS_MEDIA_TYPE,
        &contest.tenant_id,
        Some(contest.election_event_id.clone()),
        AUDITABLE_BALLOTS_FILE,
        Some(document_id),
        false,
    )
    .await?;
    Ok(())
}

/// Points an imported tally session contest at its imported auditable
/// ballots document.
pub(crate) fn remap_auditable_ballots_document(
    annotations: &mut Option<serde_json::Value>,
    replacement_map: &HashMap<String, String>,
) {
    if let Some(document_id) = annotations
        .as_mut()
        .and_then(|value| value.get_mut("auditable_ballots_document_id"))
    {
        // An import without S3 files retains the count but has no snapshot.
        *document_id = document_id
            .as_str()
            .and_then(|id| replacement_map.get(id))
            .map(|id| serde_json::Value::String(id.clone()))
            .unwrap_or(serde_json::Value::Null);
    }
}

/// The folder of a contest area under `auditable-ballots/`.
fn area_folder(base_path: &Path, session: &TallySessionContest) -> std::path::PathBuf {
    let mut path = base_path
        .join(AUDITABLE_BALLOTS_FOLDER)
        .join(format!("election__{}", session.election_id));
    if let Some(contest_id) = &session.contest_id {
        path = path.join(format!("contest__{contest_id}"));
    }
    path.join(format!("area__{}", session.area_id))
}

/// Writes the `summary.json` of a contest area. With nothing to audit, the
/// ballots file is there and empty.
fn write_summary(
    path: &Path,
    count: u64,
    available: bool,
    identity: AuditIdentityMode,
) -> Result<()> {
    fs::create_dir_all(path)?;
    serde_json::to_writer_pretty(
        File::create(path.join("summary.json"))?,
        &serde_json::json!({
            "format_version": SUMMARY_FORMAT_VERSION,
            "count": count,
            "ballots_available": available,
            "identity_mode": identity,
        }),
    )?;
    if count == 0 {
        File::create(path.join(AUDITABLE_BALLOTS_FILE))?;
    }
    Ok(())
}

/// Adds `auditable-ballots/` to the tally results archive folder
/// `base_path`: per contest area, `summary.json` and, when available,
/// `encrypted-ballots.jsonl`. Sessions tallied before the ballots were kept,
/// and imports without their documents, report `ballots_available: false`.
pub async fn export_auditable_ballots(
    transaction: &Transaction<'_>,
    base_path: &Path,
    sessions: &[TallySessionContest],
    identity: AuditIdentityMode,
) -> Result<()> {
    for session in sessions {
        let Some(value) = session.annotations.clone() else {
            continue;
        };
        let annotations: TallySessionContestAnnotations = serde_json::from_value(value)?;
        let document = match &annotations.auditable_ballots_document_id {
            Some(document_id) => {
                let document = get_document(
                    transaction,
                    &session.tenant_id,
                    Some(session.election_event_id.clone()),
                    document_id,
                )
                .await?;
                if document.is_none() {
                    warn!(
                        tally_session_contest_id = session.id,
                        document_id, "The auditable ballots document is missing"
                    );
                }
                document
            }
            None => None,
        };
        let count = annotations.ballots_without_voter;
        let path = area_folder(base_path, session);
        write_summary(&path, count, count == 0 || document.is_some(), identity)?;
        if let Some(document) = document {
            let file = get_document_as_temp_file(&session.tenant_id, &document).await?;
            ensure!(
                Some(file.as_file().metadata()?.len() as i64) == document.size,
                "Incomplete auditable ballots document"
            );
            fs::copy(file.path(), path.join(AUDITABLE_BALLOTS_FILE))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::compress::create_archive_from_folder;

    fn ballot(policy: &ContestEncryptionPolicy) -> (String, serde_json::Value) {
        let contests = match policy {
            ContestEncryptionPolicy::SINGLE_CONTEST => serde_json::json!(["ciphertext"]),
            ContestEncryptionPolicy::MULTIPLE_CONTESTS => serde_json::json!("ciphertext"),
        };
        let content = serde_json::json!({
            "version": 1, "issue_date": "2026-09-23", "contests": contests,
            "config": "configuration", "ballot_style_hash": "hash",
            "voter_signing_pk": "private-voter-link", "voter_ballot_signature": "signature"
        })
        .to_string();
        (content, contests)
    }

    fn record(reason: AuditableBallotReason, content: String) -> AuditableBallot {
        AuditableBallot {
            reason,
            cast_vote_id: Some("10000000-0000-4000-8000-000000000011".into()),
            ballot_id: Some("ballot-id".into()),
            voter_id: "voter-identifier".into(),
            content,
        }
    }

    const POLICIES: [ContestEncryptionPolicy; 2] = [
        ContestEncryptionPolicy::SINGLE_CONTEST,
        ContestEncryptionPolicy::MULTIPLE_CONTESTS,
    ];

    #[test]
    fn imported_annotations_point_at_the_imported_document() {
        let original = Some(serde_json::json!({
            "ballots_without_voter": 1,
            "auditable_ballots_document_id": "old-document"
        }));
        for (mapping, expected) in [
            (
                HashMap::from([("old-document".to_string(), "new-document".to_string())]),
                serde_json::json!("new-document"),
            ),
            (HashMap::new(), serde_json::Value::Null),
        ] {
            let mut imported = original.clone();
            remap_auditable_ballots_document(&mut imported, &mapping);
            let imported = imported.unwrap();
            assert_eq!(imported["auditable_ballots_document_id"], expected);
            assert_eq!(imported["ballots_without_voter"], 1);
        }
        // Annotations of sessions tallied before the snapshot are untouched.
        let mut legacy = Some(serde_json::json!({"ballots_without_voter": 2}));
        remap_auditable_ballots_document(&mut legacy, &HashMap::new());
        assert_eq!(
            legacy,
            Some(serde_json::json!({"ballots_without_voter": 2}))
        );
    }

    #[test]
    fn audit_archive_retains_encrypted_ballots_without_voter_signatures() -> Result<()> {
        for policy in POLICIES {
            let (content, contests) = ballot(&policy);
            let dir = tempfile::tempdir()?;
            let path = dir.path().join(AUDITABLE_BALLOTS_FOLDER);
            fs::create_dir_all(&path)?;
            write_ballot(
                File::create(path.join(AUDITABLE_BALLOTS_FILE))?,
                &record(AuditableBallotReason::NoEligibleVoter, content),
                &policy,
                AuditIdentityMode::Identity,
            )?;
            let (_guard, archive, _) = create_archive_from_folder(dir.path(), false)?;
            let mut archive = tar::Archive::new(File::open(archive)?);
            let mut found = false;
            for entry in archive.entries()? {
                let mut entry = entry?;
                if !entry.path()?.ends_with(AUDITABLE_BALLOTS_FILE) {
                    continue;
                }
                let mut content = String::new();
                std::io::Read::read_to_string(&mut entry, &mut content)?;
                assert_eq!(content.lines().count(), 1);
                let saved: serde_json::Value = serde_json::from_str(&content)?;
                assert_eq!(saved["contests"], contests);
                assert_eq!(saved["reason"], "no_eligible_voter");
                assert!(saved.get("payload_error").is_none());
                assert_eq!(
                    saved["cast_vote_id"],
                    "10000000-0000-4000-8000-000000000011"
                );
                assert_eq!(saved["ballot_id"], "ballot-id");
                assert_eq!(saved["voter_id"], "voter-identifier");
                assert!(saved.get("voter_pseudonym").is_none());
                assert!(saved.get("voter_signing_pk").is_none());
                assert!(saved.get("voter_ballot_signature").is_none());
                found = true;
            }
            assert!(found);
        }
        Ok(())
    }

    /// A malformed excluded ballot still has one explicit audit record,
    /// including for non-JSON input that cannot safely be redacted.
    #[test]
    fn a_malformed_ballot_is_recorded_by_its_digest() -> Result<()> {
        for policy in POLICIES {
            for content in ["{}", "not JSON: voter_signing_pk=private-voter-link"] {
                let mut bytes = Vec::new();
                write_ballot(
                    &mut bytes,
                    &record(AuditableBallotReason::Discarded, content.into()),
                    &policy,
                    AuditIdentityMode::Identity,
                )?;
                let saved: serde_json::Value = serde_json::from_slice(&bytes)?;
                assert_eq!(saved["voter_id"], "voter-identifier");
                assert_eq!(saved["reason"], "discarded");
                assert_eq!(saved["payload_error"], "invalid_ballot_format");
                assert_eq!(
                    saved["content_sha256"],
                    format!("{:x}", Sha256::digest(content.as_bytes()))
                );
                assert!(saved.get("contests").is_none());
                assert!(!String::from_utf8(bytes)?.contains("private-voter-link"));
            }
        }
        Ok(())
    }

    /// In a pseudonymous event the stored voter is a pseudonym: it is written
    /// as such, and `voter_id` is null.
    #[test]
    fn a_pseudonymous_record_has_a_null_voter_id() -> Result<()> {
        let policy = ContestEncryptionPolicy::MULTIPLE_CONTESTS;
        let (content, _) = ballot(&policy);
        let mut bytes = Vec::new();
        write_ballot(
            &mut bytes,
            &record(AuditableBallotReason::NoEligibleVoter, content),
            &policy,
            AuditIdentityMode::Pseudonym,
        )?;
        let saved: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(saved["voter_id"], serde_json::Value::Null);
        assert_eq!(saved["voter_pseudonym"], "voter-identifier");
        assert_eq!(saved["ballot_id"], "ballot-id");
        Ok(())
    }

    /// The record keys are the record's own plus the unsigned ballot's, with
    /// no collision between them.
    #[test]
    fn record_fields_do_not_collide_with_ballot_fields() -> Result<()> {
        for policy in POLICIES {
            let (content, _) = ballot(&policy);
            let mut bytes = Vec::new();
            write_ballot(
                &mut bytes,
                &record(AuditableBallotReason::NoEligibleVoter, content),
                &policy,
                AuditIdentityMode::Identity,
            )?;
            let saved: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&bytes)?;
            let mut keys: Vec<&str> = saved.keys().map(String::as_str).collect();
            keys.sort();
            assert_eq!(
                keys,
                [
                    "ballot_id",
                    "ballot_style_hash",
                    "cast_vote_id",
                    "config",
                    "contests",
                    "issue_date",
                    "reason",
                    "version",
                    "voter_id"
                ]
            );
        }
        Ok(())
    }

    #[test]
    fn the_writer_counts_what_it_writes() -> Result<()> {
        let policy = ContestEncryptionPolicy::SINGLE_CONTEST;
        let mut writer = AuditableBallotsWriter::new(policy.clone(), AuditIdentityMode::Identity)?;
        for reason in [
            AuditableBallotReason::Discarded,
            AuditableBallotReason::NoEligibleVoter,
        ] {
            writer.write(record(reason, ballot(&policy).0))?;
        }
        let file = writer.finish()?;
        assert_eq!(file.count, 2);
        let content = fs::read_to_string(file.file.path())?;
        assert_eq!(content.lines().count(), 2);
        Ok(())
    }

    #[test]
    fn the_document_id_is_fixed_per_contest_area() {
        let id = auditable_ballots_document_id("contest-area");
        assert_eq!(id, auditable_ballots_document_id("contest-area"));
        assert_ne!(id, auditable_ballots_document_id("another"));
        assert_eq!(uuid::Uuid::parse_str(&id).unwrap().get_version_num(), 4);
    }

    #[test]
    fn the_summary_states_availability_and_identity_mode() -> Result<()> {
        for (count, available, identity) in [
            // Nothing to audit: available, with an empty file.
            (0, true, AuditIdentityMode::Identity),
            // Kept: the document is copied next to the summary.
            (2, true, AuditIdentityMode::Pseudonym),
            // Tallied before the ballots were kept.
            (2, false, AuditIdentityMode::Identity),
        ] {
            let dir = tempfile::tempdir()?;
            write_summary(dir.path(), count, available, identity)?;
            let summary: serde_json::Value =
                serde_json::from_reader(File::open(dir.path().join("summary.json"))?)?;
            assert_eq!(
                summary,
                serde_json::json!({
                    "format_version": 1,
                    "count": count,
                    "ballots_available": available,
                    "identity_mode": identity,
                })
            );
            assert_eq!(
                summary["identity_mode"],
                match identity {
                    AuditIdentityMode::Identity => "identity",
                    AuditIdentityMode::Pseudonym => "pseudonym",
                }
            );
            assert_eq!(dir.path().join(AUDITABLE_BALLOTS_FILE).exists(), count == 0);
        }
        Ok(())
    }

    #[test]
    fn each_contest_area_has_its_folder() {
        let session: TallySessionContest = serde_json::from_value(serde_json::json!({
            "id": "id", "tenant_id": "t", "election_event_id": "ev", "area_id": "a",
            "contest_id": "c", "session_id": 1, "tally_session_id": "s", "election_id": "e",
        }))
        .unwrap();
        assert_eq!(
            area_folder(Path::new("/base"), &session),
            Path::new("/base/auditable-ballots/election__e/contest__c/area__a")
        );
        let session = TallySessionContest {
            contest_id: None,
            ..session
        };
        assert_eq!(
            area_folder(Path::new("/base"), &session),
            Path::new("/base/auditable-ballots/election__e/area__a")
        );
    }
}
