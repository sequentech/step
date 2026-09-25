// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::join::{AuditableBallot, AuditableBallotReason};
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, HashableBallot};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub(crate) const AUDITABLE_BALLOTS_FILE: &str = "encrypted-ballots.jsonl";

#[derive(serde::Serialize)]
struct AuditRecord<'a> {
    cast_vote_id: &'a Option<String>,
    voter_id: &'a str,
    reason: &'a AuditableBallotReason,
    #[serde(flatten)]
    ballot: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload_error: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_sha256: Option<String>,
}

pub(crate) fn write_ballot(
    mut writer: impl Write,
    record: &AuditableBallot,
    policy: &ContestEncryptionPolicy,
) -> Result<()> {
    // The unsigned types omit the voter signing key and signature stored
    // with cast ballots. Excluded ballots are never decrypted.
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
    serde_json::to_writer(
        &mut writer,
        &AuditRecord {
            cast_vote_id: &record.cast_vote_id,
            voter_id: &record.voter_id,
            reason: &record.reason,
            ballot,
            payload_error,
            content_sha256,
        },
    )?;
    writeln!(writer)?;
    Ok(())
}

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

pub async fn save_auditable_ballots(
    transaction: &Transaction<'_>,
    session: &TallySessionContest,
    file: &NamedTempFile,
    count: u64,
) -> Result<Option<String>> {
    if count == 0 {
        return Ok(None);
    }
    let document_id = uuid::Uuid::new_v4().to_string();
    let result = upload_and_return_document(
        transaction,
        file.path().to_str().context("Invalid audit file path")?,
        file.as_file().metadata()?.len(),
        "application/x-ndjson",
        &session.tenant_id,
        Some(session.election_event_id.clone()),
        AUDITABLE_BALLOTS_FILE,
        Some(document_id.clone()),
        false,
    )
    .await;
    match result {
        Ok(document) => Ok(Some(document.id)),
        Err(error) => {
            remove_auditable_upload(session, &document_id).await;
            Err(error)
        }
    }
}

// Only remove uploads known not to have committed. An uncertain commit must
// retain its object so a successfully committed snapshot cannot become dangling.
pub(crate) async fn remove_auditable_upload(session: &TallySessionContest, document_id: &str) {
    let result = async {
        let key = s3::get_document_key(
            &session.tenant_id,
            Some(&session.election_event_id),
            document_id,
            AUDITABLE_BALLOTS_FILE,
        );
        s3::delete_files_from_s3(s3::get_private_bucket()?, key, s3::S3Endpoint::Server).await
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(document_id, error = %error, "Could not clean up failed audit upload");
    }
}

pub async fn export_auditable_ballots(
    transaction: &Transaction<'_>,
    base_path: &Path,
    sessions: &[TallySessionContest],
) -> Result<()> {
    for session in sessions {
        let Some(value) = session.annotations.clone() else {
            continue;
        };
        let annotations: TallySessionContestAnnotations = serde_json::from_value(value)?;
        let mut path = base_path
            .join("auditable-ballots")
            .join(format!("election__{}", session.election_id));
        if let Some(contest_id) = &session.contest_id {
            path = path.join(format!("contest__{contest_id}"));
        }
        path = path.join(format!("area__{}", session.area_id));
        fs::create_dir_all(&path)?;
        let available = annotations.auditable_ballots_document_id.is_some()
            || annotations.ballots_without_voter == 0;
        serde_json::to_writer_pretty(
            File::create(path.join("summary.json"))?,
            &serde_json::json!({
                "format_version": 1,
                "count": annotations.ballots_without_voter,
                "ballots_available": available,
            }),
        )?;
        if let Some(document_id) = annotations.auditable_ballots_document_id {
            let document = get_document(
                transaction,
                &session.tenant_id,
                Some(session.election_event_id.clone()),
                &document_id,
            )
            .await?
            .context("Auditable ballots document is missing")?;
            let file = get_document_as_temp_file(&session.tenant_id, &document).await?;
            ensure!(
                Some(file.as_file().metadata()?.len() as i64) == document.size,
                "Incomplete auditable ballots document"
            );
            fs::copy(file.path(), path.join(AUDITABLE_BALLOTS_FILE))?;
        } else if available {
            File::create(path.join(AUDITABLE_BALLOTS_FILE))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::compress::create_archive_from_folder;

    #[test]
    fn audit_archive_retains_encrypted_ballots_without_voter_signatures() -> Result<()> {
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
        for policy in [
            ContestEncryptionPolicy::SINGLE_CONTEST,
            ContestEncryptionPolicy::MULTIPLE_CONTESTS,
        ] {
            let contests = match policy {
                ContestEncryptionPolicy::SINGLE_CONTEST => serde_json::json!(["ciphertext"]),
                ContestEncryptionPolicy::MULTIPLE_CONTESTS => serde_json::json!("ciphertext"),
            };
            let ballot = serde_json::json!({
                "version": 1, "issue_date": "2026-09-23", "contests": contests,
                "config": "configuration", "ballot_style_hash": "hash",
                "voter_signing_pk": "private-voter-link", "voter_ballot_signature": "signature"
            })
            .to_string();
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("auditable-ballots");
            fs::create_dir_all(&path)?;
            write_ballot(
                File::create(path.join(AUDITABLE_BALLOTS_FILE))?,
                &AuditableBallot {
                    reason: AuditableBallotReason::NoEligibleVoter,
                    cast_vote_id: Some("10000000-0000-4000-8000-000000000011".into()),
                    voter_id: "voter-identifier".into(),
                    content: ballot,
                },
                &policy,
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
                assert_eq!(saved["voter_id"], "voter-identifier");
                assert!(saved.get("voter_signing_pk").is_none());
                assert!(saved.get("voter_ballot_signature").is_none());
                found = true;
            }
            assert!(found);
            // A malformed excluded ballot still has one explicit audit record,
            // including for non-JSON input that cannot safely be redacted.
            for content in ["{}", "not JSON: voter_signing_pk=private-voter-link"] {
                let mut bytes = Vec::new();
                write_ballot(
                    &mut bytes,
                    &AuditableBallot {
                        reason: AuditableBallotReason::Discarded,
                        cast_vote_id: Some("cast-vote-id".into()),
                        voter_id: "voter-id".into(),
                        content: content.into(),
                    },
                    &policy,
                )?;
                let saved: serde_json::Value = serde_json::from_slice(&bytes)?;
                assert_eq!(saved["cast_vote_id"], "cast-vote-id");
                assert_eq!(saved["voter_id"], "voter-id");
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
}
