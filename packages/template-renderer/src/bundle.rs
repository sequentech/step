// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned ZIP exchange shared by Studio WASM and the production importer.
//! Nothing is extracted to disk; entries are bounded and validated in memory.
use crate::{
    assets::{self, TemplateAsset, TemplateAssets},
    platform_csv::ImportedTemplate,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read, Write},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub const MAX_ZIP_BYTES: usize = 20_000_000;
pub const MAX_EXPANDED_BYTES: usize = 32_000_000;
const MAX_ENTRIES: usize = 512;
const MAX_TEMPLATES: usize = 64;
const FORMAT: &str = "sequent-template-bundle";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    version: u32,
    templates: Vec<Entry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    record: ImportedTemplate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    document: Option<String>,
    #[serde(default)]
    files: BTreeMap<String, FileRef>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRef {
    file: String,
    mime: String,
}
fn validate_record(row: &ImportedTemplate) -> Result<(), String> {
    if row.alias.is_empty() || row.alias.len() > 240 {
        return Err("Template alias must contain 1–240 bytes".into());
    }
    if uuid::Uuid::parse_str(&row.tenant_id).is_err() {
        return Err("Template tenant_id must be a UUID".into());
    }
    if !row.template.is_object() {
        return Err("Template settings must be an object".into());
    }
    if let Some(document) = row.template.get("document") {
        if !document.is_null() && !document.is_string() {
            return Err("Template source must be text".into());
        }
        if document
            .as_str()
            .is_some_and(|source| source.len() > 300_000)
        {
            return Err("Template source exceeds 300 KB".into());
        }
    }
    if !["EMAIL", "SMS", "DOCUMENT"].contains(&row.communication_method.as_str()) {
        return Err("Unsupported template communication method".into());
    }
    if row.r#type.is_empty() || row.r#type.len() > 100 {
        return Err("Template type is required".into());
    }
    Ok(())
}
pub fn encode(rows: Vec<ImportedTemplate>) -> Result<Vec<u8>, String> {
    if rows.is_empty() || rows.len() > MAX_TEMPLATES {
        return Err("A bundle must contain 1–64 templates".into());
    }
    let mut files = BTreeMap::<String, Vec<u8>>::new();
    let mut templates = Vec::new();
    let mut identities = HashSet::new();
    for (index, mut row) in rows.into_iter().enumerate() {
        validate_record(&row)?;
        if !identities.insert((row.tenant_id.clone(), row.alias.clone())) {
            return Err("Duplicate template alias in bundle".into());
        }
        let assets = assets::from_value(&row.template["assets"])?;
        let decoded = assets::decode(&assets)?;
        let settings = row.template.as_object_mut().unwrap();
        settings.remove("assets");
        let root = format!("templates/{:04}", index + 1);
        let document = if let Some(Value::String(source)) = settings.get("document") {
            if source.len() > 300_000 {
                return Err("Template source exceeds 300 KB".into());
            }
            let path = format!("{root}/template.hbs");
            files.insert(path.clone(), source.as_bytes().to_vec());
            settings.remove("document");
            Some(path)
        } else {
            None
        };
        let refs = decoded
            .into_iter()
            .map(|(path, asset)| {
                let file = format!("{root}/files/{path}");
                files.insert(file.clone(), asset.bytes);
                (
                    path,
                    FileRef {
                        file,
                        mime: asset.mime,
                    },
                )
            })
            .collect();
        if files.len() >= MAX_ENTRIES
            || files.values().map(Vec::len).sum::<usize>() > MAX_EXPANDED_BYTES
        {
            return Err("Template bundle exceeds expanded size or file count limit".into());
        }
        templates.push(Entry {
            record: row,
            document,
            files: refs,
        });
    }
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&Manifest {
            format: FORMAT.into(),
            version: 1,
            templates,
        })
        .map_err(|e| e.to_string())?,
    );
    if files.len() > MAX_ENTRIES || files.values().map(Vec::len).sum::<usize>() > MAX_EXPANDED_BYTES
    {
        return Err("Template bundle exceeds expanded size or file count limit".into());
    }
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);
    for (path, content) in files {
        zip.start_file(path, options).map_err(|e| e.to_string())?;
        zip.write_all(&content).map_err(|e| e.to_string())?;
    }
    let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
    if bytes.len() > MAX_ZIP_BYTES {
        return Err("Template ZIP exceeds 20 MB".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> Result<Vec<ImportedTemplate>, String> {
    if bytes.len() > MAX_ZIP_BYTES {
        return Err("Template ZIP exceeds 20 MB".into());
    }
    let mut zip =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Invalid template ZIP: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err("Template ZIP contains too many entries".into());
    }
    let mut seen = HashSet::new();
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        assets::validate_archive_path(name.strip_suffix('/').unwrap_or(&name))?;
        if !seen.insert(name.to_lowercase()) {
            return Err(format!("Duplicate ZIP entry: {name}"));
        }
        if file.unix_mode().is_some_and(|mode| {
            let kind = mode & 0o170000;
            kind != 0 && kind != 0o100000 && kind != 0o040000
        }) {
            return Err("Links and special files are not permitted in template ZIPs".into());
        }
        if file.is_dir() {
            continue;
        }
        if ![CompressionMethod::Stored, CompressionMethod::Deflated].contains(&file.compression()) {
            return Err("Unsupported ZIP compression method".into());
        }
        let remaining = MAX_EXPANDED_BYTES
            .checked_sub(total)
            .ok_or("Template ZIP exceeds 32 MB expanded")?;
        if file.size() > remaining as u64 {
            return Err("Template ZIP exceeds 32 MB expanded".into());
        }
        let mut content = Vec::new();
        file.by_ref()
            .take(remaining as u64 + 1)
            .read_to_end(&mut content)
            .map_err(|e| e.to_string())?;
        if content.len() > remaining {
            return Err("Template ZIP exceeds 32 MB expanded".into());
        }
        total += content.len();
        files.insert(name, content);
    }
    let manifest = files
        .remove("manifest.json")
        .ok_or("Template ZIP is missing manifest.json")?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest).map_err(|e| format!("Invalid template manifest: {e}"))?;
    if manifest.format != FORMAT || manifest.version != 1 {
        return Err("Unsupported template bundle format or version".into());
    }
    if manifest.templates.is_empty() || manifest.templates.len() > MAX_TEMPLATES {
        return Err("A bundle must contain 1–64 templates".into());
    }
    let mut identities = HashSet::new();
    let mut used = HashSet::new();
    let mut rows = Vec::new();
    for mut entry in manifest.templates {
        validate_record(&entry.record)?;
        if !identities.insert((entry.record.tenant_id.clone(), entry.record.alias.clone())) {
            return Err("Duplicate template alias in bundle".into());
        }
        if entry
            .record
            .template
            .get("document")
            .is_some_and(|value| !value.is_null())
        {
            return Err("Template source must be a declared ZIP file".into());
        }
        if entry.record.template.get("assets").is_some() {
            return Err("Files must be declared in the manifest file map".into());
        }
        let mut read = |path: &str| -> Result<&Vec<u8>, String> {
            assets::validate_archive_path(path)?;
            if !used.insert(path.to_string()) {
                return Err("A ZIP file may only be referenced once".into());
            }
            files
                .get(path)
                .ok_or_else(|| format!("Missing ZIP file: {path}"))
        };
        if let Some(path) = entry.document {
            let source = String::from_utf8(read(&path)?.clone())
                .map_err(|_| "Template source must be UTF-8")?;
            if source.len() > 300_000 {
                return Err("Template source exceeds 300 KB".into());
            }
            entry.record.template["document"] = Value::String(source);
        }
        if entry.files.len() > assets::MAX_FILES {
            return Err("A template supports at most 100 files".into());
        }
        let mut assets = TemplateAssets::new();
        let mut file_bytes = 0usize;
        for (path, file) in entry.files {
            assets::validate_path(&path)?;
            assets::validate_mime(&file.mime)?;
            let bytes = read(&file.file)?;
            file_bytes += bytes.len();
            if bytes.len() > assets::MAX_FILE_BYTES || file_bytes > assets::MAX_TOTAL_BYTES {
                return Err("Template files exceed the file or total size limit".into());
            }
            assets.insert(
                path,
                TemplateAsset {
                    mime: file.mime,
                    base64: STANDARD.encode(bytes),
                },
            );
        }
        assets::decode(&assets)?;
        if !assets.is_empty() {
            entry.record.template["assets"] =
                serde_json::to_value(assets).map_err(|e| e.to_string())?;
        }
        rows.push(entry.record);
    }
    if files.keys().any(|path| !used.contains(path)) {
        return Err("ZIP contains files not declared in its manifest".into());
    }
    Ok(rows)
}
