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
    let mut aliases = std::collections::HashSet::new();
    let mut rows = Vec::new();
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
        let mut row_metadata = metadata.as_object().cloned().unwrap_or_default();
        row_metadata.insert("alias".into(), json!(alias));
        rows.push(json!({"metadata": row_metadata, "template": template}));
    }
    Ok((rows, diagnostics))
}
pub fn export(input: &Value) -> Result<Value, String> {
    let (rows, diagnostics) = localized_rows(input)?;
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
    let (rows, diagnostics) = localized_rows(input)?;
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
