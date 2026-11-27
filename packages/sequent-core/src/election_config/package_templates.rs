// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The message templates a configuration package carries, as the platform
//! stores them. The package writes them as `export_templates-<tenant>.csv`
//! in its Admin Portal settings, one row per template, in the columns the
//! workbook's Templates sheet has (`archive::templates_csv`). Applying a
//! package stores each by its alias.
//!
//! A row's `template.document` is one of:
//! - a template as the Admin Portal stores and exports it: a JSON object
//!   with its email, SMS, WhatsApp, Viber and Messenger parts (approved
//!   template, language, placeholders, buttons), taken as it is;
//! - an email written in the Election Architect (`subject`, `body`,
//!   `body_html`), which becomes the template's email part;
//! - anything else, a document such as a receipt's Handlebars, which
//!   becomes the template's document.

use crate::types::messaging::MessageChannel;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// The columns the package writes, in order.
pub const COLUMNS: [&str; 6] = [
    "alias",
    "name",
    "type",
    "communication_method",
    "template.document",
    "template.selected_methods",
];

/// The parts of a stored template, by key, with the method that sends each,
/// in the order a template's method is chosen in.
const PARTS: [(&str, &str); 6] = [
    ("email", "EMAIL"),
    ("sms", "SMS"),
    ("whatsapp", "WHATSAPP"),
    ("viber", "VIBER"),
    ("messenger", "MESSENGER"),
    ("document", "DOCUMENT"),
];

/// Keys only a stored template has.
const STORED_KEYS: [&str; 8] = [
    "email",
    "sms",
    "whatsapp",
    "viber",
    "messenger",
    "document",
    "pdf_options",
    "selected_methods",
];

/// One template, as the platform stores it.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageTemplate {
    pub alias: String,
    /// The platform's template type (`CREDENTIALS`, `BALLOT_RECEIPT`, ...).
    pub template_type: String,
    /// The method it is listed under: the first it sends by.
    pub communication_method: String,
    /// The stored template: its name, alias, parts and selected methods.
    pub template: Value,
}

/// The platform's name for a kind the Election Architect names otherwise.
fn platform_type(kind: &str) -> String {
    match kind {
        "VOTE_RECEIPT" => "BALLOT_RECEIPT".to_string(),
        "TALLY_RESULTS" => "ELECTORAL_RESULTS".to_string(),
        other => other.to_string(),
    }
}

fn stored_template(document: &str) -> Map<String, Value> {
    let parsed = serde_json::from_str::<Value>(document).ok();
    match parsed {
        Some(Value::Object(object))
            if STORED_KEYS.iter().any(|key| object.contains_key(*key)) =>
        {
            object
        }
        Some(Value::Object(object))
            if ["subject", "body", "body_html"]
                .iter()
                .any(|key| object.contains_key(*key)) =>
        {
            let text = |key: &str| {
                object.get(key).and_then(Value::as_str).unwrap_or_default()
            };
            let mut stored = Map::new();
            stored.insert(
                "email".into(),
                serde_json::json!({
                    "subject": text("subject"),
                    "plaintext_body": text("body"),
                    "html_body": text("body_html"),
                }),
            );
            stored
        }
        _ => {
            let mut stored = Map::new();
            stored.insert("document".into(), Value::String(document.into()));
            stored
        }
    }
}

/// The methods a template sends by: the cell's, else those it has a part
/// for.
fn selected_methods(cell: &str, stored: &Map<String, Value>) -> Value {
    let given = match serde_json::from_str::<Value>(cell) {
        Ok(Value::Object(object)) if !object.is_empty() => Some(object),
        _ => None,
    };
    let methods = given.unwrap_or_else(|| {
        PARTS
            .iter()
            .filter(|(key, method)| {
                *method != "DOCUMENT"
                    && stored.get(*key).is_some_and(|part| !part.is_null())
            })
            .map(|(_, method)| ((*method).to_string(), Value::Bool(true)))
            .collect()
    });
    Value::Object(methods)
}

/// Reads the package's templates. Refuses a file in another layout, or two
/// templates with one alias: applying either would be a guess.
pub fn read(csv_bytes: &[u8]) -> Result<Vec<PackageTemplate>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_bytes);
    let headers = reader
        .headers()
        .map_err(|_| "the templates file can't be read".to_string())?
        .clone();
    if headers.iter().collect::<Vec<_>>() != COLUMNS {
        return Err(format!(
            "the templates file must have the columns {}",
            COLUMNS.join(",")
        ));
    }
    let mut templates = vec![];
    let mut aliases = BTreeSet::new();
    for (index, record) in reader.records().enumerate() {
        let record = record
            .map_err(|_| format!("template row {} can't be read", index + 2))?;
        let cell = |column: usize| record.get(column).unwrap_or_default();
        let alias = cell(0).trim().to_string();
        if alias.is_empty() {
            return Err(format!("template row {} has no alias", index + 2));
        }
        if !aliases.insert(alias.clone()) {
            return Err(format!("two templates have the alias '{alias}'"));
        }
        let name = match cell(1).trim() {
            "" => alias.clone(),
            name => name.to_string(),
        };
        let mut stored = stored_template(cell(4));
        if !stored.get("selected_methods").is_some_and(Value::is_object) {
            let methods = selected_methods(cell(5), &stored);
            stored.insert("selected_methods".into(), methods);
        }
        stored.insert("alias".into(), Value::String(alias.clone()));
        stored.insert("name".into(), Value::String(name));
        let communication_method = match cell(3).trim() {
            "" => PARTS
                .iter()
                .map(|(_, method)| *method)
                .find(|method| {
                    stored["selected_methods"].get(*method)
                        == Some(&Value::Bool(true))
                })
                .unwrap_or(if stored.contains_key("document") {
                    "DOCUMENT"
                } else {
                    "EMAIL"
                })
                .to_string(),
            method => method.to_uppercase(),
        };
        templates.push(PackageTemplate {
            alias,
            template_type: platform_type(cell(2).trim()),
            communication_method,
            template: Value::Object(stored),
        });
    }
    Ok(templates)
}

/// The channel and provider language of each approved template the
/// templates name for their WhatsApp, Viber and Messenger parts: the
/// languages a package's accounts on those channels send notices in.
pub fn notice_languages(
    templates: &[PackageTemplate],
) -> Vec<(MessageChannel, String)> {
    let mut languages = BTreeSet::new();
    for template in templates {
        for (key, channel) in [
            ("whatsapp", MessageChannel::WHATSAPP),
            ("viber", MessageChannel::VIBER),
            ("messenger", MessageChannel::MESSENGER),
        ] {
            let Some(part) = template.template.get(key) else {
                continue;
            };
            let named = |field: &str| {
                part.get(field)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            };
            if let (Some(_), Some(language)) =
                (named("provider_template"), named("provider_language"))
            {
                languages.insert((channel, language.to_string()));
            }
        }
    }
    languages.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn csv(rows: &[[&str; 6]]) -> Vec<u8> {
        let mut writer = csv::Writer::from_writer(vec![]);
        writer.write_record(COLUMNS).unwrap();
        for row in rows {
            writer.write_record(row).unwrap();
        }
        writer.into_inner().unwrap()
    }

    /// The notice as the Admin Portal stores it: every channel's part, the
    /// WhatsApp template's placeholders and buttons included, arrives as it
    /// was configured.
    #[test]
    fn a_stored_template_arrives_with_every_channel_part() {
        let document = json!({
            "email": {"subject":"Voting reminder","plaintext_body":"Hello","html_body":"<p>Hello</p>"},
            "sms": {"message":"Vote now: {{vote_url}}"},
            "whatsapp": {"message":"Hello {{user.first_name}}","parameters":["{{user.first_name}}","{{vote_url}}"],
                "provider_template":"step_notice","provider_language":"en",
                "buttons":[{"type":"URL","parameter":"{{vote_url}}"}]},
            "viber": {"message":"Hello {{user.first_name}}, vote at {{vote_url}}"},
            "selected_methods": {"EMAIL":true,"SMS":true,"WHATSAPP":true,"VIBER":true}
        });
        let templates = read(&csv(&[[
            "reminder",
            "Voting reminder",
            "CREDENTIALS",
            "EMAIL",
            &document.to_string(),
            "",
        ]]))
        .unwrap();
        assert_eq!(templates.len(), 1);
        let template = &templates[0];
        assert_eq!(template.alias, "reminder");
        assert_eq!(template.template_type, "CREDENTIALS");
        assert_eq!(template.communication_method, "EMAIL");
        let mut expected = document.clone();
        expected["alias"] = json!("reminder");
        expected["name"] = json!("Voting reminder");
        assert_eq!(template.template, expected);
        assert_eq!(
            notice_languages(&templates),
            [(MessageChannel::WHATSAPP, "en".to_string())]
        );
    }

    /// An email written in the Architect becomes the template's email
    /// part, listed under email, in the platform's own names.
    #[test]
    fn an_architect_email_becomes_the_email_part() {
        let templates = read(&csv(&[
            [
                "invitation",
                "",
                "voter",
                "email",
                r#"{"subject":"You're invited","body":"Vote now","body_html":"<p>Vote now</p>"}"#,
                "",
            ],
            [
                "vote-receipt",
                "Receipt",
                "VOTE_RECEIPT",
                "",
                "<h1>{{ballot_id}}</h1>",
                "",
            ],
        ]))
        .unwrap();
        assert_eq!(templates[0].communication_method, "EMAIL");
        assert_eq!(
            templates[0].template,
            json!({
                "alias":"invitation","name":"invitation",
                "email":{"subject":"You're invited","plaintext_body":"Vote now","html_body":"<p>Vote now</p>"},
                "selected_methods":{"EMAIL":true}
            })
        );
        assert_eq!(templates[1].template_type, "BALLOT_RECEIPT");
        assert_eq!(templates[1].communication_method, "DOCUMENT");
        assert_eq!(templates[1].template["document"], "<h1>{{ballot_id}}</h1>");
        assert_eq!(templates[1].template["selected_methods"], json!({}));
        assert!(notice_languages(&templates).is_empty());
    }

    #[test]
    fn a_selected_methods_cell_is_kept() {
        let templates = read(&csv(&[[
            "notice",
            "Notice",
            "CREDENTIALS",
            "",
            r#"{"sms":{"message":"Hi"},"viber":{"message":"Hi"}}"#,
            r#"{"VIBER":true}"#,
        ]]))
        .unwrap();
        assert_eq!(
            templates[0].template["selected_methods"],
            json!({"VIBER":true})
        );
        assert_eq!(templates[0].communication_method, "VIBER");
    }

    #[test]
    fn another_layout_or_a_repeated_alias_is_refused() {
        let admin_portal_export = b"alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type\nwelcome,t,{},admin,,,,,EMAIL,CREDENTIALS\n";
        assert!(read(admin_portal_export).unwrap_err().contains("columns"));
        let repeated = csv(&[
            ["a", "", "CREDENTIALS", "EMAIL", "{}", ""],
            ["a", "", "CREDENTIALS", "EMAIL", "{}", ""],
        ]);
        assert!(read(&repeated).unwrap_err().contains("'a'"));
        let unnamed = csv(&[["", "", "CREDENTIALS", "EMAIL", "{}", ""]]);
        assert!(read(&unnamed).unwrap_err().contains("no alias"));
    }
}
