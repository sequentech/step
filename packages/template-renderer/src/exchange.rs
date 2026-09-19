// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::localization;
use serde_json::{json, Value};
const HEADERS: [&str; 10] = [
    "alias",
    "tenant_id",
    "template",
    "created_by",
    "labels",
    "annotations",
    "created_at",
    "updated_at",
    "communication_method",
    "type",
];
fn uuid_v4(s: &str) -> bool {
    uuid::Uuid::parse_str(s)
        .map(|u| u.get_version() == Some(uuid::Version::Random))
        .unwrap_or(false)
}
pub fn import(input: &Value) -> Result<Value, String> {
    let text = input["csv"].as_str().ok_or("CSV must be text")?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| e.to_string())?;
        if index == 0 && record.get(0) == Some("alias") {
            continue;
        }
        if record.len() != 10 {
            return Err(format!("Row {}: expected the 10 Step columns", index + 1));
        }
        if !uuid_v4(&record[1]) {
            return Err(format!(
                "Row {}: tenant_id must be UUID v4; Step would skip this row",
                index + 1
            ));
        }
        let template: Value = serde_json::from_str(&record[2])
            .map_err(|e| format!("Row {}: invalid template JSON: {e}", index + 1))?;
        if !template.is_object() {
            return Err(format!("Row {}: template must be an object", index + 1));
        }
        let mut metadata = serde_json::Map::new();
        for (i, header) in HEADERS.iter().enumerate() {
            metadata.insert(header.to_string(), json!(&record[i]));
        }
        rows.push(json!({"metadata":metadata,"template":template}));
    }
    Ok(json!({"rows":rows,"diagnostics":[]}))
}
fn localized_rows(input: &Value) -> Result<(Vec<Value>, Vec<Value>), String> {
    let metadata = &input["metadata"];
    if !uuid_v4(metadata["tenant_id"].as_str().unwrap_or("")) {
        return Err("Set destination tenant_id to a UUID v4 before export".into());
    }
    let languages = input["languages"]
        .as_array()
        .ok_or("Select export languages")?;
    if languages.is_empty() {
        return Err("Select at least one export language".into());
    }
    let catalog: Value = serde_json::from_str(crate::CATALOG).map_err(|e| e.to_string())?;
    let report = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|report| report["id"] == input["reportType"]);
    let defaults = report.map(|r| &r["translations"]).unwrap_or(&Value::Null);
    let translations = localization::merge_catalogs(defaults, &input["translations"]);
    let channels = export_channels(input)?;
    if languages.len().saturating_mul(channels.len()) > 64 {
        return Err("A bundle must contain 1–64 templates".into());
    }
    let mut aliases = std::collections::HashSet::new();
    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    for lang in languages {
        let language = lang.as_str().ok_or("Language must be text")?;
        let default = input["defaultLanguage"].as_str().unwrap_or("en");
        let mut template = input["template"].as_object().cloned().unwrap_or_default();
        // Studio's authoring fields map to the same SendTemplateBody used by Step.
        for channel in ["email", "sms"] {
            if !template.contains_key(channel) {
                if let Some(config) = report.map(|r| &r["configuration"][channel]) {
                    template.insert(channel.to_string(), config.clone());
                }
            }
        }
        if channels.contains(&"document") {
            let source = localization::language_chain(language, default)
                .iter()
                .find_map(|l| input["overrides"][l].as_str())
                .unwrap_or(input["source"].as_str().ok_or("Source must be text")?);
            let source =
                localization::compile(source, language, default, &translations, &mut diagnostics)?;
            let direction = if matches!(
                language.split('-').next().unwrap_or(language),
                "ar" | "he" | "fa" | "ur"
            ) {
                "rtl"
            } else {
                "ltr"
            };
            template.insert(
                "document".into(),
                json!(format!(
                    "<section lang=\"{}\" dir=\"{direction}\">{source}</section>",
                    handlebars::html_escape(language)
                )),
            );
        }
        for (section, fields) in [
            ("email", &["subject", "plaintext_body", "html_body"][..]),
            ("sms", &["message"][..]),
        ] {
            if let Some(config) = template.get_mut(section).and_then(Value::as_object_mut) {
                localize_fields(
                    config,
                    fields,
                    language,
                    default,
                    &translations,
                    &mut diagnostics,
                )?;
            }
        }
        if let Some(communications) = template.get_mut("communication_templates") {
            for (section, fields) in [
                (
                    "email_config",
                    &["subject", "plaintext_body", "html_body"][..],
                ),
                ("sms_config", &["message"][..]),
            ] {
                if let Some(config) = communications
                    .get_mut(section)
                    .and_then(Value::as_object_mut)
                {
                    localize_fields(
                        config,
                        fields,
                        language,
                        default,
                        &translations,
                        &mut diagnostics,
                    )?;
                }
            }
        }
        for channel in &channels {
            let mut alias = metadata["alias"].as_str().unwrap_or("template").to_string();
            if *channel != "document" {
                alias.push_str(&format!("--{channel}"));
            }
            if languages.len() > 1 {
                alias.push_str(&format!("--{language}"));
            }
            if !aliases.insert(alias.clone()) {
                return Err("Export aliases must be unique".into());
            }
            let mut row_metadata = metadata.as_object().cloned().unwrap_or_default();
            row_metadata.insert("alias".into(), json!(alias));
            row_metadata.insert("communication_method".into(), json!(channel.to_uppercase()));
            let mut channel_template = template.clone();
            channel_template.insert("communication_method".into(), json!(channel.to_uppercase()));
            rows.push(json!({"metadata": row_metadata, "template": channel_template}));
        }
    }
    Ok((rows, diagnostics))
}
/// An explicit channel exports just that channel; otherwise export all enabled ones.
fn export_channels(input: &Value) -> Result<Vec<&str>, String> {
    let allowed = ["document", "email", "sms"];
    if let Some(channel) = input["channel"].as_str() {
        if !allowed.contains(&channel) {
            return Err("Unsupported template channel".into());
        }
        if input["channels"][channel] == false {
            return Err("Enable the selected channel before exporting".into());
        }
        return Ok(vec![channel]);
    }
    if let Some(channels) = input["channels"].as_object() {
        let result: Vec<_> = allowed
            .into_iter()
            .filter(|channel| {
                channels
                    .get(*channel)
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .collect();
        if result.is_empty() {
            return Err("Enable at least one template channel before exporting".into());
        }
        return Ok(result);
    }
    // Existing clients supply only communication_method; keep that exchange path usable.
    Ok(vec![
        match input["metadata"]["communication_method"].as_str() {
            Some("EMAIL") => "email",
            Some("SMS") => "sms",
            _ => "document",
        },
    ])
}
fn localize_fields(
    config: &mut serde_json::Map<String, Value>,
    fields: &[&str],
    language: &str,
    default: &str,
    translations: &Value,
    diagnostics: &mut Vec<Value>,
) -> Result<(), String> {
    for field in fields {
        if let Some(text) = config.get(*field).and_then(Value::as_str) {
            let localized =
                localization::compile(text, language, default, translations, diagnostics)?;
            config.insert((*field).to_string(), json!(localized));
        }
    }
    Ok(())
}

fn export_rows(input: &Value) -> Result<(Vec<Value>, Vec<Value>), String> {
    let Some(templates) = input.get("templates") else {
        return localized_rows(input);
    };
    let templates = templates.as_array().ok_or("Templates must be an array")?;
    if templates.is_empty() || templates.len() > 64 {
        return Err("Select between 1 and 64 templates to export".into());
    }
    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    for template in templates {
        let mut request = template
            .as_object()
            .cloned()
            .ok_or("Template must be an object")?;
        if !request.contains_key("languages") {
            let languages = input
                .get("languages")
                .or_else(|| request.get("supportedLanguages"))
                .cloned()
                .ok_or("Select export languages")?;
            request.insert("languages".into(), languages);
        }
        if let Some(channel) = input.get("channel") {
            request.insert("channel".into(), channel.clone());
        }
        let (template_rows, template_diagnostics) = localized_rows(&Value::Object(request))?;
        if rows.len() + template_rows.len() > 64 {
            return Err("A bundle must contain 1–64 templates".into());
        }
        rows.extend(template_rows);
        diagnostics.extend(template_diagnostics);
    }
    Ok((rows, diagnostics))
}

pub fn export(input: &Value) -> Result<Value, String> {
    let (rows, diagnostics) = export_rows(input)?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(HEADERS).map_err(|e| e.to_string())?;
    for value in rows {
        let mut row: Vec<String> = HEADERS
            .iter()
            .map(|h| value["metadata"][*h].as_str().unwrap_or("").to_string())
            .collect();
        row[2] = value["template"].to_string();
        writer.write_record(row).map_err(|e| e.to_string())?;
    }
    let csv = String::from_utf8(writer.into_inner().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(json!({"csv":csv,"diagnostics":diagnostics}))
}
pub fn export_zip(input: &Value) -> Result<Value, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (rows, diagnostics) = export_rows(input)?;
    let rows = rows
        .into_iter()
        .map(|value| {
            let m = &value["metadata"];
            let text = |key: &str| m[key].as_str().unwrap_or("").to_string();
            let date = |key: &str| {
                let s = text(key);
                if s.is_empty() {
                    Ok(None)
                } else {
                    s.parse()
                        .map(Some)
                        .map_err(|_| format!("Invalid {key} timestamp"))
                }
            };
            let json_cell =
                |key: &str| serde_json::from_str(&text(key)).unwrap_or_else(|_| json!(text(key)));
            Ok(crate::platform_csv::ImportedTemplate {
                alias: text("alias"),
                tenant_id: text("tenant_id"),
                template: value["template"].clone(),
                created_by: text("created_by"),
                labels: Some(json_cell("labels")),
                annotations: Some(json_cell("annotations")),
                created_at: date("created_at")?,
                updated_at: date("updated_at")?,
                communication_method: text("communication_method"),
                r#type: text("type"),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let bytes = crate::bundle::encode(rows)?;
    Ok(
        json!({"base64": STANDARD.encode(bytes), "mime":"application/zip", "diagnostics":diagnostics}),
    )
}
pub fn import_zip(input: &Value) -> Result<Value, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let text = input["base64"].as_str().ok_or("ZIP must be base64")?;
    if text.len() > crate::bundle::MAX_ZIP_BYTES.div_ceil(3) * 4 {
        return Err("Template ZIP exceeds 20 MB".into());
    }
    let bytes = STANDARD.decode(text).map_err(|_| "Invalid ZIP base64")?;
    let rows = crate::bundle::decode(&bytes)?
        .into_iter()
        .map(|row| {
            json!({
                "template": row.template,
                "metadata": {
                    "alias":row.alias, "tenant_id":row.tenant_id, "created_by":row.created_by,
                    "labels":row.labels.map(|v| v.to_string()).unwrap_or_default(),
                    "annotations":row.annotations.map(|v| v.to_string()).unwrap_or_default(),
                    "created_at":row.created_at.map(|v| v.to_rfc3339()).unwrap_or_default(),
                    "updated_at":row.updated_at.map(|v| v.to_rfc3339()).unwrap_or_default(),
                    "communication_method":row.communication_method, "type":row.r#type,
                }
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({"rows":rows,"diagnostics":[]}))
}
