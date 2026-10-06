// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `report-manifest.json`: for an event imported from a signed
//! configuration, the hash manifest a report's folder gets beside its files,
//! and the one that lists those manifests for the whole of an output.

use crate::pipes::error::{Error, Result};
use sequent_core::election_config::manifest::{
    sha256_hex, ConfigurationStamp, FileEntry, ReportManifest, ReportManifestFormat,
    REPORT_MANIFEST_NAME,
};
use std::fs;
use std::path::{Path, PathBuf};

fn entry_of(path: &Path, name: String) -> Result<FileEntry> {
    let bytes = fs::read(path)?;
    Ok(FileEntry {
        path: name,
        size: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        members: Vec::new(),
    })
}

/// Writes in `folder` the hash manifest of the files the `report_type`
/// report has there: every file directly in it, by name. Written again, it
/// lists what the folder holds then.
pub fn write_folder_manifest(
    folder: &Path,
    report_type: &str,
    stamp: &ConfigurationStamp,
) -> Result<()> {
    let mut files: Vec<FileEntry> = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == REPORT_MANIFEST_NAME || !entry.file_type()?.is_file() {
            continue;
        }
        files.push(entry_of(&entry.path(), name)?);
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let manifest = ReportManifest {
        format: ReportManifestFormat::V1,
        report_type: report_type.to_string(),
        configuration: stamp.clone(),
        files,
    };
    let bytes = manifest.to_bytes().map_err(|error| {
        Error::UnexpectedError(format!("Error writing the report's hash manifest: {error}"))
    })?;
    fs::write(folder.join(REPORT_MANIFEST_NAME), bytes)?;
    Ok(())
}

/// The hash manifest `folder` holds, if it holds one.
pub fn read_folder_manifest(folder: &Path) -> Result<Option<ReportManifest>> {
    let path = folder.join(REPORT_MANIFEST_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    serde_json::from_slice(&fs::read(&path)?)
        .map(Some)
        .map_err(|error| {
            Error::UnexpectedError(format!(
                "Error reading the hash manifest {}: {error}",
                path.display()
            ))
        })
}

/// The hash manifest of the hash manifests below a folder, as it was
/// written there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedManifests {
    pub manifest: ReportManifest,
    pub sha256: String,
}

fn folders_with_a_manifest(folder: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        if path.join(REPORT_MANIFEST_NAME).is_file() {
            found.push(path.clone());
        }
        folders_with_a_manifest(&path, found)?;
    }
    Ok(())
}

/// Seals the reports below `root` as they are delivered: the hash manifest
/// of every folder below it is written again, to list the files the folder
/// holds now, and `root` gets the hash manifest of those manifests, each
/// by its path from `root`. One digest then stands for all of them.
/// `None`, and nothing written, when no folder below `root` has one.
pub fn seal_report_manifests(root: &Path) -> Result<Option<SealedManifests>> {
    let mut folders = Vec::new();
    folders_with_a_manifest(root, &mut folders)?;
    folders.sort();
    let mut sealed: Option<(String, ConfigurationStamp)> = None;
    let mut files = Vec::new();
    for folder in &folders {
        let Some(manifest) = read_folder_manifest(folder)? else {
            continue;
        };
        write_folder_manifest(folder, &manifest.report_type, &manifest.configuration)?;
        let name = folder
            .strip_prefix(root)
            .map_err(|error| Error::UnexpectedError(error.to_string()))?
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .chain([REPORT_MANIFEST_NAME.to_string()])
            .collect::<Vec<String>>()
            .join("/");
        files.push(entry_of(&folder.join(REPORT_MANIFEST_NAME), name)?);
        sealed.get_or_insert((manifest.report_type, manifest.configuration));
    }
    let Some((report_type, configuration)) = sealed else {
        return Ok(None);
    };
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let manifest = ReportManifest {
        format: ReportManifestFormat::V1,
        report_type,
        configuration,
        files,
    };
    let bytes = manifest.to_bytes().map_err(|error| {
        Error::UnexpectedError(format!("Error writing the reports' hash manifest: {error}"))
    })?;
    fs::write(root.join(REPORT_MANIFEST_NAME), &bytes)?;
    Ok(Some(SealedManifests {
        manifest,
        sha256: sha256_hex(&bytes),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::election_config::manifest::sha256_hex;

    #[test]
    fn sealing_lists_what_each_folder_delivers_and_every_folders_manifest() {
        let root = tempfile::tempdir().unwrap();
        let post = root.path().join("election__1");
        let country = post.join("area__2");
        fs::create_dir_all(&country).unwrap();
        fs::write(post.join("report.html"), b"<main>").unwrap();
        fs::write(post.join("report.json"), b"{}").unwrap();
        write_folder_manifest(&post, "ELECTORAL_RESULTS", &stamp()).unwrap();
        fs::write(country.join("report.html"), b"<main>country").unwrap();
        write_folder_manifest(&country, "ELECTORAL_RESULTS", &stamp()).unwrap();
        fs::create_dir(root.path().join("unstamped")).unwrap();
        fs::write(root.path().join("unstamped/report.html"), b"<main>").unwrap();

        // The Post's rendering is withheld and the country's PDF rendered
        // after their manifests were written.
        fs::remove_file(post.join("report.html")).unwrap();
        fs::write(country.join("report.pdf"), b"%PDF-1.5").unwrap();

        let sealed = seal_report_manifests(root.path()).unwrap().unwrap();

        let names = |manifest: ReportManifest| -> Vec<String> {
            manifest.files.into_iter().map(|file| file.path).collect()
        };
        assert_eq!(names(read(&post)), vec!["report.json"]);
        assert_eq!(names(read(&country)), vec!["report.html", "report.pdf"]);
        assert_eq!(read(&country).files[1].sha256, sha256_hex(b"%PDF-1.5"));

        assert_eq!(read(root.path()), sealed.manifest);
        assert_eq!(
            sealed.sha256,
            sha256_hex(&fs::read(root.path().join(REPORT_MANIFEST_NAME)).unwrap())
        );
        assert_eq!(sealed.manifest.report_type, "ELECTORAL_RESULTS");
        assert_eq!(sealed.manifest.configuration, stamp());
        let listed: Vec<(&str, String)> = sealed
            .manifest
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.sha256.clone()))
            .collect();
        assert_eq!(
            listed,
            vec![
                (
                    "election__1/area__2/report-manifest.json",
                    sha256_hex(&fs::read(country.join(REPORT_MANIFEST_NAME)).unwrap())
                ),
                (
                    "election__1/report-manifest.json",
                    sha256_hex(&fs::read(post.join(REPORT_MANIFEST_NAME)).unwrap())
                ),
            ]
        );

        // Sealed again with nothing changed, it is the same.
        assert_eq!(seal_report_manifests(root.path()).unwrap().unwrap(), sealed);
    }

    #[test]
    fn reports_without_manifests_are_not_sealed() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("election__1")).unwrap();
        fs::write(root.path().join("election__1/report.html"), b"<main>").unwrap();

        assert_eq!(seal_report_manifests(root.path()).unwrap(), None);
        assert!(!root.path().join(REPORT_MANIFEST_NAME).exists());
        assert_eq!(read_folder_manifest(root.path()).unwrap(), None);
        assert!(seal_report_manifests(&root.path().join("missing")).is_err());
    }

    #[test]
    fn a_manifest_that_cannot_be_read_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("election__1");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join(REPORT_MANIFEST_NAME), b"not json").unwrap();

        assert!(read_folder_manifest(&folder).is_err());
        assert!(seal_report_manifests(root.path()).is_err());
    }

    fn stamp() -> ConfigurationStamp {
        ConfigurationStamp {
            external_id: "ov-2028".into(),
            revision: 3,
            manifest_sha256: "ab".repeat(32),
            template_sha256: "cd".repeat(32),
        }
    }

    fn read(folder: &Path) -> ReportManifest {
        serde_json::from_slice(&fs::read(folder.join(REPORT_MANIFEST_NAME)).unwrap()).unwrap()
    }

    #[test]
    fn the_manifest_lists_the_files_of_its_folder_and_nothing_below_it() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("ballots_batch-0.pdf"), b"%PDF-1.5").unwrap();
        fs::write(folder.path().join("ballots_files.csv"), b"file,hash\n").unwrap();
        fs::create_dir(folder.path().join("area__1")).unwrap();
        fs::write(folder.path().join("area__1/report.html"), b"<main>").unwrap();

        write_folder_manifest(folder.path(), "BALLOT_IMAGES", &stamp()).unwrap();
        let manifest = read(folder.path());
        assert_eq!(manifest.format, ReportManifestFormat::V1);
        assert_eq!(manifest.report_type, "BALLOT_IMAGES");
        assert_eq!(manifest.configuration, stamp());
        let listed: Vec<(&str, u64, &str)> = manifest
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.size, file.sha256.as_str()))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("ballots_batch-0.pdf", 8, sha256_hex(b"%PDF-1.5").as_str()),
                ("ballots_files.csv", 10, sha256_hex(b"file,hash\n").as_str()),
            ]
        );
    }

    #[test]
    fn written_again_it_lists_what_the_folder_holds_then_and_never_itself() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("report.html"), b"<main>").unwrap();
        write_folder_manifest(folder.path(), "ELECTORAL_RESULTS", &stamp()).unwrap();
        fs::write(folder.path().join("report.json"), b"{}").unwrap();
        write_folder_manifest(folder.path(), "ELECTORAL_RESULTS", &stamp()).unwrap();

        let names: Vec<String> = read(folder.path())
            .files
            .into_iter()
            .map(|file| file.path)
            .collect();
        assert_eq!(names, vec!["report.html", "report.json"]);
    }

    #[test]
    fn a_folder_that_is_not_there_is_an_error() {
        let folder = tempfile::tempdir().unwrap();
        assert!(
            write_folder_manifest(&folder.path().join("missing"), "BALLOT_IMAGES", &stamp())
                .is_err()
        );
    }
}
