// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Opaque Scanovate settings in authoring documents and imported realms.
//!
//! Credentials are supplied by an authorized export or the workbook author. This
//! module never obtains credentials, unmasks values or formats settings in errors.

use serde_json::Value;

use super::problem::{Code, Problem};

const PROVIDER: &str = "scanovate-authenticator";
const PRESET_ALIAS: &str = "scanovate-registration";
const SECRET: &str = "liveness-secret";

/// Whether text is a masked credential rather than a value that can be installed.
pub fn is_masked(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().all(|character| character == '*')
}

/// Configurations used by Scanovate, with whether they are exclusive to it.
/// Keycloak exports refer by alias; a supplied realm may instead refer by id.
fn configs(realm: &Value) -> Vec<(usize, &Value, bool)> {
    let Some(configs) =
        realm.get("authenticatorConfig").and_then(Value::as_array)
    else {
        return Vec::new();
    };
    configs
        .iter()
        .enumerate()
        .filter_map(|(index, config)| {
            let references: Vec<&str> = ["alias", "id"]
                .iter()
                .filter_map(|key| config.get(key).and_then(Value::as_str))
                .collect();
            let providers: Vec<&str> = realm
                .get("authenticationFlows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|flow| {
                    flow.get("authenticationExecutions")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter(|execution| {
                    execution
                        .get("authenticatorConfig")
                        .and_then(Value::as_str)
                        .is_some_and(|reference| {
                            references.contains(&reference)
                        })
                })
                .map(|execution| {
                    execution
                        .get("authenticator")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                })
                .collect();
            let preset = providers.is_empty()
                && config.get("alias").and_then(Value::as_str)
                    == Some(PRESET_ALIAS);
            (preset || providers.contains(&PROVIDER)).then(|| {
                (
                    index,
                    config,
                    providers.iter().all(|provider| *provider == PROVIDER),
                )
            })
        })
        .collect()
}

fn masked_problem(config: &Value) -> Option<Problem> {
    config
        .get("config")
        .and_then(|settings| settings.get(SECRET))
        .and_then(Value::as_str)
        .filter(|value| is_masked(value))
        .map(|_| {
            super::import_problems::masked_integration_secret(
                config
                    .get("alias")
                    .and_then(Value::as_str)
                    .unwrap_or("Scanovate"),
                SECRET,
            )
        })
}

/// Refuse masks and shared configurations whose settings cannot stay opaque.
pub fn validate(realm: &Value) -> Result<(), Problem> {
    for (_, config, exclusive) in configs(realm) {
        if let Some(problem) = masked_problem(config) {
            return Err(problem);
        }
        if !exclusive {
            return Err(Problem::error(
                Code::InvalidValue,
                "keycloak_event_realm",
                "a Scanovate authenticator configuration is shared with another provider; \
                 give Scanovate its own configuration before exporting or importing it",
            ).id("file.shared-scanovate-config"));
        }
    }
    Ok(())
}

/// Authoring patches may not carry executions yet. Refuse a masked liveness
/// credential there too, even before its configuration can be identified by use.
pub fn validate_authoring(realm: &Value) -> Result<(), Problem> {
    validate(realm)?;
    for config in realm
        .get("authenticatorConfig")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(problem) = masked_problem(config) {
            return Err(problem);
        }
    }
    Ok(())
}

/// Value-free notice: derivative files need the same protection as their source.
pub fn credential_notice(realm: &Value) -> Option<Problem> {
    realm.get("authenticatorConfig").and_then(Value::as_array)
        .into_iter().flatten().any(|config| {
        config.get("config").and_then(|settings| settings.get(SECRET))
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty() && !is_masked(value))
    }).then(|| Problem::warning(
        Code::InvalidValue,
        "keycloak_event_realm",
        "The workbook and derived plan/import files contain credentials supplied in the realm, \
         including the real Scanovate secret. Keep these files private and encrypted in storage and transit; share them only \
         with administrators authorized to configure this service.",
    ).id("scanovate.credentials-included"))
}

/// Restore service settings after structural IDs were rewritten in the same realm.
///
/// The entire settings map is opaque, including future provider fields. Realm,
/// config and execution IDs remain rewritten. As on authorized export, shared
/// configurations are excluded because another provider may use reference fields.
/// Callers must preserve array order; a mismatched length is refused silently.
pub fn restore_settings(original: &Value, remapped: &mut Value) {
    let Some(originals) = original
        .get("authenticatorConfig")
        .and_then(Value::as_array)
    else {
        return;
    };
    let Some(remapped) = remapped
        .get_mut("authenticatorConfig")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    if originals.len() != remapped.len() {
        return;
    }
    for (index, config, exclusive) in configs(original) {
        if !exclusive {
            continue;
        }
        if let (Some(settings), Some(target)) = (
            config.get("config").filter(|value| value.is_object()),
            remapped.get_mut(index).and_then(Value::as_object_mut),
        ) {
            target.insert("config".to_string(), settings.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn realm(reference: &str, secret: &str) -> Value {
        json!({
            "authenticationFlows": [{"authenticationExecutions": [{
                "authenticator": PROVIDER, "authenticatorConfig": reference
            }]}],
            "authenticatorConfig": [{"alias": "custom", "id": "config-id", "config": {
                "liveness-secret": secret, "future-setting": "opaque-id"
            }}]
        })
    }

    #[test]
    fn masks_are_refused_by_alias_or_id_but_real_strings_are_unchanged() {
        for reference in ["custom", "config-id"] {
            for mask in ["*", "*****", "**********", " ********** "] {
                let problem = validate(&realm(reference, mask)).unwrap_err();
                assert_eq!(
                    problem.id.as_deref(),
                    Some("file.masked-integration-secret")
                );
            }
            for value in ["", " x*y ", "000123", "true", "=synthetic", "{}"] {
                let source = realm(reference, value);
                validate(&source).unwrap();
                let mut rewritten = realm(reference, "changed");
                rewritten["authenticatorConfig"][0]["id"] = json!("new-id");
                restore_settings(&source, &mut rewritten);
                assert_eq!(
                    rewritten["authenticatorConfig"][0]["config"],
                    source["authenticatorConfig"][0]["config"]
                );
                assert_eq!(rewritten["authenticatorConfig"][0]["id"], "new-id");
            }
        }
    }

    #[test]
    fn shared_configs_are_validated_but_not_restored_as_opaque_data() {
        let mut original = realm("custom", "*****");
        original["authenticationFlows"][0]["authenticationExecutions"]
            .as_array_mut().unwrap().push(json!({
                "authenticator": "other-provider", "authenticatorConfig": "config-id"
            }));
        assert!(validate(&original).is_err());
        original["authenticatorConfig"][0]["config"][SECRET] =
            json!("synthetic-real");
        assert_eq!(
            validate(&original).unwrap_err().id.as_deref(),
            Some("file.shared-scanovate-config")
        );
        let mut remapped = realm("custom", "changed");
        restore_settings(&original, &mut remapped);
        assert_eq!(
            remapped["authenticatorConfig"][0]["config"][SECRET],
            "changed"
        );
    }

    #[test]
    fn unreferenced_preset_and_absent_or_different_configs_are_handled() {
        let patch = json!({"authenticatorConfig": [{
            "alias": PRESET_ALIAS, "config": {SECRET: "*****"}
        }]});
        assert!(validate(&patch).is_err());
        assert!(validate(&json!({})).is_ok());
        let source = realm("custom", "synthetic-real");
        let mut absent = json!({});
        restore_settings(&source, &mut absent);
        assert_eq!(absent, json!({}));
        let mut different = realm("custom", "changed");
        different["authenticatorConfig"]
            .as_array_mut()
            .unwrap()
            .push(json!({}));
        let expected = different.clone();
        restore_settings(&source, &mut different);
        assert_eq!(different, expected);
    }
}
