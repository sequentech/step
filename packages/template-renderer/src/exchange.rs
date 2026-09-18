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
pub fn export(input: &Value) -> Result<Value, String> {
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
    let mut aliases = std::collections::HashSet::new();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(HEADERS).map_err(|e| e.to_string())?;
    let mut diagnostics = Vec::new();
    for lang in languages {
        let language = lang.as_str().ok_or("Language must be text")?;
        let default = input["defaultLanguage"].as_str().unwrap_or("en");
        let source = localization::language_chain(language, default)
            .iter()
            .find_map(|l| input["overrides"][l].as_str())
            .unwrap_or(input["source"].as_str().ok_or("Source must be text")?);
        let source = localization::compile(
            source,
            language,
            default,
            &input["translations"],
            &mut diagnostics,
        )?;
        let alias = if languages.len() > 1 {
            format!(
                "{}--{}",
                metadata["alias"].as_str().unwrap_or("template"),
                language
            )
        } else {
            metadata["alias"].as_str().unwrap_or("template").to_string()
        };
        if !aliases.insert(alias.clone()) {
            return Err("Export aliases must be unique".into());
        }
        let mut template = input["template"].as_object().cloned().unwrap_or_default();
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
        for (section, fields) in [
            ("email", &["subject", "plaintext_body", "html_body"][..]),
            ("sms", &["message"][..]),
        ] {
            if let Some(config) = template.get_mut(section).and_then(Value::as_object_mut) {
                for field in fields {
                    if let Some(text) = config.get(*field).and_then(Value::as_str) {
                        let localized = localization::compile(
                            text,
                            language,
                            default,
                            &input["translations"],
                            &mut diagnostics,
                        )?;
                        config.insert((*field).to_string(), json!(localized));
                    }
                }
            }
        }
        // Other ordinary fields (email, SMS, destinations and PDF/report options) survive.
        let mut row: Vec<String> = HEADERS
            .iter()
            .map(|h| metadata[*h].as_str().unwrap_or("").to_string())
            .collect();
        row[0] = alias;
        row[2] = Value::Object(template).to_string();
        writer.write_record(row).map_err(|e| e.to_string())?;
    }
    let csv = String::from_utf8(writer.into_inner().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(json!({"csv":csv,"diagnostics":diagnostics}))
}
