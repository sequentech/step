// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, HashableBallot};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub(crate) const AUDITABLE_BALLOTS_FILE: &str = "encrypted-ballots.jsonl";

fn write_ballots(
    writer: impl Write,
    ballots: &[String],
    policy: ContestEncryptionPolicy,
) -> Result<()> {
    let mut writer = BufWriter::new(writer);
    // Do not preserve the voter-ID ordering of the merge input.
    let mut ballots = ballots.iter().collect::<Vec<_>>();
    ballots.sort_unstable();
    for content in ballots {
        // The unsigned types omit the voter signing key and signature stored
        // with cast ballots. Excluded ballots are never decrypted.
        match policy {
            ContestEncryptionPolicy::SINGLE_CONTEST => {
                let ballot: HashableBallot = serde_json::from_str(content)?;
                serde_json::to_writer(&mut writer, &ballot)?;
            }
            ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
                let ballot: HashableMultiBallot = serde_json::from_str(content)?;
                serde_json::to_writer(&mut writer, &ballot)?;
            }
        }
        writeln!(writer)?;
    }
    writer.flush()?;
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
    ballots: &[String],
    policy: ContestEncryptionPolicy,
) -> Result<Option<String>> {
    if ballots.is_empty() {
        return Ok(None);
    }
    let file = NamedTempFile::new()?;
    write_ballots(file.as_file(), ballots, policy)?;
    let document = upload_and_return_document(
        transaction,
        file.path().to_str().context("Invalid audit file path")?,
        file.as_file().metadata()?.len(),
        "application/x-ndjson",
        &session.tenant_id,
        Some(session.election_event_id.clone()),
        AUDITABLE_BALLOTS_FILE,
        None,
        false,
    )
    .await?;
    Ok(Some(document.id))
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
            write_ballots(
                File::create(path.join(AUDITABLE_BALLOTS_FILE))?,
                &[ballot],
                policy,
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
                assert!(saved.get("voter_signing_pk").is_none());
                assert!(saved.get("voter_ballot_signature").is_none());
                found = true;
            }
            assert!(found);
        }
        Ok(())
    }
}
