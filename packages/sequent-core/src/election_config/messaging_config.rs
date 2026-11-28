// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Event messaging references carried by Architect's existing Parameters sheet.

use std::collections::BTreeMap;

use serde_json::Value;

use super::problem::{Code, Problem};
use super::sheet::Row;
use crate::types::messaging::{EventMessagingConfig, MessagingConfigError};

pub(super) const PARAMETER: &str = "messaging_config";

/// A workbook's explicit configuration wins over the base, including null.
/// Blank values have the same fallback semantics as other Parameters rows.
pub(super) fn from_rows(
    rows: &[Row],
    base: &Value,
    elections: &[(String, String)],
) -> Result<Option<Value>, Problem> {
    let mut matching = rows.iter().filter(|row| {
        row.text("key").map(str::trim) == Some(PARAMETER)
            && row.get("value").is_some()
    });
    let row = matching.next();
    if matching.next().is_some() {
        return Err(Problem::error(
            Code::DuplicateId,
            PARAMETER,
            "keep one messaging_config Parameters row",
        ));
    }
    let selected = row
        .and_then(|row| row.get("value"))
        .or_else(|| base.get(PARAMETER));
    selected
        .map(|value| {
            normalize(value, base, elections).map_err(|problem| match row {
                Some(row) => problem.at(&row.origin(Some("value"))),
                None => problem,
            })
        })
        .transpose()
}

/// The wizard validates the same carried Parameters rows before compiling.
pub(super) fn from_plan(
    plan: &super::architect::Blueprint,
) -> Result<Option<Value>, Problem> {
    let rows = plan
        .platform
        .iter()
        .filter(|sheet| sheet.key == super::sheet::SHEET_PARAMETERS)
        .flat_map(|sheet| sheet.rows.iter().cloned())
        .collect::<Vec<_>>();
    let ids = super::ids::IdFactory::new(&plan.external_id);
    let elections = plan
        .elections
        .iter()
        .map(|election| {
            (
                election.external_id.clone(),
                ids.as_ref()
                    .map(|ids| ids.uid("election", &[&election.external_id]))
                    .unwrap_or_else(|| election.external_id.clone()),
            )
        })
        .collect::<Vec<_>>();
    from_rows(
        &rows,
        plan.base_export.as_ref().unwrap_or(&Value::Null),
        &elections,
    )
}

/// Resolve workbook external IDs, already-built UUIDs, and UUIDs from a base
/// export to the current election IDs. Account IDs remain tenant references.
pub(super) fn normalize(
    value: &Value,
    base: &Value,
    elections: &[(String, String)],
) -> Result<Value, Problem> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let mut config: EventMessagingConfig =
        serde_json::from_value(value.clone()).map_err(|_| {
            // Deserialization errors may quote arbitrary input, including an
            // accidentally pasted credential. Only describe the expected shape.
            Problem::error(
                Code::InvalidValue,
                PARAMETER,
                "messaging_config must be an EventMessagingConfig JSON object \
                 with a version, valid channels, purposes and election_channels",
            )
        })?;
    let mut remapped = BTreeMap::new();
    for (reference, channels) in &config.election_channels {
        let old_external = base
            .get("elections")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|election| {
                election.get("id").and_then(Value::as_str)
                    == Some(reference.as_str())
            })
            .and_then(|election| election.get("external_id"))
            .and_then(Value::as_str);
        let mut targets = elections
            .iter()
            .filter(|(external, id)| {
                reference == external
                    || reference == id
                    || old_external == Some(external.as_str())
            })
            .map(|(_, id)| id);
        let Some(target) = targets.next() else {
            return Err(Problem::error(
                Code::DanglingReference,
                "messaging_config.election_channels",
                format!("messaging configuration refers to an unknown election: {reference}"),
            ));
        };
        if targets.any(|id| id != target)
            || remapped.insert(target.clone(), channels.clone()).is_some()
        {
            return Err(Problem::error(
                Code::ConflictingColumns,
                "messaging_config.election_channels",
                "more than one messaging restriction resolves to the same election, \
                 or an election reference is ambiguous; use one external ID per election",
            ));
        }
    }
    config.election_channels = remapped;
    if config
        .channels
        .iter()
        .any(|channel| channel.account_id.trim().is_empty())
    {
        return Err(Problem::error(
            Code::MissingField,
            "messaging_config.channels.account_id",
            "each messaging channel must reference an existing tenant account",
        ));
    }
    // Reuse the platform's checks without pretending to know tenant accounts.
    // UNKNOWN_ACCOUNT is the expected result of an offline build; the importer
    // still checks existence, ownership, channel compatibility and readiness.
    let ids = elections
        .iter()
        .map(|(_, id)| id.clone())
        .collect::<Vec<_>>();
    if let Err(errors) = config.validate("", &[], &ids) {
        if let Some(error) = errors.into_iter().find(|error| {
            !matches!(error, MessagingConfigError::UNKNOWN_ACCOUNT { .. })
        }) {
            return Err(Problem::error(
                Code::InvalidValue,
                PARAMETER,
                format!("invalid messaging configuration: {error:?}"),
            ));
        }
    }
    // Serialize only the platform's reference-only type, never provider account
    // definitions, arbitrary extra fields or credentials from input JSON.
    serde_json::to_value(config).map_err(|_| {
        Problem::error(
            Code::InvalidValue,
            PARAMETER,
            "cannot serialize messaging configuration",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn template_bindings_travel_with_their_buttons_and_codes() {
        // What a WhatsApp code or notice needs to be sent as configured: the
        // template, the provider's language code, the message key and the
        // buttons in order. Losing any of them sends something else.
        let config = json!({"version":1,"channels":[{
        "channel":"WHATSAPP","account_id":"package:whatsapp","purposes":["OTP","NOTICE"],
        "templates":[
            {"purpose":"OTP","language":"en","provider_template":"otp_code",
             "provider_language":"en_US",
             "buttons":[{"type":"URL","parameter":"{{code}}"},
                        {"type":"QUICK_REPLY","parameter":"NA"}]},
            {"purpose":"NOTICE","key":"vote_invitation","language":"es",
             "provider_template":"invitacion","buttons":[]}
        ]}]});
        let elections = [("e1".to_string(), "e1".to_string())];
        let normalized = normalize(&config, &Value::Null, &elections).unwrap();
        let channel = &normalized["channels"][0];
        let typed =
            |value: &Value| -> Vec<crate::types::messaging::TemplateBinding> {
                serde_json::from_value(value.clone()).unwrap()
            };
        assert_eq!(
            typed(&channel["templates"]),
            typed(&config["channels"][0]["templates"])
        );
        // An empty list (send no buttons) stays distinct from none given
        // (the provider's default buttons).
        assert_eq!(channel["templates"][1]["buttons"], json!([]));
        assert_eq!(
            channel["templates"][0]["buttons"][0]["parameter"],
            "{{code}}"
        );
        assert_eq!(channel["account_id"], "package:whatsapp");
    }
}
