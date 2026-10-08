// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `report-manifest.json` of a transmission package: the hash manifest of
//! the election returns' XML as it is stored, of its compressed file and of
//! the archive that goes to the canvassing servers, for an event imported
//! from a signed configuration.
//!
//! It is a document of its own, linked from the package's documents. The
//! archive cannot hold it: each server's zip is the canvassing system's
//! format, and a manifest inside the archive could not list the archive.

use crate::postgres::document::set_document_annotations;
use crate::postgres::reports::ReportType;
use crate::services::reports::generation::{
    attach_report_manifest, ReportRequester, WrittenManifest,
};
use crate::services::reports::report_variables::configuration_stamp_without_template;
use crate::services::signing::context::PostReach;
use crate::services::signing::SigningCaller;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::archive::Artifact;
use sequent_core::election_config::manifest::{
    file_entries, report_manifest, sha256_hex, ConfigurationStamp,
};
use sequent_core::types::hasura::core::{Document, DocumentAnnotations};
use tracing::instrument;
use uuid::Uuid;

/// The name the archive of every server's package is stored with.
pub const ALL_SERVERS_NAME: &str = "all_servers.zip";

/// The name the returns' XML of a transaction is stored with.
pub fn eml_name(transaction_id: &str) -> String {
    format!("er_{transaction_id}.xml")
}

/// The name the compressed XML of a transaction is stored with.
pub fn xz_name(transaction_id: &str) -> String {
    format!("er_{transaction_id}.xz")
}

/// The returns of a package as its documents store them.
#[derive(Debug, Clone, Copy)]
pub struct StoredReturns<'a> {
    pub transaction_id: &'a str,
    pub eml: &'a [u8],
    pub compressed: &'a [u8],
}

/// The hash manifest of a package under `stamp`: the returns' XML, its
/// compressed file, and the archive with each server's zip and what that
/// zip holds listed under it. An event that was not imported from a signed
/// configuration has no stamp, and its package no manifest.
pub fn package_manifest(
    stamp: Option<&ConfigurationStamp>,
    returns: &StoredReturns<'_>,
    all_servers: &[u8],
) -> Result<Option<WrittenManifest>> {
    let Some(stamp) = stamp else {
        return Ok(None);
    };
    let files = [
        Artifact {
            name: eml_name(returns.transaction_id),
            bytes: returns.eml.to_vec(),
        },
        Artifact {
            name: xz_name(returns.transaction_id),
            bytes: returns.compressed.to_vec(),
        },
        Artifact {
            name: ALL_SERVERS_NAME.to_string(),
            bytes: all_servers.to_vec(),
        },
    ];
    let mut manifest = report_manifest(&ReportType::ELECTORAL_RESULTS.to_string(), stamp, &[]);
    manifest.files = file_entries(&files).map_err(|problem| anyhow!(problem.message))?;
    let bytes = manifest
        .to_bytes()
        .context("Error writing the transmission package's hash manifest")?;
    Ok(Some(WrittenManifest {
        sha256: sha256_hex(&bytes),
        manifest,
        bytes,
    }))
}

/// Who the electoral log names for a package: the person who asked for it,
/// and nobody when the system made it.
pub fn package_requester(caller: Option<&SigningCaller>) -> ReportRequester {
    match caller {
        Some(caller) if caller.reach == PostReach::Labels => ReportRequester {
            user_id: Some(caller.user_id.clone()),
            username: Some(caller.username.clone()),
        },
        _ => ReportRequester::default(),
    }
}

/// The annotations that link a hash manifest, when `annotations` do.
pub fn manifest_link(
    annotations: Option<&serde_json::Value>,
) -> Result<Option<DocumentAnnotations>> {
    let Some(annotations) = annotations.filter(|annotations| !annotations.is_null()) else {
        return Ok(None);
    };
    let annotations: DocumentAnnotations = serde_json::from_value(annotations.clone())
        .context("The document's annotations are malformed")?;
    Ok(annotations
        .report_manifest_file
        .is_some()
        .then_some(annotations))
}

/// Stores and logs the hash manifest of a package whose archive is
/// `all_servers`, and answers the annotations its documents link it with.
/// None for an event that was not imported from a signed configuration.
#[instrument(err, skip(hasura_transaction, returns, all_servers))]
pub async fn store_package_manifest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    returns: &StoredReturns<'_>,
    all_servers: &[u8],
    requester: &ReportRequester,
) -> Result<Option<DocumentAnnotations>> {
    let stamp =
        configuration_stamp_without_template(hasura_transaction, tenant_id, election_event_id)
            .await?;
    let Some(written) = package_manifest(stamp.as_ref(), returns, all_servers)? else {
        return Ok(None);
    };
    let mut annotations = DocumentAnnotations::default();
    attach_report_manifest(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &written,
        requester,
        &mut annotations,
    )
    .await?;
    Ok(Some(annotations))
}

/// Links the hash manifest the `archive` was stored with from the returns'
/// own documents, so that the XML names the manifest that lists it.
#[instrument(err, skip(hasura_transaction, archive))]
pub async fn link_returns(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    archive: &Document,
    returns: [&str; 2],
) -> Result<()> {
    let Some(annotations) = manifest_link(archive.annotations.as_ref())? else {
        return Ok(());
    };
    let tenant = Uuid::parse_str(tenant_id).context("Error parsing the tenant id")?;
    let event =
        Uuid::parse_str(election_event_id).context("Error parsing the election event id")?;
    for document_id in returns {
        let document = Uuid::parse_str(document_id)
            .with_context(|| format!("Error parsing document id {document_id}"))?;
        if !set_document_annotations(hasura_transaction, tenant, event, document, &annotations)
            .await?
        {
            return Err(anyhow!("Can't find document {document_id}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::consolidation::zip::compress_folder_to_zip;
    use crate::services::reports::generation::report_generated;
    use sequent_core::election_config::manifest::{FileEntry, ReportManifest};
    use sequent_core::types::hasura::core::ReportManifestFile;
    use serde_json::json;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    const EML: &[u8] = b"<EML><!-- Configuration revision 3 --></EML>";
    const COMPRESSED: &[u8] = b"\xfd7zXZ\x00 the returns";
    const ENCRYPTED: &[u8] = b"Salted__ the returns";
    const ACM: &[u8] = b"{\"hash\": \"AB\"}";

    fn stamp() -> ConfigurationStamp {
        ConfigurationStamp {
            external_id: "ov-2028".to_string(),
            revision: 3,
            manifest_sha256: "ab".repeat(32),
            template_sha256: sha256_hex(&[]),
        }
    }

    fn returns() -> StoredReturns<'static> {
        StoredReturns {
            transaction_id: "0000001234",
            eml: EML,
            compressed: COMPRESSED,
        }
    }

    fn zip_of(folder: &Path) -> Vec<u8> {
        let target = tempdir().unwrap();
        let path = target.path().join("folder.zip");
        compress_folder_to_zip(folder, &path).unwrap();
        fs::read(path).unwrap()
    }

    /// The archive as `generate_all_servers_document` assembles it, for one
    /// server: its folder with the station's zip. Answers the station's zip
    /// too.
    fn all_servers() -> (Vec<u8>, Vec<u8>) {
        let station = tempdir().unwrap();
        fs::write(station.path().join("er_901.exz"), ENCRYPTED).unwrap();
        fs::write(station.path().join("er_901.json"), ACM).unwrap();
        let station_zip = zip_of(station.path());

        let servers = tempdir().unwrap();
        fs::create_dir(servers.path().join("ccs-1")).unwrap();
        fs::write(servers.path().join("ccs-1/er_901.zip"), &station_zip).unwrap();
        (zip_of(servers.path()), station_zip)
    }

    fn entry<'a>(entries: &'a [FileEntry], path: &str) -> &'a FileEntry {
        entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("{path} is not listed"))
    }

    #[test]
    fn the_manifest_lists_the_returns_xml_as_stored_and_the_archive_that_is_sent() {
        let (archive, station_zip) = all_servers();

        let written = package_manifest(Some(&stamp()), &returns(), &archive)
            .unwrap()
            .expect("a manifest");

        assert_eq!(written.manifest.report_type, "ELECTORAL_RESULTS");
        assert_eq!(written.manifest.configuration, stamp());
        let files = &written.manifest.files;
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["all_servers.zip", "er_0000001234.xml", "er_0000001234.xz"]
        );
        let xml = entry(files, "er_0000001234.xml");
        assert_eq!((xml.size, xml.sha256.as_str()), (44, &*sha256_hex(EML)));
        assert!(xml.members.is_empty());
        assert_eq!(
            entry(files, "er_0000001234.xz").sha256,
            sha256_hex(COMPRESSED)
        );

        let stored = entry(files, "all_servers.zip");
        assert_eq!(stored.sha256, sha256_hex(&archive));
        assert_eq!(stored.size, archive.len() as u64);
        let sent = entry(&stored.members, "ccs-1/er_901.zip");
        assert_eq!(sent.sha256, sha256_hex(&station_zip));
        assert_eq!(
            entry(&sent.members, "er_901.exz").sha256,
            sha256_hex(ENCRYPTED)
        );
        assert_eq!(entry(&sent.members, "er_901.json").sha256, sha256_hex(ACM));

        assert_eq!(written.sha256, sha256_hex(&written.bytes));
        assert_eq!(
            serde_json::from_slice::<ReportManifest>(&written.bytes).unwrap(),
            written.manifest
        );
    }

    #[test]
    fn the_electoral_log_names_the_packages_manifest_and_configuration() {
        let (archive, _) = all_servers();
        let written = package_manifest(Some(&stamp()), &returns(), &archive)
            .unwrap()
            .expect("a manifest");

        let logged = report_generated(&written.manifest, &written.sha256, Some("document"));

        assert_eq!(logged.report_type, "ELECTORAL_RESULTS");
        assert_eq!(logged.document_id.as_deref(), Some("document"));
        assert_eq!(logged.report_manifest_sha256, written.sha256);
        assert_eq!(logged.external_id, "ov-2028");
        assert_eq!(logged.revision, 3);
        assert_eq!(logged.manifest_sha256, "ab".repeat(32));
    }

    #[test]
    fn a_package_of_an_event_without_a_signed_configuration_has_no_manifest() {
        let (archive, _) = all_servers();
        assert_eq!(package_manifest(None, &returns(), &archive).unwrap(), None);
        // Nothing is read of what would have been listed.
        assert_eq!(
            package_manifest(None, &returns(), b"not a zip").unwrap(),
            None
        );
    }

    #[test]
    fn an_archive_that_cannot_be_read_has_no_manifest_written() {
        let refused = package_manifest(Some(&stamp()), &returns(), b"not a zip").unwrap_err();
        assert!(refused.to_string().contains("all_servers.zip"), "{refused}");
    }

    #[test]
    fn the_person_who_asked_for_the_package_is_named_and_the_system_is_not() {
        let mut person = SigningCaller::system("sbei-1");
        person.user_id = "user-id".to_string();
        person.reach = PostReach::Labels;
        assert_eq!(
            package_requester(Some(&person)),
            ReportRequester {
                user_id: Some("user-id".to_string()),
                username: Some("sbei-1".to_string()),
            }
        );
        assert_eq!(
            package_requester(Some(&SigningCaller::system("tally"))),
            ReportRequester::default()
        );
        assert_eq!(package_requester(None), ReportRequester::default());
    }

    #[test]
    fn only_a_document_stored_with_a_manifest_links_one() {
        let linked = DocumentAnnotations {
            report_manifest: Some(json!({"format": "sequent.report-manifest/1"})),
            report_manifest_file: Some(ReportManifestFile {
                document_id: "manifest".to_string(),
                sha256: "cd".repeat(32),
            }),
            ..Default::default()
        };
        assert_eq!(
            manifest_link(Some(&serde_json::to_value(&linked).unwrap())).unwrap(),
            Some(linked)
        );

        assert_eq!(manifest_link(None).unwrap(), None);
        assert_eq!(manifest_link(Some(&serde_json::Value::Null)).unwrap(), None);
        assert_eq!(manifest_link(Some(&json!({}))).unwrap(), None);
        assert_eq!(
            manifest_link(Some(&json!({"access": {"password_secret_id": "secret"}}))).unwrap(),
            None
        );
        assert!(manifest_link(Some(&json!("annotations"))).is_err());
    }
}
