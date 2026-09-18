// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Legacy positional CSV decoding retained for migration. Header and malformed
//! tenant rows are skipped, matching the existing import task.
use chrono::{DateTime, Local};
use serde_json::Value;
use std::io::Read;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportedTemplate {
    pub alias: String,
    pub tenant_id: String,
    pub template: Value,
    pub created_by: String,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    pub communication_method: String,
    pub r#type: String,
}
pub fn parse<R: Read>(reader: R) -> Result<Vec<ImportedTemplate>, csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(reader);
    let mut result = Vec::new();
    for row in reader.records() {
        let row = row?;
        let field = |i| row.get(i).unwrap_or("");
        let Ok(tenant) = uuid::Uuid::parse_str(field(1)) else {
            continue;
        };
        if tenant.get_version() != Some(uuid::Version::Random) {
            continue;
        }
        result.push(ImportedTemplate {
            alias: field(0).into(),
            tenant_id: tenant.to_string(),
            template: serde_json::from_str(field(2)).unwrap_or_default(),
            created_by: field(3).into(),
            labels: Some(Value::String(field(4).into())),
            annotations: Some(Value::String(field(5).into())),
            created_at: Some(field(6).parse().unwrap_or_default()),
            updated_at: Some(field(7).parse().unwrap_or_default()),
            communication_method: field(8).into(),
            r#type: field(9).into(),
        });
    }
    Ok(result)
}
