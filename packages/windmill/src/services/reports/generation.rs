// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What one generation of a report writes: its files, in the copies and
//! formats its report asks for, and for an event imported from a signed
//! configuration `report-manifest.json`, the hash manifest of those files,
//! stored beside them and recorded in the electoral log and the task's.

use super::report_variables::EXECUTION_ANNOTATIONS;
use crate::postgres::reports::ReportType;
use crate::services::documents::upload_and_return_document;
use crate::services::electoral_log::{ElectoralLog, ElectoralLogAdminContext};
use crate::services::protocol_manager::get_event_board;
use crate::services::serialize_tasks_logs::append_general_log;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::ReportGeneratedDetails;
use sequent_core::election_config::archive::{zip, Artifact};
use sequent_core::election_config::manifest::{
    report_manifest, sha256_hex, ConfigurationStamp, ReportManifest, REPORT_MANIFEST_NAME,
};
use sequent_core::types::hasura::core::{DocumentAnnotations, ReportManifestFile, TasksExecution};
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, instrument};
use velvet::pipes::report_manifest::{
    read_folder_manifest, seal_report_manifests, SealedManifests,
};

pub const COPY_NUMBER: &str = "copy_number";
pub const COPY_TOTAL: &str = "copy_total";

const MANIFEST_MEDIA_TYPE: &str = "application/json";
/// Where Velvet writes what each of its pipes produces.
const TALLY_OUTPUT_FOLDER: &str = "output";
const BUNDLE_MEDIA_TYPE: &str = "application/zip";

/// One of the copies a report is printed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportCopy {
    pub number: u32,
    pub total: u32,
}

/// The copies to draw of a report printed `copies` times. A report printed
/// once is drawn as one without copies: its data names none.
pub fn report_copies(copies: u32) -> Vec<Option<ReportCopy>> {
    if copies <= 1 {
        return vec![None];
    }
    (1..=copies)
        .map(|number| {
            Some(ReportCopy {
                number,
                total: copies,
            })
        })
        .collect()
}

/// Adds `values` to a template's data under `execution_annotations`, beside
/// what the report already says there.
pub fn annotate_template_data<const N: usize>(
    data: &mut Map<String, Value>,
    values: [(&'static str, String); N],
) {
    let annotations = data
        .entry(EXECUTION_ANNOTATIONS)
        .or_insert_with(|| Value::Object(Map::new()));
    if !annotations.is_object() {
        *annotations = Value::Object(Map::new());
    }
    if let Value::Object(annotations) = annotations {
        for (name, value) in values {
            annotations.insert(name.to_string(), Value::String(value));
        }
    }
}

/// Names the copy a rendering draws under `execution_annotations`, where
/// templates print "Copy 2 of 7".
pub fn copy_template_data(data: &mut Map<String, Value>, copy: ReportCopy) {
    annotate_template_data(data, copy_annotations(copy));
}

/// The copy as execution annotations name it.
pub fn copy_annotations(copy: ReportCopy) -> [(&'static str, String); 2] {
    [
        (COPY_NUMBER, copy.number.to_string()),
        (COPY_TOTAL, copy.total.to_string()),
    ]
}

/// A file a generation wrote, under the name it is delivered with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub path: String,
    pub media_type: String,
}

/// `report-manifest.json` as a generation wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenManifest {
    pub manifest: ReportManifest,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

fn artifacts(files: &[GeneratedFile]) -> Result<Vec<Artifact>> {
    files
        .iter()
        .map(|file| {
            Ok(Artifact {
                name: file.name.clone(),
                bytes: fs::read(&file.path)
                    .with_context(|| format!("Error reading {} to hash it", file.name))?,
            })
        })
        .collect()
}

/// The hash manifest of `files`, for a report of `report_type` under `stamp`.
pub fn manifest_of(
    report_type: &str,
    stamp: &ConfigurationStamp,
    files: &[Artifact],
) -> Result<WrittenManifest> {
    let manifest = report_manifest(report_type, stamp, files);
    let bytes = manifest
        .to_bytes()
        .context("Error writing the report's hash manifest")?;
    Ok(WrittenManifest {
        sha256: sha256_hex(&bytes),
        manifest,
        bytes,
    })
}

/// The hash manifest of the `files` a stamped generation wrote.
pub fn write_report_manifest(
    report_type: &ReportType,
    stamp: &ConfigurationStamp,
    files: &[GeneratedFile],
) -> Result<WrittenManifest> {
    manifest_of(&report_type.to_string(), stamp, &artifacts(files)?)
}

/// How the one document of a generation is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery<'a> {
    /// As generated: its file, or the zip of its files.
    AsGenerated,
    /// Wrapped in the report's password, as this file.
    Encrypted(&'a GeneratedFile),
    /// Held for its signatures, as this file with its signature page.
    ToSign(&'a GeneratedFile),
}

/// The hash manifest stored with a generation's `document`: it lists the
/// file that is stored, the one an auditor downloads and hashes. An
/// encrypted document is listed beside the file it opens to. A document
/// held for its signatures is the only file there is: what was drawn before
/// its signature page is kept nowhere.
pub fn delivered_manifest(
    generated: &WrittenManifest,
    document: &GeneratedFile,
    delivery: Delivery<'_>,
) -> Result<WrittenManifest> {
    let files = match delivery {
        Delivery::AsGenerated => return Ok(generated.clone()),
        Delivery::Encrypted(stored) => vec![stored.clone(), document.clone()],
        Delivery::ToSign(stored) => vec![stored.clone()],
    };
    manifest_of(
        &generated.manifest.report_type,
        &generated.manifest.configuration,
        &artifacts(&files)?,
    )
}

/// The one document a generation is delivered as: its file when it wrote
/// one, and when it wrote several a zip of them with their hash manifest,
/// written in `directory`.
pub fn bundle(
    prefix: &str,
    files: &[GeneratedFile],
    manifest: Option<&WrittenManifest>,
    directory: &Path,
) -> Result<GeneratedFile> {
    match files {
        [] => Err(anyhow!("The report generated no file")),
        [only] => Ok(only.clone()),
        several => {
            let mut members = artifacts(several)?;
            if let Some(manifest) = manifest {
                members.push(Artifact {
                    name: REPORT_MANIFEST_NAME.to_string(),
                    bytes: manifest.bytes.clone(),
                });
            }
            let bytes = zip(&members).map_err(|problem| anyhow!(problem.message))?;
            let name = format!("{prefix}.zip");
            let path = directory.join(&name);
            fs::write(&path, bytes).context("Error writing the report's files as one zip")?;
            Ok(GeneratedFile {
                name,
                path: path.to_string_lossy().to_string(),
                media_type: BUNDLE_MEDIA_TYPE.to_string(),
            })
        }
    }
}

/// The hash manifest a document's annotations carry, if they carry one.
pub fn manifest_of_document(annotations: Option<&Value>) -> Result<Option<ReportManifest>> {
    let Some(annotations) = annotations.filter(|annotations| !annotations.is_null()) else {
        return Ok(None);
    };
    serde_json::from_value::<DocumentAnnotations>(annotations.clone())
        .context("The document's annotations are malformed")?
        .report_manifest
        .map(|manifest| {
            serde_json::from_value(manifest).context("The document's hash manifest is malformed")
        })
        .transpose()
}

/// The hash manifest of a report released with its signatures, under the
/// configuration `held` names: the `signed` file, and the `stored` one
/// when it is released wrapped in a password.
pub fn released_manifest(
    held: &ReportManifest,
    signed: Artifact,
    stored: Option<Artifact>,
) -> Result<WrittenManifest> {
    let files: Vec<Artifact> = stored.into_iter().chain([signed]).collect();
    manifest_of(&held.report_type, &held.configuration, &files)
}

/// The annotations of a tally's file as it is stored: the hash manifest of
/// its folder, when the folder has one.
pub fn folder_annotations(file: &Path) -> Result<DocumentAnnotations> {
    let manifest = match file.parent() {
        Some(folder) => read_folder_manifest(folder).map_err(|error| anyhow!("{error}"))?,
        None => None,
    };
    Ok(DocumentAnnotations {
        report_manifest: manifest.map(serde_json::to_value).transpose()?,
        ..Default::default()
    })
}

/// Seals each output of the tally in `base_tally_path` as it is archived:
/// see [`seal_report_manifests`]. An output without hash manifests (any
/// event not imported from a signed configuration) has none to seal.
pub fn seal_tally_outputs(base_tally_path: &Path) -> Result<Vec<SealedManifests>> {
    let outputs = base_tally_path.join(TALLY_OUTPUT_FOLDER);
    if !outputs.is_dir() {
        return Ok(Vec::new());
    }
    let mut folders: Vec<PathBuf> = fs::read_dir(&outputs)
        .context("Error reading the tally's outputs")?
        .collect::<std::io::Result<Vec<_>>>()
        .context("Error reading the tally's outputs")?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    let mut sealed = Vec::new();
    for folder in folders {
        sealed.extend(seal_report_manifests(&folder).map_err(|error| anyhow!("{error}"))?);
    }
    Ok(sealed)
}

/// Posts the sealed hash manifests of a tally to the electoral log. The
/// tally keeps them in its archive, so they name no document.
#[instrument(err, skip(hasura_transaction, sealed))]
pub async fn log_sealed_manifests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    sealed: &[SealedManifests],
    requester: &ReportRequester,
) -> Result<()> {
    for sealed in sealed {
        log_report_manifest(
            hasura_transaction,
            tenant_id,
            election_event_id,
            report_generated(&sealed.manifest, &sealed.sha256, None),
            requester,
        )
        .await?;
    }
    Ok(())
}

/// What a generation's task log says of its hash manifest.
pub fn manifest_log(written: &WrittenManifest) -> String {
    format!(
        "{REPORT_MANIFEST_NAME} of the {} report: SHA-256 {}, {} file(s), {}",
        written.manifest.report_type,
        written.sha256,
        written.manifest.files.len(),
        written.manifest.configuration.line(),
    )
}

/// Who asked for a report. One the system generates on its own has no
/// requester.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportRequester {
    pub user_id: Option<String>,
    pub username: Option<String>,
}

impl ReportRequester {
    /// The requester a task knows by name only.
    pub fn named(username: Option<String>) -> Self {
        ReportRequester {
            user_id: None,
            username,
        }
    }
}

impl From<&ElectoralLogAdminContext> for ReportRequester {
    fn from(admin: &ElectoralLogAdminContext) -> Self {
        ReportRequester {
            user_id: Some(admin.user_id.clone()),
            username: admin.username.clone(),
        }
    }
}

/// What the electoral log says of a hash manifest whose bytes hash to
/// `sha256`, stored as `document_id` when it is a document of its own.
pub fn report_generated(
    manifest: &ReportManifest,
    sha256: &str,
    document_id: Option<&str>,
) -> ReportGeneratedDetails {
    ReportGeneratedDetails {
        report_type: manifest.report_type.clone(),
        document_id: document_id.map(str::to_string),
        report_manifest_sha256: sha256.to_string(),
        external_id: manifest.configuration.external_id.clone(),
        revision: manifest.configuration.revision,
        manifest_sha256: manifest.configuration.manifest_sha256.clone(),
    }
}

/// Posts a generated report's hash manifest to its event's electoral log.
#[instrument(err, skip(hasura_transaction))]
pub async fn log_report_manifest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    details: ReportGeneratedDetails,
    requester: &ReportRequester,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name.as_str(),
    )
    .await?;
    electoral_log
        .post_report_generated(
            election_event_id.to_string(),
            details,
            requester.user_id.clone(),
            requester.username.clone(),
        )
        .await
        .context("Error posting the report's hash manifest to the electoral log")
}

/// Links a stored `report-manifest.json` from the annotations of the
/// document it was written for.
pub fn link_report_manifest(
    annotations: &mut DocumentAnnotations,
    written: &WrittenManifest,
    file: ReportManifestFile,
) -> Result<()> {
    annotations.report_manifest = Some(serde_json::to_value(&written.manifest)?);
    annotations.report_manifest_file = Some(file);
    Ok(())
}

/// Stores and logs `written`, and links it from `annotations`, which are
/// those of the document it lists.
#[instrument(err, skip(hasura_transaction, written, annotations))]
pub async fn attach_report_manifest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    written: &WrittenManifest,
    requester: &ReportRequester,
    annotations: &mut DocumentAnnotations,
) -> Result<()> {
    let file = store_report_manifest(
        hasura_transaction,
        tenant_id,
        election_event_id,
        written,
        requester,
    )
    .await?;
    link_report_manifest(annotations, written, file)
}

/// `task` with `message` added to its log, for the update that completes it.
pub fn task_with_log(task: &TasksExecution, message: &str) -> Result<TasksExecution> {
    let mut task = task.clone();
    task.logs = Some(serde_json::to_value(append_general_log(
        &task.logs, message,
    ))?);
    Ok(task)
}

/// Stores a generation's `report-manifest.json` as a document of the
/// report's event, and posts it to the event's electoral log: a manifest
/// that cannot be logged is not stored. The link it returns goes in the
/// annotations of the report's document.
#[instrument(err, skip(hasura_transaction, written))]
pub async fn store_report_manifest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    written: &WrittenManifest,
    requester: &ReportRequester,
) -> Result<ReportManifestFile> {
    let (_temp_path, path, size) =
        write_into_named_temp_file(&written.bytes, "report-manifest-", ".json")
            .context("Error writing the report's hash manifest")?;
    let document = upload_and_return_document(
        hasura_transaction,
        &path,
        size,
        MANIFEST_MEDIA_TYPE,
        tenant_id,
        Some(election_event_id.to_string()),
        REPORT_MANIFEST_NAME,
        None,
        false,
    )
    .await
    .map_err(|err| anyhow!("Error uploading the report's hash manifest: {err:?}"))?;
    info!(document_id = %document.id, "{}", manifest_log(written));
    log_report_manifest(
        hasura_transaction,
        tenant_id,
        election_event_id,
        report_generated(&written.manifest, &written.sha256, Some(&document.id)),
        requester,
    )
    .await?;
    Ok(ReportManifestFile {
        document_id: document.id,
        sha256: written.sha256.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::election_config::manifest::{read_zip, ReportManifestFormat};
    use serde_json::json;
    use tempfile::tempdir;

    fn stamp() -> ConfigurationStamp {
        ConfigurationStamp {
            external_id: "ov-2028".to_string(),
            revision: 3,
            manifest_sha256: "ab".repeat(32),
            template_sha256: "cd".repeat(32),
        }
    }

    fn file(directory: &Path, name: &str, bytes: &[u8]) -> GeneratedFile {
        let path = directory.join(name);
        fs::write(&path, bytes).unwrap();
        GeneratedFile {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            media_type: "application/octet-stream".to_string(),
        }
    }

    fn data(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            other => panic!("not an object: {other}"),
        }
    }

    #[test]
    fn a_report_printed_once_names_no_copy() {
        assert_eq!(report_copies(0), vec![None]);
        assert_eq!(report_copies(1), vec![None]);
    }

    #[test]
    fn a_report_printed_several_times_numbers_its_copies() {
        assert_eq!(
            report_copies(3),
            vec![
                Some(ReportCopy {
                    number: 1,
                    total: 3
                }),
                Some(ReportCopy {
                    number: 2,
                    total: 3
                }),
                Some(ReportCopy {
                    number: 3,
                    total: 3
                }),
            ]
        );
    }

    #[test]
    fn the_copy_is_named_beside_what_the_report_already_says() {
        let mut map = data(json!({"execution_annotations": {"report_hash": "hash"}}));
        copy_template_data(
            &mut map,
            ReportCopy {
                number: 2,
                total: 7,
            },
        );
        assert_eq!(
            Value::Object(map),
            json!({"execution_annotations": {
                "report_hash": "hash",
                "copy_number": "2",
                "copy_total": "7",
            }})
        );

        let mut without = data(json!({"execution_annotations": "none"}));
        copy_template_data(
            &mut without,
            ReportCopy {
                number: 1,
                total: 2,
            },
        );
        assert_eq!(without[EXECUTION_ANNOTATIONS][COPY_TOTAL], "2");
    }

    #[test]
    fn the_manifest_lists_every_file_of_the_generation() {
        let directory = tempdir().unwrap();
        let files = [
            file(directory.path(), "logs.pdf", b"%PDF-1.5"),
            file(directory.path(), "logs.csv", b"id,created\n"),
            file(directory.path(), "logs.sql", b"BEGIN;\nCOMMIT;\n"),
        ];
        let written = write_report_manifest(&ReportType::ACTIVITY_LOGS, &stamp(), &files).unwrap();

        assert_eq!(written.manifest.format, ReportManifestFormat::V1);
        assert_eq!(written.manifest.report_type, "ACTIVITY_LOGS");
        assert_eq!(written.manifest.configuration, stamp());
        let listed: Vec<(&str, u64, &str)> = written
            .manifest
            .files
            .iter()
            .map(|entry| (entry.path.as_str(), entry.size, entry.sha256.as_str()))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("logs.csv", 11, sha256_hex(b"id,created\n").as_str()),
                ("logs.pdf", 8, sha256_hex(b"%PDF-1.5").as_str()),
                ("logs.sql", 15, sha256_hex(b"BEGIN;\nCOMMIT;\n").as_str()),
            ]
        );
        assert_eq!(written.sha256, sha256_hex(&written.bytes));
        assert_eq!(
            serde_json::from_slice::<ReportManifest>(&written.bytes).unwrap(),
            written.manifest
        );
    }

    #[test]
    fn a_file_that_is_gone_cannot_be_hashed() {
        let missing = GeneratedFile {
            name: "gone.pdf".to_string(),
            path: "/nonexistent/gone.pdf".to_string(),
            media_type: "application/pdf".to_string(),
        };
        let refused =
            write_report_manifest(&ReportType::ACTIVITY_LOGS, &stamp(), &[missing]).unwrap_err();
        assert!(refused.to_string().contains("gone.pdf"), "{refused}");
    }

    #[test]
    fn one_file_is_delivered_as_it_is() {
        let directory = tempdir().unwrap();
        let only = file(directory.path(), "results.pdf", b"%PDF-1.5");
        assert_eq!(
            bundle("results", &[only.clone()], None, directory.path()).unwrap(),
            only
        );
        assert!(bundle("results", &[], None, directory.path()).is_err());
    }

    #[test]
    fn several_files_are_delivered_with_their_manifest_in_one_zip() {
        let directory = tempdir().unwrap();
        let files = [
            file(directory.path(), "logs.pdf", b"%PDF-1.5"),
            file(directory.path(), "logs.csv", b"id,created\n"),
        ];
        let written = write_report_manifest(&ReportType::ACTIVITY_LOGS, &stamp(), &files).unwrap();

        let stamped = bundle("logs", &files, Some(&written), directory.path()).unwrap();
        assert_eq!(stamped.name, "logs.zip");
        assert_eq!(stamped.media_type, "application/zip");
        let members = read_zip(&fs::read(&stamped.path).unwrap(), "the bundle").unwrap();
        let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
        assert_eq!(names, vec!["logs.pdf", "logs.csv", "report-manifest.json"]);
        assert_eq!(members[2].bytes, written.bytes);
        for entry in &written.manifest.files {
            let member = members
                .iter()
                .find(|member| member.name == entry.path)
                .unwrap();
            assert_eq!(sha256_hex(&member.bytes), entry.sha256);
        }

        let plain = bundle("plain", &files, None, directory.path()).unwrap();
        let members = read_zip(&fs::read(&plain.path).unwrap(), "the bundle").unwrap();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn an_encrypted_report_lists_the_file_that_is_stored_and_what_it_opens_to() {
        let directory = tempdir().unwrap();
        let drawn = file(directory.path(), "results.pdf", b"%PDF-1.5");
        let generated =
            write_report_manifest(&ReportType::ELECTORAL_RESULTS, &stamp(), &[drawn.clone()])
                .unwrap();
        let stored = file(directory.path(), "results.epdf", b"Salted__ciphertext");

        let written = delivered_manifest(&generated, &drawn, Delivery::Encrypted(&stored)).unwrap();

        assert_eq!(written.manifest.report_type, "ELECTORAL_RESULTS");
        assert_eq!(written.manifest.configuration, stamp());
        let listed: Vec<(&str, &str)> = written
            .manifest
            .files
            .iter()
            .map(|entry| (entry.path.as_str(), entry.sha256.as_str()))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("results.epdf", sha256_hex(b"Salted__ciphertext").as_str()),
                ("results.pdf", sha256_hex(b"%PDF-1.5").as_str()),
            ]
        );
        assert_ne!(written.sha256, generated.sha256);
        assert_eq!(written.sha256, sha256_hex(&written.bytes));
    }

    #[test]
    fn a_report_held_for_signatures_lists_the_file_kept_to_be_signed() {
        let directory = tempdir().unwrap();
        let drawn = file(directory.path(), "returns.pdf", b"%PDF-1.5");
        let generated =
            write_report_manifest(&ReportType::ELECTORAL_RESULTS, &stamp(), &[drawn.clone()])
                .unwrap();
        let to_sign = file(
            directory.path(),
            "returns-to-sign.pdf",
            b"%PDF-1.5 with its signature page",
        );

        let written = delivered_manifest(&generated, &drawn, Delivery::ToSign(&to_sign)).unwrap();

        let listed: Vec<(&str, &str)> = written
            .manifest
            .files
            .iter()
            .map(|entry| (entry.path.as_str(), entry.sha256.as_str()))
            .collect();
        assert_eq!(
            listed,
            vec![(
                "returns-to-sign.pdf",
                sha256_hex(b"%PDF-1.5 with its signature page").as_str()
            )]
        );
    }

    #[test]
    fn a_report_stored_as_generated_keeps_the_manifest_of_its_files() {
        let directory = tempdir().unwrap();
        let drawn = file(directory.path(), "results.pdf", b"%PDF-1.5");
        let generated =
            write_report_manifest(&ReportType::ELECTORAL_RESULTS, &stamp(), &[drawn.clone()])
                .unwrap();
        assert_eq!(
            delivered_manifest(&generated, &drawn, Delivery::AsGenerated).unwrap(),
            generated
        );

        let gone = GeneratedFile {
            name: "results.epdf".to_string(),
            path: "/nonexistent/results.epdf".to_string(),
            media_type: "application/pdf".to_string(),
        };
        assert!(delivered_manifest(&generated, &drawn, Delivery::Encrypted(&gone)).is_err());
    }

    #[test]
    fn the_electoral_log_names_the_manifest_its_document_and_the_configuration() {
        let directory = tempdir().unwrap();
        let files = [file(directory.path(), "logs.csv", b"id\n")];
        let written = write_report_manifest(&ReportType::ACTIVITY_LOGS, &stamp(), &files).unwrap();

        let stored = report_generated(&written.manifest, &written.sha256, Some("document"));
        assert_eq!(
            stored,
            ReportGeneratedDetails {
                report_type: "ACTIVITY_LOGS".to_string(),
                document_id: Some("document".to_string()),
                report_manifest_sha256: written.sha256.clone(),
                external_id: "ov-2028".to_string(),
                revision: 3,
                manifest_sha256: "ab".repeat(32),
            }
        );
        assert_eq!(
            report_generated(&written.manifest, &written.sha256, None).document_id,
            None
        );
    }

    #[test]
    fn a_requester_known_by_name_has_no_user_id() {
        assert_eq!(
            ReportRequester::named(Some("admin".to_string())),
            ReportRequester {
                user_id: None,
                username: Some("admin".to_string()),
            }
        );
        assert_eq!(ReportRequester::named(None), ReportRequester::default());

        let admin = ElectoralLogAdminContext {
            user_id: "admin-id".to_string(),
            username: Some("admin".to_string()),
            authorized_election_ids: None,
            area_id: None,
        };
        assert_eq!(
            ReportRequester::from(&admin),
            ReportRequester {
                user_id: Some("admin-id".to_string()),
                username: Some("admin".to_string()),
            }
        );
    }

    #[test]
    fn a_stored_manifest_is_linked_from_its_documents_annotations() {
        let directory = tempdir().unwrap();
        let files = [file(directory.path(), "letter.pdf", b"%PDF-1.5")];
        let written = write_report_manifest(&ReportType::CREDENTIALS, &stamp(), &files).unwrap();
        let mut annotations = DocumentAnnotations::password_protected("secret");

        link_report_manifest(
            &mut annotations,
            &written,
            ReportManifestFile {
                document_id: "document".to_string(),
                sha256: written.sha256.clone(),
            },
        )
        .unwrap();

        assert_eq!(
            annotations.report_manifest,
            Some(serde_json::to_value(&written.manifest).unwrap())
        );
        assert_eq!(
            annotations.report_manifest_file,
            Some(ReportManifestFile {
                document_id: "document".to_string(),
                sha256: written.sha256.clone(),
            })
        );
        assert_eq!(
            annotations
                .access
                .and_then(|access| access.password_secret_id),
            Some("secret".to_string())
        );
    }

    #[test]
    fn a_documents_annotations_say_which_manifest_it_was_stored_with() {
        let directory = tempdir().unwrap();
        let files = [file(directory.path(), "report.html", b"<main>")];
        let written =
            write_report_manifest(&ReportType::ELECTORAL_RESULTS, &stamp(), &files).unwrap();
        let annotations = json!({"report_manifest": written.manifest});

        assert_eq!(
            manifest_of_document(Some(&annotations)).unwrap(),
            Some(written.manifest)
        );
        assert_eq!(manifest_of_document(None).unwrap(), None);
        assert_eq!(manifest_of_document(Some(&Value::Null)).unwrap(), None);
        assert_eq!(
            manifest_of_document(Some(&json!({"access": {"password_secret_id": "secret"}})))
                .unwrap(),
            None
        );
        assert!(manifest_of_document(Some(&json!({"report_manifest": {"format": 9}}))).is_err());
        assert!(manifest_of_document(Some(&json!("annotations"))).is_err());
    }

    #[test]
    fn a_signed_report_lists_the_file_that_is_released() {
        let directory = tempdir().unwrap();
        let to_sign = [file(directory.path(), "returns-to-sign.pdf", b"%PDF base")];
        let held = write_report_manifest(&ReportType::ELECTORAL_RESULTS, &stamp(), &to_sign)
            .unwrap()
            .manifest;
        let signed = || Artifact {
            name: "returns.pdf".to_string(),
            bytes: b"%PDF base, signed".to_vec(),
        };

        let released = released_manifest(&held, signed(), None).unwrap();
        assert_eq!(released.manifest.report_type, "ELECTORAL_RESULTS");
        assert_eq!(released.manifest.configuration, stamp());
        let listed = |written: &WrittenManifest| -> Vec<(String, String)> {
            written
                .manifest
                .files
                .iter()
                .map(|entry| (entry.path.clone(), entry.sha256.clone()))
                .collect()
        };
        assert_eq!(
            listed(&released),
            vec![("returns.pdf".to_string(), sha256_hex(b"%PDF base, signed"))]
        );

        let wrapped = released_manifest(
            &held,
            signed(),
            Some(Artifact {
                name: "returns.epdf".to_string(),
                bytes: b"Salted__ciphertext".to_vec(),
            }),
        )
        .unwrap();
        assert_eq!(
            listed(&wrapped),
            vec![
                (
                    "returns.epdf".to_string(),
                    sha256_hex(b"Salted__ciphertext")
                ),
                ("returns.pdf".to_string(), sha256_hex(b"%PDF base, signed")),
            ]
        );
    }

    fn tally_folder(base: &Path, pipe: &str, report_type: &str) -> PathBuf {
        let folder = base.join("output").join(pipe).join("election__1");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("report.html"), b"<main>").unwrap();
        fs::write(folder.join("report.json"), b"{}").unwrap();
        velvet::pipes::report_manifest::write_folder_manifest(&folder, report_type, &stamp())
            .unwrap();
        folder
    }

    #[test]
    fn a_tallys_file_is_stored_with_the_manifest_of_its_folder() {
        let base = tempdir().unwrap();
        let folder = tally_folder(base.path(), "velvet-generate-reports", "ELECTORAL_RESULTS");

        let annotations = folder_annotations(&folder.join("report.html")).unwrap();
        let manifest = manifest_of_document(Some(&serde_json::to_value(&annotations).unwrap()))
            .unwrap()
            .unwrap();
        assert_eq!(manifest.report_type, "ELECTORAL_RESULTS");
        assert_eq!(manifest.configuration, stamp());
        assert_eq!(annotations.report_manifest_file, None);

        let plain = base.path().join("plain");
        fs::create_dir(&plain).unwrap();
        assert_eq!(
            folder_annotations(&plain.join("report.html")).unwrap(),
            DocumentAnnotations::default()
        );
        assert_eq!(
            folder_annotations(Path::new("/")).unwrap(),
            DocumentAnnotations::default()
        );
    }

    #[test]
    fn each_output_of_a_tally_is_sealed_and_named_in_the_electoral_log() {
        let base = tempdir().unwrap();
        let images = tally_folder(base.path(), "velvet-ballot-images", "BALLOT_IMAGES");
        let reports = tally_folder(base.path(), "velvet-generate-reports", "ELECTORAL_RESULTS");
        fs::create_dir_all(base.path().join("output/velvet-do-tally")).unwrap();
        fs::write(base.path().join("output/notes.txt"), b"notes").unwrap();
        // The rendering is withheld before the tally is archived.
        fs::remove_file(reports.join("report.html")).unwrap();

        let sealed = seal_tally_outputs(base.path()).unwrap();

        let types: Vec<&str> = sealed
            .iter()
            .map(|sealed| sealed.manifest.report_type.as_str())
            .collect();
        assert_eq!(types, vec!["BALLOT_IMAGES", "ELECTORAL_RESULTS"]);
        for (sealed, folder) in sealed.iter().zip([&images, &reports]) {
            let root = folder.parent().unwrap();
            assert_eq!(
                sealed.sha256,
                sha256_hex(&fs::read(root.join(REPORT_MANIFEST_NAME)).unwrap())
            );
            assert_eq!(
                sealed.manifest.files[0].path,
                "election__1/report-manifest.json"
            );
            let logged = report_generated(&sealed.manifest, &sealed.sha256, None);
            assert_eq!(logged.report_manifest_sha256, sealed.sha256);
            assert_eq!(logged.document_id, None);
            assert_eq!(logged.revision, 3);
        }
        let delivered: ReportManifest =
            serde_json::from_slice(&fs::read(reports.join(REPORT_MANIFEST_NAME)).unwrap()).unwrap();
        let names: Vec<&str> = delivered
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(names, vec!["report.json"]);

        assert!(seal_tally_outputs(&base.path().join("missing"))
            .unwrap()
            .is_empty());
        let unstamped = tempdir().unwrap();
        fs::create_dir_all(unstamped.path().join("output/velvet-generate-reports")).unwrap();
        assert!(seal_tally_outputs(unstamped.path()).unwrap().is_empty());
    }

    #[test]
    fn the_task_log_records_the_manifests_digest() {
        let directory = tempdir().unwrap();
        let files = [file(directory.path(), "logs.csv", b"id\n")];
        let written = write_report_manifest(&ReportType::ACTIVITY_LOGS, &stamp(), &files).unwrap();
        let message = manifest_log(&written);
        assert_eq!(
            message,
            format!(
                "report-manifest.json of the ACTIVITY_LOGS report: SHA-256 {}, 1 file(s), \
                 Configuration revision 3, manifest SHA-256 {}",
                written.sha256,
                "ab".repeat(32)
            )
        );

        let task: TasksExecution = serde_json::from_value(json!({
            "id": "task",
            "tenant_id": "tenant",
            "election_event_id": null,
            "name": "Generate Report",
            "task_type": "GENERATE_REPORT",
            "execution_status": "IN_PROGRESS",
            "created_at": "2026-01-01T00:00:00Z",
            "start_at": null,
            "end_at": null,
            "annotations": null,
            "labels": null,
            "logs": [{"created_date": "2026-01-01T00:00:00Z", "log_text": "Task started"}],
            "executed_by_user": "admin"
        }))
        .unwrap();
        let logged = task_with_log(&task, &message).unwrap();
        let texts: Vec<&str> = logged
            .logs
            .as_ref()
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(|log| log["log_text"].as_str())
            .collect();
        assert_eq!(texts, vec!["Task started", message.as_str()]);
    }
}
