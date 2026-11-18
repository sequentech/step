// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Scanovate authenticator's `liveness-secret` in authorized,
//! password-encrypted election event exports.
//!
//! Keycloak's partial export writes `**********` in place of every secret
//! setting. [`embed_integration_secrets`] puts the value Keycloak stores back
//! in its place, for the Scanovate configurations only. No other secret is
//! read or exported.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The Scanovate authenticator's provider id.
pub const SCANOVATE_AUTHENTICATOR_PROVIDER: &str = "scanovate-authenticator";
/// The Scanovate setting holding the Liveness Plus callback secret.
pub const SCANOVATE_LIVENESS_SECRET_KEY: &str = "liveness-secret";
/// The Scanovate setting naming the Liveness Plus endpoint.
pub const SCANOVATE_LIVENESS_URL_KEY: &str = "liveness-url";
/// What Keycloak's partial export writes in place of a secret setting.
#[cfg(test)]
const KEYCLOAK_MASKED_SECRET: &str = "**********";

/// Whether an election event export carries the Scanovate `liveness-secret`.
/// Only the server decides it.
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IntegrationSecretsExport {
    /// The realm travels as Keycloak exports it: masked, secret-free.
    #[default]
    OMITTED,
    INCLUDED,
}

/// A credential value. Neither `Debug` nor `Display` shows it, and it has no
/// `Serialize`: it only reaches an archive through
/// [`embed_integration_secrets`].
#[derive(Clone, PartialEq, Eq)]
pub struct IntegrationSecretValue(String);

impl IntegrationSecretValue {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Debug for IntegrationSecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IntegrationSecretValue(<redacted>)")
    }
}

/// A Scanovate configuration's secret as Keycloak stores it, read together
/// with the configuration's id and endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredIntegrationSecret {
    pub config_id: String,
    pub config_alias: String,
    pub value: IntegrationSecretValue,
    pub endpoint: Option<String>,
}

/// Why an export with integration secrets stopped. Names the configuration,
/// never a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationSecretExportError {
    /// The configuration changed between the realm export and the read of
    /// its secret; exporting again reads both anew.
    ConfigurationChanged { config_alias: String },
    /// No single usable secret with an endpoint, or the configuration is
    /// shared with another authenticator: the archive would not carry a
    /// working credential.
    Unavailable { config_alias: String },
}

impl fmt::Display for IntegrationSecretExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigurationChanged { config_alias } => write!(
                formatter,
                "Scanovate authenticator configuration '{config_alias}' changed \
                 while the event was exported; export it again"
            ),
            Self::Unavailable { config_alias } => write!(
                formatter,
                "Scanovate authenticator configuration '{config_alias}' has no \
                 exportable {SCANOVATE_LIVENESS_SECRET_KEY}; configure it or \
                 export without integration secrets"
            ),
        }
    }
}

impl std::error::Error for IntegrationSecretExportError {}

/// The provider ids of the executions using each authenticator configuration.
fn config_providers(realm: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut providers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let flows = realm.get("authenticationFlows").and_then(Value::as_array);
    for flow in flows.into_iter().flatten() {
        let executions = flow
            .get("authenticationExecutions")
            .and_then(Value::as_array);
        for execution in executions.into_iter().flatten() {
            if let Some(alias) = execution.get("authenticatorConfig").and_then(Value::as_str) {
                let provider = execution
                    .get("authenticator")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                providers
                    .entry(alias.to_string())
                    .or_default()
                    .insert(provider.to_string());
            }
        }
    }
    providers
}

/// Each authenticator configuration a Scanovate execution uses: its alias,
/// id, whether only Scanovate executions use it, and its settings.
fn scanovate_configs(
    realm: &mut Value,
) -> Vec<(String, Option<String>, bool, &mut Map<String, Value>)> {
    let providers = config_providers(realm);
    let Some(configs) = realm
        .get_mut("authenticatorConfig")
        .and_then(Value::as_array_mut)
    else {
        return vec![];
    };
    configs
        .iter_mut()
        .filter_map(|config| {
            let object = config.as_object_mut()?;
            let alias = object.get("alias")?.as_str()?.to_string();
            let used_by = providers.get(&alias)?;
            if !used_by.contains(SCANOVATE_AUTHENTICATOR_PROVIDER) {
                return None;
            }
            let exclusive = used_by.len() == 1;
            let id = object.get("id").and_then(Value::as_str).map(String::from);
            let settings = object.get_mut("config")?.as_object_mut()?;
            Some((alias, id, exclusive, settings))
        })
        .collect()
}

/// Puts the `liveness-secret` Keycloak stores into each Scanovate
/// configuration of the exported realm, in place of its mask.
///
/// `stored` comes from one read that returns, with each secret, its
/// configuration's id and endpoint; when those differ from the realm export
/// the export fails rather than pairing a secret with another endpoint. A
/// configuration without a single usable secret and endpoint, or shared
/// with another authenticator, fails it too, so a requested secret is never
/// silently left out.
pub fn embed_integration_secrets(
    realm: &mut Value,
    stored: &[StoredIntegrationSecret],
) -> Result<(), IntegrationSecretExportError> {
    for (alias, id, exclusive, settings) in scanovate_configs(realm) {
        let unavailable = IntegrationSecretExportError::Unavailable {
            config_alias: alias.clone(),
        };
        // One row per execution using the configuration.
        let mut rows = stored.iter().filter(|row| row.config_alias == alias);
        let Some(row) = rows.next() else {
            return Err(unavailable);
        };
        let secret = &row.value.0;
        if !exclusive
            || rows.any(|other| other != row)
            || secret.trim().is_empty()
            || sequent_core::election_config::scanovate::is_masked(secret)
        {
            return Err(unavailable);
        }
        let endpoint = settings
            .get(SCANOVATE_LIVENESS_URL_KEY)
            .and_then(Value::as_str);
        if id.is_some_and(|id| id != row.config_id) || endpoint != row.endpoint.as_deref() {
            return Err(IntegrationSecretExportError::ConfigurationChanged {
                config_alias: alias,
            });
        }
        if endpoint.is_none_or(|endpoint| endpoint.trim().is_empty()) {
            return Err(unavailable);
        }
        settings.insert(
            SCANOVATE_LIVENESS_SECRET_KEY.to_string(),
            Value::String(secret.clone()),
        );
    }
    Ok(())
}

/// The aliases of the configurations a Scanovate execution uses whose
/// `liveness-secret` is Keycloak's export placeholder, `**********`, rather
/// than a secret.
///
/// Keycloak stores an imported realm's authenticator settings as given, so
/// importing such a configuration would install the placeholder as the
/// credential: Liveness Plus callbacks would then fail while Keycloak's
/// console, which masks the setting either way, looks the same. An import
/// refuses these instead (see `reject_masked_integration_secrets`). Only the
/// nonempty all-asterisk placeholders count; no setting is printed.
pub fn masked_integration_secret_configs(realm: &Value) -> Vec<String> {
    let providers = config_providers(realm);
    let configs = realm.get("authenticatorConfig").and_then(Value::as_array);
    configs
        .into_iter()
        .flatten()
        .filter_map(|config| {
            let alias = config.get("alias")?.as_str()?;
            if !providers
                .get(alias)?
                .contains(SCANOVATE_AUTHENTICATOR_PROVIDER)
            {
                return None;
            }
            let secret = config.get("config")?.get(SCANOVATE_LIVENESS_SECRET_KEY)?;
            secret
                .as_str()
                .is_some_and(sequent_core::election_config::scanovate::is_masked)
                .then(|| alias.to_string())
        })
        .collect()
}

/// Restore opaque Scanovate settings after import rewrites structural UUIDs.
/// Service settings, including unknown future keys, are not platform references.
/// Shared configurations remain excluded, as on authorized secret export.
pub fn restore_remapped_integration_secrets(
    original: Option<&Value>,
    remapped: Option<&mut Value>,
) {
    if let (Some(original), Some(remapped)) = (original, remapped) {
        sequent_core::election_config::scanovate::restore_settings(original, remapped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Synthetic values only.
    const SECRET: &str = "synthetic-liveness-secret-6f1d";
    const ENDPOINT: &str = "https://source.example.test/biometric";
    const ALIAS: &str = "scanovate-registration";
    const ID: &str = "3b3c7e73-0000-4000-8000-000000000001";

    fn realm() -> Value {
        json!({
            "authenticationFlows": [{"authenticationExecutions": [
                {"authenticator": SCANOVATE_AUTHENTICATOR_PROVIDER,
                 "authenticatorConfig": ALIAS},
                {"authenticator": "conditional-user-attribute",
                 "authenticatorConfig": "other"}
            ]}],
            "authenticatorConfig": [
                {"id": ID, "alias": ALIAS, "config": {
                    "liveness-url": ENDPOINT, "liveness-secret": KEYCLOAK_MASKED_SECRET,
                    "max-attempts": "3"}},
                {"alias": "other", "config": {
                    "liveness-secret": KEYCLOAK_MASKED_SECRET}}
            ]
        })
    }

    fn stored(value: &str) -> StoredIntegrationSecret {
        StoredIntegrationSecret {
            config_id: ID.to_string(),
            config_alias: ALIAS.to_string(),
            value: IntegrationSecretValue::new(value.to_string()),
            endpoint: Some(ENDPOINT.to_string()),
        }
    }

    fn setting<'a>(realm: &'a Value, index: usize, key: &str) -> Option<&'a Value> {
        realm["authenticatorConfig"][index]["config"].get(key)
    }

    /// The archive carries the real value, and the import's own typed read
    /// of the realm (`deserialize_value` into `RealmRepresentation`, as
    /// `process_election_event_file` does) yields the same credential.
    #[test]
    fn the_stored_secret_replaces_the_mask_and_survives_the_import_read() {
        use keycloak::types::RealmRepresentation;
        use sequent_core::serialization::deserialize_with_path::deserialize_value;

        let mut exported = realm();
        // Two executions using the configuration read as two equal rows.
        embed_integration_secrets(&mut exported, &[stored(SECRET), stored(SECRET)]).unwrap();
        let archived: Value =
            serde_json::from_slice(&serde_json::to_vec(&exported).unwrap()).unwrap();
        let imported: RealmRepresentation = deserialize_value(archived).unwrap();
        let configs = imported.authenticator_config.unwrap();
        let config = |alias: &str| {
            configs
                .iter()
                .find(|config| config.alias.as_deref() == Some(alias))
                .and_then(|config| config.config.clone())
                .unwrap()
        };
        let scanovate = config(ALIAS);
        assert_eq!(
            scanovate.get("liveness-secret").map(String::as_str),
            Some(SECRET)
        );
        assert_eq!(
            scanovate.get("liveness-url").map(String::as_str),
            Some(ENDPOINT)
        );
        assert_eq!(scanovate.get("max-attempts").map(String::as_str), Some("3"));
        // Only Scanovate's secret travels.
        assert_eq!(
            config("other").get("liveness-secret").map(String::as_str),
            Some(KEYCLOAK_MASKED_SECRET)
        );
    }

    #[test]
    fn a_secret_stored_for_another_endpoint_or_configuration_is_refused() {
        let mut moved = stored(SECRET);
        moved.endpoint = Some("https://elsewhere.example.test".to_string());
        let mut recreated = stored(SECRET);
        recreated.config_id = "3b3c7e73-0000-4000-8000-000000000002".to_string();
        for row in [moved, recreated] {
            let mut exported = realm();
            let error = embed_integration_secrets(&mut exported, &[row]).unwrap_err();
            assert_eq!(
                error,
                IntegrationSecretExportError::ConfigurationChanged {
                    config_alias: ALIAS.to_string()
                }
            );
            assert!(!error.to_string().contains(SECRET));
            assert_eq!(
                setting(&exported, 0, "liveness-secret"),
                Some(&json!(KEYCLOAK_MASKED_SECRET))
            );
        }
    }

    #[test]
    fn an_unavailable_secret_is_refused_rather_than_left_out() {
        let mut other_alias = stored(SECRET);
        other_alias.config_alias = "another".to_string();
        let mut no_endpoint = realm();
        no_endpoint["authenticatorConfig"][0]["config"]
            .as_object_mut()
            .unwrap()
            .remove("liveness-url");
        let mut no_endpoint_row = stored(SECRET);
        no_endpoint_row.endpoint = None;
        let mut shared = realm();
        shared["authenticationFlows"][0]["authenticationExecutions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"authenticator": "attacker-authenticator",
                         "authenticatorConfig": ALIAS}));
        for (mut exported, rows) in [
            (realm(), vec![]),
            (realm(), vec![other_alias]),
            (realm(), vec![stored(KEYCLOAK_MASKED_SECRET)]),
            (realm(), vec![stored("*****")]),
            (realm(), vec![stored("  **********  ")]),
            (realm(), vec![stored(" ")]),
            (
                realm(),
                vec![stored(SECRET), stored("synthetic-conflicting")],
            ),
            (no_endpoint, vec![no_endpoint_row]),
            (shared, vec![stored(SECRET)]),
        ] {
            assert_eq!(
                embed_integration_secrets(&mut exported, &rows),
                Err(IntegrationSecretExportError::Unavailable {
                    config_alias: ALIAS.to_string()
                }),
                "{rows:?}"
            );
        }
    }

    #[test]
    fn diagnostics_never_show_the_secret() {
        let row = stored(SECRET);
        assert!(!format!("{row:?}").contains(SECRET));
        assert!(!format!("{:#?}", row.value).contains(SECRET));
    }

    #[test]
    fn restoring_secrets_needs_the_same_configurations() {
        let flows = json!([{"authenticationExecutions": [
            {"authenticator": SCANOVATE_AUTHENTICATOR_PROVIDER, "authenticatorConfig": ALIAS},
            {"authenticator": "conditional-user-attribute", "authenticatorConfig": "other"}]}]);
        let original = json!({"authenticationFlows": flows, "authenticatorConfig": [
            {"alias": ALIAS, "config": {"liveness-secret": SECRET, "liveness-url": ENDPOINT,
                        "max-attempts": "original"}},
            {"alias": "other", "config": {"liveness-secret": SECRET}}]});
        let remapped_secret = json!({"authenticationFlows": flows, "authenticatorConfig": [
            {"alias": ALIAS, "config": {"liveness-secret": "remapped", "liveness-url": "remapped",
                        "max-attempts": "remapped"}},
            {"alias": "other", "config": {"liveness-secret": "remapped"}}]});

        let mut restored = remapped_secret.clone();
        restore_remapped_integration_secrets(Some(&original), Some(&mut restored));
        let settings = &restored["authenticatorConfig"][0]["config"];
        assert_eq!(settings["liveness-secret"], SECRET);
        assert_eq!(settings["liveness-url"], ENDPOINT);
        // Service settings are opaque; other authenticators keep remapping.
        assert_eq!(settings["max-attempts"], "original");
        assert_eq!(
            restored["authenticatorConfig"][1]["config"]["liveness-secret"],
            "remapped"
        );

        // Configurations out of step, or no realm: nothing is restored.
        let mut extra = json!({"authenticationFlows": flows, "authenticatorConfig": [
            {"alias": ALIAS, "config": {"liveness-secret": "remapped"}},
            {"config": {}}, {"config": {}}]});
        restore_remapped_integration_secrets(Some(&original), Some(&mut extra));
        assert_eq!(
            extra["authenticatorConfig"][0]["config"]["liveness-secret"],
            "remapped"
        );
        let mut untouched = remapped_secret.clone();
        restore_remapped_integration_secrets(None, Some(&mut untouched));
        assert_eq!(untouched, remapped_secret);
        restore_remapped_integration_secrets(Some(&original), None);
    }

    #[test]
    fn only_a_scanovate_placeholder_secret_is_reported() {
        let configs = |secret: &str, other: &str| {
            json!({
                "authenticationFlows": [{"authenticationExecutions": [
                    {"authenticator": SCANOVATE_AUTHENTICATOR_PROVIDER,
                     "authenticatorConfig": ALIAS},
                    {"authenticator": "conditional-user-attribute",
                     "authenticatorConfig": "other"}
                ]}],
                "authenticatorConfig": [
                    {"alias": ALIAS, "config": {
                        "liveness-url": ENDPOINT, "liveness-secret": secret}},
                    {"alias": "other", "config": {"liveness-secret": other}},
                    // Unused by any execution: not installed as a Scanovate secret.
                    {"alias": "unused", "config": {
                        "liveness-secret": KEYCLOAK_MASKED_SECRET}}
                ]
            })
        };
        assert_eq!(
            masked_integration_secret_configs(&configs(KEYCLOAK_MASKED_SECRET, SECRET)),
            vec![ALIAS.to_string()]
        );
        // A real secret, or a placeholder in another authenticator's settings,
        // is not one.
        assert!(
            masked_integration_secret_configs(&configs(SECRET, KEYCLOAK_MASKED_SECRET)).is_empty()
        );
        assert!(masked_integration_secret_configs(&json!({})).is_empty());
        // A configuration Scanovate shares with another authenticator still
        // installs the placeholder as Scanovate's secret: reported too.
        let mut shared = configs(KEYCLOAK_MASKED_SECRET, SECRET);
        shared["authenticationFlows"][0]["authenticationExecutions"][1]["authenticatorConfig"] =
            json!(ALIAS);
        assert_eq!(
            masked_integration_secret_configs(&shared),
            vec![ALIAS.to_string()]
        );
    }
}
