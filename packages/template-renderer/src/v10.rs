// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Deployment exchange for the unmodified release/10.0 runtime. The Studio
//! archive format remains a separate, versioned authoring exchange.
use crate::{assets, localization};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read, Write},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const MAX_ENTRIES: usize = 512;
const MAX_BYTES: usize = 32_000_000;
const MAX_ZIP: usize = 20_000_000;
const SETTINGS: &[&str] = &[
    "audience_selection",
    "audience_voter_ids",
    "communication_method",
    "schedule_now",
    "schedule_date",
    "email",
    "sms",
    "document",
    "name",
    "alias",
    "pdf_options",
    "report_options",
    "secret_attribute_names",
];

fn catalog() -> Result<Value, String> {
    serde_json::from_str(crate::CATALOG).map_err(|e| e.to_string())
}
fn report<'a>(catalog: &'a Value, input: &Value) -> Result<&'a Value, String> {
    catalog["reports"]
        .as_array()
        .and_then(|reports| {
            reports
                .iter()
                .find(|report| report["id"] == input["reportType"])
        })
        .ok_or_else(|| {
            format!(
                "Unsupported STEP 10.0 template type: {}",
                input["reportType"]
            )
        })
}
fn public_assets(input: &Value, report: &Value) -> Result<bool, String> {
    match input["deployment"].as_str().unwrap_or("auto") {
        "public-assets" => Ok(true),
        "auto" => Ok(report["exportTarget"] == "public-assets"),
        _ => Err("Choose STEP 10.0 automatic deployment or public-assets deployment".into()),
    }
}
fn requests(input: &Value) -> Result<Vec<Value>, String> {
    let Some(templates) = input.get("templates") else {
        return Ok(vec![input.clone()]);
    };
    let templates = templates.as_array().ok_or("Templates must be an array")?;
    if templates.is_empty() || templates.len() > 64 {
        return Err("Select between 1 and 64 templates to export".into());
    }
    templates
        .iter()
        .map(|template| {
            let mut request = template
                .as_object()
                .cloned()
                .ok_or("Template must be an object")?;
            for key in ["deployment", "channel"] {
                if let Some(value) = input.get(key) {
                    request.insert(key.into(), value.clone());
                }
            }
            if !request.contains_key("languages") {
                let languages = input
                    .get("languages")
                    .or_else(|| request.get("supportedLanguages"))
                    .cloned()
                    .ok_or("Select export languages")?;
                request.insert("languages".into(), languages);
            }
            Ok(Value::Object(request))
        })
        .collect()
}
fn languages(input: &Value) -> Result<Vec<&str>, String> {
    let languages = input["languages"]
        .as_array()
        .ok_or("Select export languages")?;
    if languages.is_empty() || languages.len() > 32 {
        return Err("Select between 1 and 32 export languages".into());
    }
    let mut seen = HashSet::new();
    languages
        .iter()
        .map(|language| {
            let language = language.as_str().ok_or("Language must be text")?;
            if language.is_empty()
                || language.len() > 64
                || !language.bytes().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_')
                })
            {
                return Err(format!("Invalid export language: {language:?}"));
            }
            if !seen.insert(language.to_lowercase()) {
                return Err(format!("Duplicate export language: {language}"));
            }
            Ok(language)
        })
        .collect()
}
fn settings(input: &Value, report: &Value) -> Result<Map<String, Value>, String> {
    let config = input.get("template").unwrap_or(&report["configuration"]);
    let config = config
        .as_object()
        .ok_or("Template configuration must be an object")?;
    let mut settings = config.clone();
    for (channel, legacy) in [("email", "email_config"), ("sms", "sms_config")] {
        if !settings.contains_key(channel) {
            if let Some(value) = config
                .get("communication_templates")
                .and_then(|value| value.get(legacy))
            {
                settings.insert(channel.into(), value.clone());
            } else if let Some(value) = report["configuration"].get(channel) {
                settings.insert(channel.into(), value.clone());
            }
        }
    }
    Ok(settings)
}
fn source<'a>(input: &'a Value, report: &'a Value, language: &str) -> Result<&'a str, String> {
    let default = input["defaultLanguage"].as_str().unwrap_or("en");
    localization::language_chain(language, default)
        .iter()
        .find_map(|language| input["overrides"][language].as_str())
        .or_else(|| input["source"].as_str())
        .or_else(|| report["source"].as_str())
        .ok_or_else(|| "Template source must be text".into())
}
struct Localizer<'a> {
    language: &'a str,
    default: &'a str,
    translations: Value,
    diagnostics: &'a mut Vec<Value>,
}
impl Localizer<'_> {
    fn compile(&mut self, source: &str, html: bool) -> Result<String, String> {
        localization::compile_v10(
            source,
            self.language,
            self.default,
            &self.translations,
            self.diagnostics,
            html,
        )
    }
    fn fields(&mut self, settings: &mut Map<String, Value>, channel: &str) -> Result<(), String> {
        for (section, fields) in [
            (
                "email",
                &[
                    ("subject", false),
                    ("plaintext_body", false),
                    ("html_body", true),
                ][..],
            ),
            ("sms", &[("message", false)][..]),
        ] {
            if section != channel {
                continue;
            }
            if let Some(object) = settings.get_mut(section).and_then(Value::as_object_mut) {
                for (key, html) in fields {
                    if let Some(value) = object.get(*key).and_then(Value::as_str) {
                        let compiled = self.compile(value, *html)?;
                        object.insert((*key).into(), json!(compiled));
                    }
                }
            }
        }
        Ok(())
    }
}
fn validate_literal_fields(settings: &Map<String, Value>, document: bool) -> Result<(), String> {
    for (section, fields, context) in [
        (
            "pdf_options",
            &["header_template", "footer_template"][..],
            "PDF headers and footers",
        ),
        (
            "email",
            if document {
                &["subject", "plaintext_body", "html_body"][..]
            } else {
                &[][..]
            },
            "DOCUMENT report notifications",
        ),
    ] {
        for field in fields {
            if settings
                .get(section)
                .and_then(|value| value.get(*field))
                .and_then(Value::as_str)
                .is_some_and(|source| source.contains("{{"))
            {
                return Err(format!(
                    "STEP 10.0 does not evaluate Handlebars in {section}.{field} for {context}. Use literal wording and HTML; translations and runtime bindings are unsupported here. Independent EMAIL templates can use runtime bindings when exported without a DOCUMENT channel."
                ));
            }
        }
    }
    Ok(())
}
fn validate_features(
    settings: &Map<String, Value>,
    public: bool,
    input: &Value,
    report: &Value,
) -> Result<(), String> {
    // These authoring fields have explicit handling above/below; all other
    // configuration must belong to release/10.0's SendTemplateBody contract.
    const AUTHORING_SETTINGS: &[&str] = &[
        "communication_templates",
        "assets",
        "system_template",
        "pre_render",
        "selected_methods",
    ];
    if let Some(key) = settings.keys().find(|key| {
        !SETTINGS.contains(&key.as_str()) && !AUTHORING_SETTINGS.contains(&key.as_str())
    }) {
        return Err(format!(
            "Template setting '{key}' is unsupported by STEP 10.0. Remove it before deployment or keep it in a Studio archive."
        ));
    }
    if settings
        .get("pre_render")
        .and_then(|value| value.get("enabled"))
        == Some(&Value::Bool(true))
    {
        return Err("STEP 10.0 does not support Studio pre-render templates. Disable pre-render before deployment; keep pre-render projects in a Studio archive.".into());
    }
    let files = assets::from_value(settings.get("assets").unwrap_or(&Value::Null))?;
    if !files.is_empty() {
        return Err("STEP 10.0 deployment cannot resolve Studio attachment URLs: native PDF rendering loads a temporary file:// document. Use inline data URLs or absolute deployment URLs in the templates, then remove attached files before CSV or Public-assets ZIP export; retain a Studio archive to preserve the attachments.".into());
    }
    if !public {
        if let Some(wrapper) = settings
            .get("system_template")
            .or_else(|| input.get("wrapper"))
        {
            if !wrapper.is_null() && wrapper != &report["wrapper"] {
                return Err("STEP 10.0 loads the system wrapper from MinIO, so CSV cannot deploy this styling change. Choose Public-assets ZIP to export the system template.".into());
            }
        }
    } else if settings
        .get("secret_attribute_names")
        .and_then(Value::as_array)
        .is_some_and(|names| !names.is_empty())
    {
        return Err("STEP 10.0 reads secret_attribute_names from database templates only. Export the declared user template as CSV; deploy the system wrapper separately as public assets.".into());
    }
    Ok(())
}
fn add_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    assets::validate_archive_path(&path)?;
    if files
        .keys()
        .any(|existing| existing.eq_ignore_ascii_case(&path))
    {
        return Err(format!("Deployment files collide at '{path}'. Export these templates separately or rename the attached file."));
    }
    if files.len() >= MAX_ENTRIES
        || files
            .values()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_add(bytes.len())
            > MAX_BYTES
    {
        return Err("Deployment ZIP exceeds 512 files or 32 MB expanded size".into());
    }
    files.insert(path, bytes);
    Ok(())
}
fn csv(rows: &[Value]) -> Result<String, String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record(super::HEADERS)
        .map_err(|e| e.to_string())?;
    for value in rows {
        let mut row: Vec<String> = super::HEADERS
            .iter()
            .map(|key| value["metadata"][key].as_str().unwrap_or("").to_string())
            .collect();
        row[2] = value["template"].to_string();
        writer.write_record(row).map_err(|e| e.to_string())?;
    }
    String::from_utf8(writer.into_inner().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn zip(files: BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);
    for (path, bytes) in files {
        writer
            .start_file(path, options)
            .map_err(|e| e.to_string())?;
        writer.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    let bytes = writer.finish().map_err(|e| e.to_string())?.into_inner();
    if bytes.len() > MAX_ZIP {
        return Err("Deployment ZIP exceeds 20 MB".into());
    }
    Ok(bytes)
}

pub fn export_v10(input: &Value) -> Result<Value, String> {
    let catalog = catalog()?;
    let requests = requests(input)?;
    let mut public_languages = HashSet::new();
    for input in &requests {
        if public_assets(input, report(&catalog, input)?)? {
            for language in languages(input)? {
                public_languages.insert(language.to_string());
            }
        }
    }
    let mut rows = Vec::new();
    let mut identities = HashSet::new();
    let mut files = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for input in &requests {
        let report = report(&catalog, input)?;
        let public = public_assets(input, report)?;
        let config = settings(input, report)?;
        validate_features(&config, public, input, report)?;
        let languages = languages(input)?;
        let channels = super::export_channels(input)?;
        let document = public || channels.contains(&"document");
        validate_literal_fields(&config, document)?;
        for language in &languages {
            let mut localizer = Localizer {
                language,
                default: input["defaultLanguage"].as_str().unwrap_or("en"),
                translations: localization::merge_catalogs(
                    &report["translations"],
                    &input["translations"],
                ),
                diagnostics: &mut diagnostics,
            };
            let mut settings = config.clone();
            if public {
                let base = report["baseName"]
                    .as_str()
                    .or_else(|| report["id"].as_str())
                    .ok_or("Missing public-assets template basename")?;
                assets::validate_path(base)?;
                if base.contains('/') {
                    return Err("Public-assets basename must not contain directories".into());
                }
                let root = if public_languages.len() > 1 {
                    format!("language/{language}/public-assets")
                } else {
                    "public-assets".into()
                };
                let source = localizer.compile(source(input, report, language)?, true)?;
                let wrapper = config
                    .get("system_template")
                    .and_then(Value::as_str)
                    .or_else(|| input["wrapper"].as_str())
                    .or_else(|| report["wrapper"].as_str())
                    .ok_or("System template must be text")?;
                let wrapper = localizer.compile(wrapper, true)?;
                let extra = json!({
                    "pdf_options": settings.get("pdf_options").cloned().unwrap_or_else(|| report["configuration"]["pdf_options"].clone()),
                    "report_options": settings.get("report_options").cloned().unwrap_or_else(|| report["configuration"]["report_options"].clone()),
                    "communication_templates": {
                        "email_config": settings.get("email").cloned().unwrap_or_else(|| json!({"subject":"", "plaintext_body":"", "html_body":null})),
                        "sms_config": settings.get("sms").cloned().unwrap_or_else(|| json!({"message":""})),
                    },
                });
                add_file(
                    &mut files,
                    format!("{root}/{base}_user.hbs"),
                    source.into_bytes(),
                )?;
                add_file(
                    &mut files,
                    format!("{root}/{base}_system.hbs"),
                    wrapper.into_bytes(),
                )?;
                add_file(
                    &mut files,
                    format!("{root}/{base}_extra_config.json"),
                    serde_json::to_vec_pretty(&extra).map_err(|e| e.to_string())?,
                )?;
            } else {
                let metadata = input["metadata"]
                    .as_object()
                    .ok_or("Set destination template metadata before CSV export")?;
                let tenant = metadata
                    .get("tenant_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !super::uuid_v4(tenant) {
                    return Err("Set destination tenant_id to a UUID v4 before export".into());
                }
                settings.retain(|key, _| SETTINGS.contains(&key.as_str()));
                if channels.contains(&"document") {
                    let source = localizer.compile(source(input, report, language)?, true)?;
                    let direction = if matches!(
                        language.split('-').next().unwrap_or(language),
                        "ar" | "he" | "fa" | "ur"
                    ) {
                        "rtl"
                    } else {
                        "ltr"
                    };
                    settings.insert(
                        "document".into(),
                        json!(format!(
                            "<section lang=\"{language}\" dir=\"{direction}\">{source}</section>"
                        )),
                    );
                } else {
                    settings.remove("document");
                }
                for channel in &channels {
                    let mut metadata = metadata.clone();
                    let mut alias = metadata
                        .get("alias")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if alias.is_empty() {
                        return Err("Set a template alias before CSV export".into());
                    }
                    if *channel != "document" {
                        alias.push_str(&format!("--{channel}"));
                    }
                    if languages.len() > 1 {
                        alias.push_str(&format!("--{language}"));
                    }
                    if !identities.insert((tenant.to_string(), alias.clone())) {
                        return Err(format!("Duplicate template alias: {alias}"));
                    }
                    metadata.insert("alias".into(), json!(alias));
                    metadata.insert("communication_method".into(), json!(channel.to_uppercase()));
                    let mut settings = settings.clone();
                    // The independent communication row is rendered by v10;
                    // DOCUMENT notification wording is passed through literally.
                    localizer.fields(&mut settings, channel)?;
                    settings.insert("communication_method".into(), json!(channel.to_uppercase()));
                    rows.push(json!({"metadata":metadata, "template":settings}));
                    if rows.len() > 64 {
                        return Err(
                            "A STEP 10.0 CSV supports at most 64 templates per export".into()
                        );
                    }
                }
            }
        }
    }
    if files.is_empty() {
        return Ok(
            json!({"target":"step-10.0", "csv":csv(&rows)?, "mime":"text/csv", "filename":"step-10.0-templates.csv", "diagnostics":diagnostics}),
        );
    }
    if !rows.is_empty() {
        add_file(&mut files, "templates.csv".into(), csv(&rows)?.into_bytes())?;
    }
    add_file(&mut files, "README.txt".into(), b"STEP 10.0 template deployment\n\nImport templates.csv, when present, through Admin Portal Templates; then assign its alias to the election event report.\n\nCopy the CONTENTS of public-assets/ into the deployment's PUBLIC_ASSETS_PATH in MinIO. This changes deployment-wide defaults and system print styling. User templates assigned in the database still take precedence.\n\nIf language/<locale>/public-assets/ directories are present, choose ONE locale and deploy the contents of that directory. STEP 10.0 does not select these directories automatically.\n\nThis deployment package contains source templates and configuration. Studio scenario data, attachments and rendered PDFs are not exported. Native PDFs load a temporary file:// document; custom resources must use inline data URLs or absolute deployment URLs.\n".to_vec())?;
    Ok(
        json!({"target":"step-10.0", "base64":STANDARD.encode(zip(files)?), "mime":"application/zip", "filename":"step-10.0-public-assets.zip", "diagnostics":diagnostics}),
    )
}

pub fn import_v10(input: &Value) -> Result<Value, String> {
    if input.get("csv").is_some() {
        let mut result = super::import(input)?;
        annotate_csv_rows(&mut result, &catalog()?);
        result["target"] = json!("step-10.0");
        return Ok(result);
    }
    import_public_assets(input)
}

fn read_zip(input: &Value) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let encoded = input["base64"]
        .as_str()
        .ok_or("Provide CSV text or a base64 public-assets ZIP")?;
    if encoded.len() > MAX_ZIP.div_ceil(3) * 4 {
        return Err("Deployment ZIP exceeds 20 MB".into());
    }
    let bytes = STANDARD.decode(encoded).map_err(|_| "Invalid ZIP base64")?;
    if bytes.len() > MAX_ZIP {
        return Err("Deployment ZIP exceeds 20 MB".into());
    }
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Invalid deployment ZIP: {e}"))?;
    if archive.len() > MAX_ENTRIES {
        return Err("Deployment ZIP exceeds 512 entries".into());
    }
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    let mut names = HashSet::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;
        let path = file.name().to_string();
        assets::validate_archive_path(path.trim_end_matches('/'))?;
        if !names.insert(path.to_lowercase()) {
            return Err(format!("Duplicate deployment ZIP entry: {path}"));
        }
        if let Some(mode) = file.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                return Err("Deployment ZIP must not contain symbolic links".into());
            }
        }
        if file.is_dir() {
            continue;
        }
        if file.size() > MAX_BYTES as u64 || file.size() > (MAX_BYTES - total) as u64 {
            return Err("Deployment ZIP exceeds 32 MB expanded size".into());
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take((MAX_BYTES - total + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        total = total.saturating_add(bytes.len());
        if total > MAX_BYTES {
            return Err("Deployment ZIP exceeds 32 MB expanded size".into());
        }
        files.insert(path, bytes);
    }
    Ok(files)
}
fn asset_location(path: &str) -> Result<(String, Option<String>, String), String> {
    if let Some(tail) = path.strip_prefix("public-assets/") {
        return Ok(("public-assets/".into(), None, tail.into()));
    }
    if let Some(tail) = path.strip_prefix("language/") {
        let (language, tail) = tail
            .split_once('/')
            .ok_or("Invalid language directory in deployment ZIP")?;
        let file = tail
            .strip_prefix("public-assets/")
            .ok_or("Language directories must contain public-assets/")?;
        languages(&json!({"languages":[language]}))?;
        return Ok((
            format!("language/{language}/public-assets/"),
            Some(language.into()),
            file.into(),
        ));
    }
    Ok((String::new(), None, path.into()))
}
fn text_file<'a>(
    files: &'a BTreeMap<String, Vec<u8>>,
    path: &str,
) -> Result<Option<&'a str>, String> {
    files
        .get(path)
        .map(|bytes| {
            if bytes.len() > 300_000 {
                return Err(format!("Template file exceeds 300 KB: {path}"));
            }
            std::str::from_utf8(bytes).map_err(|_| format!("Template file must be UTF-8: {path}"))
        })
        .transpose()
}
fn mime(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "txt" | "hbs" => "text/plain",
        "html" => "text/html",
        _ => "application/octet-stream",
    }
}
fn annotate_csv_rows(result: &mut Value, catalog: &Value) {
    if let Some(rows) = result["rows"].as_array_mut() {
        for row in rows {
            if let Some(report) = catalog["reports"].as_array().and_then(|reports| {
                reports.iter().find(|report| {
                    report["id"].as_str().is_some_and(|id| {
                        Some(id.to_uppercase().as_str()) == row["metadata"]["type"].as_str()
                    })
                })
            }) {
                row["reportType"] = report["id"].clone();
            }
        }
    }
}
fn import_public_assets(input: &Value) -> Result<Value, String> {
    let files = read_zip(input)?;
    let catalog = catalog()?;
    let reports = catalog["reports"]
        .as_array()
        .ok_or("Missing STEP 10.0 catalog")?;
    let basenames: BTreeMap<&str, &Value> = reports
        .iter()
        .filter_map(|report| {
            report["baseName"]
                .as_str()
                .or_else(|| report["id"].as_str())
                .map(|base| (base, report))
        })
        .collect();
    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    if let Some(bytes) = files.get("templates.csv") {
        let csv = std::str::from_utf8(bytes).map_err(|_| "templates.csv must be UTF-8")?;
        let mut imported = super::import(&json!({"csv":csv}))?;
        annotate_csv_rows(&mut imported, &catalog);
        rows.extend(imported["rows"].as_array().cloned().unwrap_or_default());
    }
    let mut groups = BTreeMap::new();
    let mut template_files = HashSet::new();
    for path in files.keys() {
        if matches!(path.as_str(), "templates.csv" | "README.txt") {
            continue;
        }
        let (root, language, tail) = asset_location(path)?;
        for suffix in ["_user.hbs", "_system.hbs", "_extra_config.json"] {
            if let Some(base) = tail.strip_suffix(suffix).filter(|base| !base.contains('/')) {
                let report = basenames
                    .get(base)
                    .ok_or_else(|| format!("Unknown STEP 10.0 public-assets family: {base}"))?;
                groups
                    .entry((root.clone(), base.to_string()))
                    .or_insert((language.clone(), *report));
                template_files.insert(path.clone());
            }
        }
    }
    for ((root, base), (language, report)) in groups {
        let source = text_file(&files, &format!("{root}{base}_user.hbs"))?
            .ok_or_else(|| format!("Missing {base}_user.hbs in deployment ZIP"))?;
        let wrapper = text_file(&files, &format!("{root}{base}_system.hbs"))?
            .or_else(|| report["wrapper"].as_str())
            .ok_or("Missing system template")?;
        let mut config = report["configuration"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        if let Some(extra) = text_file(&files, &format!("{root}{base}_extra_config.json"))? {
            let extra: Value = serde_json::from_str(extra)
                .map_err(|e| format!("Invalid {base}_extra_config.json: {e}"))?;
            let extra = extra
                .as_object()
                .ok_or("Public-assets extra configuration must be an object")?;
            config.extend(extra.clone());
            for (channel, legacy) in [("email", "email_config"), ("sms", "sms_config")] {
                if let Some(value) = extra
                    .get("communication_templates")
                    .and_then(|value| value.get(legacy))
                {
                    config.insert(channel.into(), value.clone());
                }
            }
        }
        config.insert("document".into(), json!(source));
        config.insert("system_template".into(), json!(wrapper));
        let mut attachments = assets::TemplateAssets::new();
        for (path, bytes) in &files {
            if template_files.contains(path)
                || matches!(path.as_str(), "templates.csv" | "README.txt")
            {
                continue;
            }
            let (asset_root, _, relative) = asset_location(path)?;
            if asset_root == root {
                attachments.insert(
                    relative.clone(),
                    assets::TemplateAsset {
                        mime: mime(&relative).into(),
                        base64: STANDARD.encode(bytes),
                    },
                );
            }
        }
        assets::decode(&attachments)?;
        if !attachments.is_empty() {
            config.insert(
                "assets".into(),
                serde_json::to_value(attachments).map_err(|e| e.to_string())?,
            );
        }
        let alias = if let Some(language) = &language {
            format!("{base}--{language}")
        } else {
            base.clone()
        };
        let mut row = json!({
            "reportType":report["id"], "deployment":"public-assets", "template":config,
            "metadata":{"alias":alias,"tenant_id":"","type":report["id"].as_str().unwrap_or(&base).to_uppercase(),"communication_method":"DOCUMENT","created_by":"","labels":"{}","annotations":"{}","created_at":"","updated_at":""},
        });
        if let Some(language) = language {
            row["language"] = json!(language);
        }
        rows.push(row);
        if rows.len() > 64 {
            return Err("Deployment ZIP contains more than 64 templates".into());
        }
    }
    if rows.is_empty() {
        return Err("ZIP contains no STEP 10.0 templates. Use a public-assets deployment ZIP or import a Studio archive through the archive importer.".into());
    }
    if files.keys().any(|path| path.starts_with("language/")) {
        diagnostics.push(json!({"severity":"info","code":"deployment-languages","message":"This package contains separate public-assets locales. Choose one locale directory when deploying to MinIO."}));
    }
    Ok(json!({"target":"step-10.0", "rows":rows, "diagnostics":diagnostics}))
}
