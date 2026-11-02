// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The authenticator integration secrets an election event archive can carry.
//!
//! Keycloak's partial export writes [`KEYCLOAK_MASKED_SECRET`] in place of
//! every secret setting, so an archive built from it alone carries a
//! placeholder that, once imported, silently breaks the integration. Only one
//! credential class travels for real: the Scanovate authenticator's
//! `liveness-secret`, read from Keycloak's own storage by an authorized,
//! password-encrypted export. Each embedded value is recorded with a
//! [`IntegrationSecretBinding`] (no value, only where it belongs) so the
//! import keeps it only with the configuration, provider and endpoint it was
//! exported with. Nothing here ever takes a secret from the destination.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// What Keycloak's partial export writes in place of each secret setting.
pub const KEYCLOAK_MASKED_SECRET: &str = "**********";
/// The Scanovate authenticator's provider id.
pub const SCANOVATE_AUTHENTICATOR_PROVIDER: &str = "scanovate-authenticator";
/// The Scanovate setting holding the Liveness Plus callback secret.
pub const SCANOVATE_LIVENESS_SECRET_KEY: &str = "liveness-secret";
/// The Scanovate setting naming the Liveness Plus endpoint the secret pairs with.
pub const SCANOVATE_LIVENESS_URL_KEY: &str = "liveness-url";

const AUTHENTICATION_FLOWS: &str = "authenticationFlows";
const AUTHENTICATION_EXECUTIONS: &str = "authenticationExecutions";
const AUTHENTICATOR: &str = "authenticator";
const AUTHENTICATOR_CONFIG: &str = "authenticatorConfig";
const ALIAS: &str = "alias";
const CONFIG: &str = "config";

/// Whether an election event export carries the designated integration
/// secrets (the Scanovate `liveness-secret`). Only the server decides it.
#[allow(non_camel_case_types)]
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq,
)]
pub enum IntegrationSecretsExport {
    /// The realm travels as Keycloak exports it: masked, secret-free.
    #[default]
    OMITTED,
    /// The designated secrets travel, with their bindings.
    INCLUDED,
}

/// A credential value. Neither `Debug` nor `Display` shows it, and it has no
/// `Serialize`: it only reaches an archive through [`embed_integration_secrets`].
#[derive(Clone, PartialEq, Eq)]
pub struct IntegrationSecretValue(String);

impl IntegrationSecretValue {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    fn expose(&self) -> &str {
        &self.0
    }

    fn is_usable(&self) -> bool {
        is_usable_secret(&self.0)
    }
}

impl fmt::Debug for IntegrationSecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IntegrationSecretValue(<redacted>)")
    }
}

/// A designated secret as Keycloak stores it for one authenticator
/// configuration of the exported realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredIntegrationSecret {
    /// Keycloak's id of the authenticator configuration.
    pub config_id: String,
    pub config_alias: String,
    pub provider_id: String,
    pub secret_key: String,
    pub value: IntegrationSecretValue,
    /// The configuration's `liveness-url`, read with the secret.
    pub endpoint: Option<String>,
}

/// Where an exported secret belongs. Holds no secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationSecretBinding {
    /// The authenticator configuration's alias.
    pub config_alias: String,
    /// The provider of every execution that uses that configuration.
    pub provider_id: String,
    /// The setting holding the secret.
    pub secret_key: String,
    /// The setting naming the endpoint the secret pairs with.
    pub endpoint_key: String,
    /// That endpoint, as exported.
    pub endpoint: String,
}

/// Why an imported designated secret was left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationSecretOmission {
    /// The archive holds Keycloak's mask or an empty value.
    Masked,
    /// The configuration is also used by another authenticator provider.
    ProviderMismatch,
    /// The archive binds the secret to another endpoint or provider.
    BindingMismatch,
    /// A secret-bearing archive carries the value without a binding.
    Unbound,
    /// The importing user may not install integration credentials.
    NotAuthorized,
    /// The archive binds a secret that its realm no longer carries.
    Missing,
}

/// What an import did with one designated secret. Its `Display` is the
/// task-log note; it names the configuration and setting, never the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportedIntegrationSecret {
    /// Kept with the configuration, provider and endpoint it was exported with.
    KeptBound { config_alias: String },
    /// Kept as the archive's author wrote it: a bundle without bindings, such
    /// as a hand-authored or template-generated one.
    KeptAsAuthored { config_alias: String },
    /// Left out: the setting has to be configured again.
    Omitted {
        config_alias: String,
        reason: IntegrationSecretOmission,
    },
}

impl fmt::Display for ImportedIntegrationSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeptBound { config_alias } => write!(
                formatter,
                "Scanovate authenticator configuration '{config_alias}': \
                 {SCANOVATE_LIVENESS_SECRET_KEY} imported with the endpoint \
                 it was exported with; check that this deployment's Liveness \
                 Plus service uses the same secret"
            ),
            Self::KeptAsAuthored { config_alias } => write!(
                formatter,
                "Scanovate authenticator configuration '{config_alias}': \
                 {SCANOVATE_LIVENESS_SECRET_KEY} imported as provided by the \
                 archive"
            ),
            Self::Omitted {
                config_alias,
                reason,
            } => {
                let why = match reason {
                    IntegrationSecretOmission::Masked => {
                        "the archive holds a masked placeholder, not the secret"
                    }
                    IntegrationSecretOmission::ProviderMismatch => {
                        "the configuration is also used by another authenticator"
                    }
                    IntegrationSecretOmission::BindingMismatch => {
                        "the archive bound it to another endpoint or provider"
                    }
                    IntegrationSecretOmission::Unbound => {
                        "the archive carries it without a binding"
                    }
                    IntegrationSecretOmission::Missing => {
                        "the archive's realm no longer carries it"
                    }
                    IntegrationSecretOmission::NotAuthorized => {
                        "importing integration secrets requires the \
                         integration-secret-write permission"
                    }
                };
                write!(
                    formatter,
                    "Scanovate authenticator configuration '{config_alias}': \
                     {SCANOVATE_LIVENESS_SECRET_KEY} not imported because {why}; \
                     set it again in the event realm, or re-export the event \
                     with integration secrets"
                )
            }
        }
    }
}

/// Whether a value can be a credential: not empty and not Keycloak's mask.
pub fn is_usable_secret(value: &str) -> bool {
    !value.trim().is_empty() && value != KEYCLOAK_MASKED_SECRET
}

/// The provider ids of the executions using each authenticator configuration.
fn config_providers(realm: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut providers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let flows = realm.get(AUTHENTICATION_FLOWS).and_then(Value::as_array);
    for flow in flows.into_iter().flatten() {
        let executions = flow
            .get(AUTHENTICATION_EXECUTIONS)
            .and_then(Value::as_array);
        for execution in executions.into_iter().flatten() {
            let Some(alias) =
                execution.get(AUTHENTICATOR_CONFIG).and_then(Value::as_str)
            else {
                continue;
            };
            // An execution without a provider id is not the designated one.
            let provider = execution
                .get(AUTHENTICATOR)
                .and_then(Value::as_str)
                .unwrap_or_default();
            providers
                .entry(alias.to_string())
                .or_default()
                .insert(provider.to_string());
        }
    }
    providers
}

/// Whether only Scanovate executions use the configuration.
fn is_exclusively_scanovate(providers: Option<&BTreeSet<String>>) -> bool {
    providers.is_some_and(|providers| {
        providers.len() == 1
            && providers.contains(SCANOVATE_AUTHENTICATOR_PROVIDER)
    })
}

/// The realm's authenticator configurations: alias, id and settings.
fn configs_mut(
    realm: &mut Value,
) -> Vec<(String, Option<String>, &mut Map<String, Value>)> {
    let Some(configs) = realm
        .get_mut(AUTHENTICATOR_CONFIG)
        .and_then(Value::as_array_mut)
    else {
        return vec![];
    };
    configs
        .iter_mut()
        .filter_map(|config| {
            let object = config.as_object_mut()?;
            let alias = object.get(ALIAS)?.as_str()?.to_string();
            let id =
                object.get("id").and_then(Value::as_str).map(str::to_string);
            let settings = object.get_mut(CONFIG)?.as_object_mut()?;
            Some((alias, id, settings))
        })
        .collect()
}

fn endpoint_of(settings: &Map<String, Value>) -> Option<&str> {
    settings
        .get(SCANOVATE_LIVENESS_URL_KEY)
        .and_then(Value::as_str)
        .filter(|endpoint| !endpoint.trim().is_empty())
}

/// Why an export with integration secrets stopped. Names the configuration,
/// never a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationSecretExportError {
    /// The stored configuration is not the one the realm export read: it
    /// changed in between. Exporting again reads both anew.
    ConfigurationChanged { config_alias: String },
    /// The configuration has no single usable stored secret with an endpoint,
    /// or another authenticator also uses it: the archive could not carry a
    /// working credential, so it is not written.
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
                 exportable {SCANOVATE_LIVENESS_SECRET_KEY} (missing, masked, \
                 ambiguous, without {SCANOVATE_LIVENESS_URL_KEY}, or shared \
                 with another authenticator); configure it or export without \
                 integration secrets"
            ),
        }
    }
}

impl std::error::Error for IntegrationSecretExportError {}

/// Puts each designated secret Keycloak stores into the exported realm in
/// place of its mask, and returns where each one belongs.
///
/// `stored` comes from one read of Keycloak's storage that returns, with each
/// secret, its configuration's id and endpoint. When those differ from the
/// realm export the configuration changed in between, and the export fails
/// rather than pairing a secret with an endpoint it was not stored with.
///
/// Every configuration a Scanovate execution uses must come out with a
/// working credential: one without a single usable stored secret, without an
/// endpoint, or shared with another provider fails the export, so an archive
/// requested with integration secrets never silently lacks one. Other
/// configurations are untouched.
pub fn embed_integration_secrets(
    realm: &mut Value,
    stored: &[StoredIntegrationSecret],
) -> Result<Vec<IntegrationSecretBinding>, IntegrationSecretExportError> {
    let providers = config_providers(realm);
    let mut bindings = vec![];
    for (alias, id, settings) in configs_mut(realm) {
        let Some(alias_providers) = providers.get(&alias) else {
            continue;
        };
        if !alias_providers.contains(SCANOVATE_AUTHENTICATOR_PROVIDER) {
            continue;
        }
        let unavailable = || IntegrationSecretExportError::Unavailable {
            config_alias: alias.clone(),
        };
        if !is_exclusively_scanovate(Some(alias_providers)) {
            return Err(unavailable());
        }
        let candidates: Vec<&StoredIntegrationSecret> = stored
            .iter()
            .filter(|secret| {
                secret.config_alias == alias
                    && secret.provider_id == SCANOVATE_AUTHENTICATOR_PROVIDER
                    && secret.secret_key == SCANOVATE_LIVENESS_SECRET_KEY
            })
            .collect();
        // One row per execution using the configuration: identical rows are
        // the same secret, differing ones are ambiguous.
        let Some(first) = candidates.first() else {
            return Err(unavailable());
        };
        let consistent = candidates.iter().all(|secret| {
            secret.value == first.value
                && secret.config_id == first.config_id
                && secret.endpoint == first.endpoint
        });
        if !consistent || !first.value.is_usable() {
            return Err(unavailable());
        }
        let exported_endpoint = endpoint_of(settings).map(str::to_string);
        let stored_endpoint = first
            .endpoint
            .as_deref()
            .filter(|endpoint| !endpoint.trim().is_empty());
        if id.as_deref().is_some_and(|id| id != first.config_id)
            || exported_endpoint.as_deref() != stored_endpoint
        {
            return Err(IntegrationSecretExportError::ConfigurationChanged {
                config_alias: alias,
            });
        }
        let Some(endpoint) = exported_endpoint else {
            return Err(unavailable());
        };
        settings.insert(
            SCANOVATE_LIVENESS_SECRET_KEY.to_string(),
            Value::String(first.value.expose().to_string()),
        );
        bindings.push(IntegrationSecretBinding {
            config_alias: alias,
            provider_id: SCANOVATE_AUTHENTICATOR_PROVIDER.to_string(),
            secret_key: SCANOVATE_LIVENESS_SECRET_KEY.to_string(),
            endpoint_key: SCANOVATE_LIVENESS_URL_KEY.to_string(),
            endpoint,
        });
    }
    Ok(bindings)
}

/// Whether the importing user may install integration credentials
/// (`integration-secret-write`). Stamped from the caller's token.
#[allow(non_camel_case_types)]
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq,
)]
pub enum IntegrationSecretImportPermission {
    #[default]
    NOT_PERMITTED,
    PERMITTED,
}

/// Decides, for each designated secret of an imported realm, whether it is
/// kept, and removes it when it is not.
///
/// `bindings` is `None` for an archive without integration-secret bindings
/// (ordinary or older exports, hand-authored or template-generated bundles)
/// and the bindings of a secret-bearing export otherwise. The bindings are
/// not a tamper-proof manifest: whoever can edit the archive can edit them and
/// the secret alike. They keep an authorized export's secret with the
/// configuration, provider and endpoint it was exported with. Installing any
/// credential the archive provides, bound or not, is a decision of a user
/// holding `integration-secret-write`; without it every designated secret is
/// removed. The destination's own secrets are never used.
pub fn reconcile_imported_integration_secrets(
    realm: &mut Value,
    bindings: Option<&[IntegrationSecretBinding]>,
    permission: IntegrationSecretImportPermission,
) -> Vec<ImportedIntegrationSecret> {
    let providers = config_providers(realm);
    let mut outcomes = vec![];
    for (alias, _id, settings) in configs_mut(realm) {
        let alias_providers = providers.get(&alias);
        let bound: Vec<&IntegrationSecretBinding> = bindings
            .into_iter()
            .flatten()
            .filter(|binding| binding.config_alias == alias)
            .collect();
        // A usable value under the Scanovate key anywhere is in scope too: a
        // configuration no Scanovate execution uses now (orphan, or another
        // provider's) could be wired to one later.
        let carries_usable_secret = settings
            .get(SCANOVATE_LIVENESS_SECRET_KEY)
            .and_then(Value::as_str)
            .is_some_and(is_usable_secret);
        let designated = alias_providers.is_some_and(|providers| {
            providers.contains(SCANOVATE_AUTHENTICATOR_PROVIDER)
        }) || !bound.is_empty()
            || carries_usable_secret;
        if !designated {
            continue;
        }
        let Some(value) = settings.get(SCANOVATE_LIVENESS_SECRET_KEY) else {
            if !bound.is_empty() {
                outcomes.push(ImportedIntegrationSecret::Omitted {
                    config_alias: alias,
                    reason: IntegrationSecretOmission::Missing,
                });
            }
            continue;
        };
        let reason = if !value.as_str().is_some_and(is_usable_secret) {
            Some(IntegrationSecretOmission::Masked)
        } else if !is_exclusively_scanovate(alias_providers) {
            Some(IntegrationSecretOmission::ProviderMismatch)
        } else if permission != IntegrationSecretImportPermission::PERMITTED {
            Some(IntegrationSecretOmission::NotAuthorized)
        } else {
            match (bindings, bound.as_slice()) {
                (None, _) => None,
                (Some(_), []) => Some(IntegrationSecretOmission::Unbound),
                (Some(_), [binding]) => {
                    let matches = binding.provider_id
                        == SCANOVATE_AUTHENTICATOR_PROVIDER
                        && binding.secret_key == SCANOVATE_LIVENESS_SECRET_KEY
                        && binding.endpoint_key == SCANOVATE_LIVENESS_URL_KEY
                        && endpoint_of(settings)
                            == Some(binding.endpoint.as_str());
                    (!matches)
                        .then_some(IntegrationSecretOmission::BindingMismatch)
                }
                (Some(_), _) => {
                    Some(IntegrationSecretOmission::BindingMismatch)
                }
            }
        };
        let outcome = match reason {
            Some(reason) => {
                settings.remove(SCANOVATE_LIVENESS_SECRET_KEY);
                ImportedIntegrationSecret::Omitted {
                    config_alias: alias,
                    reason,
                }
            }
            None if bindings.is_some() => {
                ImportedIntegrationSecret::KeptBound {
                    config_alias: alias,
                }
            }
            None => ImportedIntegrationSecret::KeptAsAuthored {
                config_alias: alias,
            },
        };
        outcomes.push(outcome);
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Synthetic values only.
    const SYNTHETIC_SECRET: &str = "synthetic-liveness-secret-6f1d";
    const SOURCE_ENDPOINT: &str = "https://source.example.test/biometric";
    const ALIAS_SCANOVATE: &str = "scanovate-registration";
    const CONFIG_ID: &str = "3b3c7e73-0000-4000-8000-000000000001";

    fn realm(secret: &str, endpoint: &str) -> Value {
        json!({
            "realm": "tenant-x-event-y",
            "authenticationFlows": [{
                "alias": "registration form",
                "authenticationExecutions": [
                    {"authenticator": "registration-page-form", "authenticatorFlow": true},
                    {"authenticator": SCANOVATE_AUTHENTICATOR_PROVIDER,
                     "authenticatorConfig": ALIAS_SCANOVATE},
                    {"authenticator": "conditional-user-attribute",
                     "authenticatorConfig": "has-temporary-password"}
                ]
            }],
            "authenticatorConfig": [
                {"id": CONFIG_ID, "alias": ALIAS_SCANOVATE, "config": {
                    "ocr-url": "http://ocr.example.test:5040",
                    "liveness-url": endpoint,
                    "liveness-secret": secret,
                    "max-attempts": "3"
                }},
                {"alias": "has-temporary-password", "config": {
                    "userAttribute": "sequent.read-only.expirationDate",
                    "liveness-secret": KEYCLOAK_MASKED_SECRET
                }}
            ]
        })
    }

    fn stored(alias: &str, value: &str) -> StoredIntegrationSecret {
        StoredIntegrationSecret {
            config_id: CONFIG_ID.to_string(),
            config_alias: alias.to_string(),
            provider_id: SCANOVATE_AUTHENTICATOR_PROVIDER.to_string(),
            secret_key: SCANOVATE_LIVENESS_SECRET_KEY.to_string(),
            value: IntegrationSecretValue::new(value.to_string()),
            endpoint: Some(SOURCE_ENDPOINT.to_string()),
        }
    }

    fn embed(
        realm: &mut Value,
        stored: &[StoredIntegrationSecret],
    ) -> Vec<IntegrationSecretBinding> {
        embed_integration_secrets(realm, stored).expect("consistent snapshot")
    }

    fn reconcile(
        realm: &mut Value,
        bindings: Option<&[IntegrationSecretBinding]>,
    ) -> Vec<ImportedIntegrationSecret> {
        reconcile_imported_integration_secrets(
            realm,
            bindings,
            IntegrationSecretImportPermission::PERMITTED,
        )
    }

    fn binding(endpoint: &str) -> IntegrationSecretBinding {
        IntegrationSecretBinding {
            config_alias: ALIAS_SCANOVATE.to_string(),
            provider_id: SCANOVATE_AUTHENTICATOR_PROVIDER.to_string(),
            secret_key: SCANOVATE_LIVENESS_SECRET_KEY.to_string(),
            endpoint_key: SCANOVATE_LIVENESS_URL_KEY.to_string(),
            endpoint: endpoint.to_string(),
        }
    }

    fn setting<'a>(
        realm: &'a Value,
        alias: &str,
        key: &str,
    ) -> Option<&'a Value> {
        realm["authenticatorConfig"]
            .as_array()
            .unwrap()
            .iter()
            .find(|config| config["alias"] == alias)
            .unwrap()["config"]
            .get(key)
    }

    #[test]
    fn export_replaces_the_mask_with_the_stored_secret_and_binds_it() {
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        let bindings =
            embed(&mut exported, &[stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET)]);
        assert_eq!(
            setting(&exported, ALIAS_SCANOVATE, "liveness-secret"),
            Some(&json!(SYNTHETIC_SECRET))
        );
        assert_eq!(bindings, vec![binding(SOURCE_ENDPOINT)]);
        // Every other setting is untouched, including another
        // configuration's masked value: only the designated class travels.
        assert_eq!(
            setting(&exported, ALIAS_SCANOVATE, "ocr-url"),
            Some(&json!("http://ocr.example.test:5040"))
        );
        assert_eq!(
            setting(&exported, "has-temporary-password", "liveness-secret"),
            Some(&json!(KEYCLOAK_MASKED_SECRET))
        );
    }

    fn assert_unavailable(
        exported: &mut Value,
        stored_secrets: &[StoredIntegrationSecret],
    ) {
        let error = embed_integration_secrets(exported, stored_secrets)
            .expect_err(
                "a requested secret that cannot travel must fail the export",
            );
        assert_eq!(
            error,
            IntegrationSecretExportError::Unavailable {
                config_alias: ALIAS_SCANOVATE.to_string()
            },
            "{stored_secrets:?}"
        );
        assert!(!error.to_string().contains(SYNTHETIC_SECRET));
    }

    #[test]
    fn export_fails_rather_than_omit_an_unavailable_secret() {
        // No stored value, an unusable one, one for another provider, alias
        // or setting, and two conflicting values for the alias.
        let mut other_provider = stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
        other_provider.provider_id = "other-authenticator".to_string();
        let mut other_key = stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
        other_key.secret_key = "ocr-url".to_string();
        for stored_secrets in [
            vec![],
            vec![stored(ALIAS_SCANOVATE, KEYCLOAK_MASKED_SECRET)],
            vec![stored(ALIAS_SCANOVATE, "  ")],
            vec![stored("another-alias", SYNTHETIC_SECRET)],
            vec![other_provider],
            vec![other_key],
            vec![
                stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET),
                stored(ALIAS_SCANOVATE, "synthetic-conflicting-secret"),
            ],
        ] {
            let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
            assert_unavailable(&mut exported, &stored_secrets);
        }
    }

    #[test]
    fn export_fails_for_a_configuration_shared_with_another_provider() {
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        exported["authenticationFlows"][0]["authenticationExecutions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"authenticator": "attacker-authenticator",
                         "authenticatorConfig": ALIAS_SCANOVATE}));
        assert_unavailable(
            &mut exported,
            &[stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET)],
        );
    }

    #[test]
    fn export_fails_without_an_endpoint() {
        for endpoint in [json!(null), json!(""), json!(7)] {
            let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
            exported["authenticatorConfig"][0]["config"]["liveness-url"] =
                endpoint.clone();
            // The stored snapshot agrees: no usable endpoint either.
            let mut without_endpoint =
                stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
            without_endpoint.endpoint = endpoint.as_str().map(str::to_string);
            assert_unavailable(&mut exported, &[without_endpoint]);
        }
    }

    #[test]
    fn export_without_scanovate_configuration_carries_no_binding() {
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        exported["authenticationFlows"][0]["authenticationExecutions"][1]
            ["authenticator"] = json!("other-authenticator");
        let before = exported.clone();
        assert_eq!(embed(&mut exported, &[]), vec![]);
        assert_eq!(exported, before);
    }

    #[test]
    fn import_reports_a_bound_secret_the_realm_no_longer_carries() {
        let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
        imported["authenticatorConfig"][0]["config"]
            .as_object_mut()
            .unwrap()
            .remove("liveness-secret");
        let outcome =
            reconcile(&mut imported, Some(&[binding(SOURCE_ENDPOINT)]));
        assert_eq!(
            outcome,
            vec![ImportedIntegrationSecret::Omitted {
                config_alias: ALIAS_SCANOVATE.to_string(),
                reason: IntegrationSecretOmission::Missing,
            }]
        );
    }

    #[test]
    fn exported_secret_roundtrips_into_the_imported_realm() {
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        let bindings =
            embed(&mut exported, &[stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET)]);
        // The archive is JSON: go through bytes as the import does.
        let mut imported: Value =
            serde_json::from_slice(&serde_json::to_vec(&exported).unwrap())
                .unwrap();
        let bindings: Vec<IntegrationSecretBinding> =
            serde_json::from_slice(&serde_json::to_vec(&bindings).unwrap())
                .unwrap();
        let outcome = reconcile(&mut imported, Some(&bindings));
        assert_eq!(
            outcome,
            vec![ImportedIntegrationSecret::KeptBound {
                config_alias: ALIAS_SCANOVATE.to_string()
            }]
        );
        assert_eq!(
            setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
            Some(&json!(SYNTHETIC_SECRET))
        );
    }

    #[test]
    fn import_removes_a_masked_or_empty_secret_with_guidance() {
        for value in [KEYCLOAK_MASKED_SECRET, "", "   "] {
            for bindings in [None, Some(vec![binding(SOURCE_ENDPOINT)])] {
                let mut imported = realm(value, SOURCE_ENDPOINT);
                let outcome = reconcile(&mut imported, bindings.as_deref());
                assert_eq!(
                    outcome,
                    vec![ImportedIntegrationSecret::Omitted {
                        config_alias: ALIAS_SCANOVATE.to_string(),
                        reason: IntegrationSecretOmission::Masked,
                    }],
                    "{value:?} {bindings:?}"
                );
                assert_eq!(
                    setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
                    None
                );
                // Non-designated configurations keep their settings as before.
                assert_eq!(
                    setting(
                        &imported,
                        "has-temporary-password",
                        "liveness-secret"
                    ),
                    Some(&json!(KEYCLOAK_MASKED_SECRET))
                );
            }
        }
    }

    #[test]
    fn import_removes_a_secret_bound_to_another_endpoint_or_provider() {
        let mut other_provider = binding(SOURCE_ENDPOINT);
        other_provider.provider_id = "other-authenticator".to_string();
        let mut other_endpoint_key = binding(SOURCE_ENDPOINT);
        other_endpoint_key.endpoint_key = "ocr-url".to_string();
        let mut other_secret_key = binding(SOURCE_ENDPOINT);
        other_secret_key.secret_key = "ocr-url".to_string();
        for bound in [
            binding("https://attacker.example.test/biometric"),
            other_provider,
            other_endpoint_key,
            other_secret_key,
        ] {
            let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
            let outcome = reconcile(&mut imported, Some(&[bound.clone()]));
            assert_eq!(
                outcome,
                vec![ImportedIntegrationSecret::Omitted {
                    config_alias: ALIAS_SCANOVATE.to_string(),
                    reason: IntegrationSecretOmission::BindingMismatch,
                }],
                "{bound:?}"
            );
            assert_eq!(
                setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
                None
            );
        }
    }

    #[test]
    fn import_removes_a_secret_whose_configuration_another_provider_uses() {
        for bindings in [None, Some(vec![binding(SOURCE_ENDPOINT)])] {
            let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
            imported["authenticationFlows"][0]["authenticationExecutions"]
                .as_array_mut()
                .unwrap()
                .push(json!({"authenticator": "attacker-authenticator",
                             "authenticatorConfig": ALIAS_SCANOVATE}));
            let outcome = reconcile(&mut imported, bindings.as_deref());
            assert_eq!(
                outcome,
                vec![ImportedIntegrationSecret::Omitted {
                    config_alias: ALIAS_SCANOVATE.to_string(),
                    reason: IntegrationSecretOmission::ProviderMismatch,
                }]
            );
            assert_eq!(
                setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
                None
            );
        }
    }

    #[test]
    fn import_removes_an_unbound_secret_from_a_secret_bearing_archive() {
        let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
        let outcome = reconcile(&mut imported, Some(&[]));
        assert_eq!(
            outcome,
            vec![ImportedIntegrationSecret::Omitted {
                config_alias: ALIAS_SCANOVATE.to_string(),
                reason: IntegrationSecretOmission::Unbound,
            }]
        );
        assert_eq!(
            setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
            None
        );
    }

    #[test]
    fn import_keeps_an_authored_secret_of_a_bundle_without_bindings() {
        // Template-generated bundles (janitor) carry the real value and no
        // bindings; they keep importing as before.
        let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
        let before = imported.clone();
        let outcome = reconcile(&mut imported, None);
        assert_eq!(
            outcome,
            vec![ImportedIntegrationSecret::KeptAsAuthored {
                config_alias: ALIAS_SCANOVATE.to_string()
            }]
        );
        assert_eq!(imported, before);
    }

    #[test]
    fn import_without_designated_configuration_changes_nothing() {
        let mut imported = json!({"realm": "r", "authenticatorConfig": [
            {"alias": "has-temporary-password",
             "config": {"liveness-secret": KEYCLOAK_MASKED_SECRET}}]});
        let before = imported.clone();
        assert_eq!(reconcile(&mut imported, None), vec![]);
        assert_eq!(imported, before);
        let mut empty = json!({});
        assert_eq!(reconcile(&mut empty, Some(&[])), vec![]);
    }

    #[test]
    fn diagnostics_never_show_the_secret() {
        let secret = stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
        assert!(!format!("{secret:?}").contains(SYNTHETIC_SECRET));
        assert!(!format!("{:#?}", secret.value).contains(SYNTHETIC_SECRET));

        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        let bindings = embed(&mut exported, &[secret]);
        assert!(!serde_json::to_string(&bindings)
            .unwrap()
            .contains(SYNTHETIC_SECRET));
        assert!(!format!("{bindings:?}").contains(SYNTHETIC_SECRET));

        for bindings in [None, Some(vec![]), Some(bindings)] {
            let mut imported = exported.clone();
            for outcome in reconcile(&mut imported, bindings.as_deref()) {
                assert!(!outcome.to_string().contains(SYNTHETIC_SECRET));
                assert!(!format!("{outcome:?}").contains(SYNTHETIC_SECRET));
            }
        }
    }

    #[test]
    fn export_fails_when_the_configuration_changed_after_the_realm_export() {
        // The stored row was read after an edit: another endpoint or another
        // configuration under the same alias. Never pair them.
        let mut moved = stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
        moved.endpoint =
            Some("https://elsewhere.example.test/biometric".to_string());
        let mut recreated = stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
        recreated.config_id =
            "3b3c7e73-0000-4000-8000-000000000002".to_string();
        let mut cleared = stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET);
        cleared.endpoint = None;
        for changed in [moved, recreated, cleared] {
            let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
            let error =
                embed_integration_secrets(&mut exported, &[changed.clone()])
                    .expect_err("a changed configuration must fail the export");
            assert_eq!(
                error,
                IntegrationSecretExportError::ConfigurationChanged {
                    config_alias: ALIAS_SCANOVATE.to_string()
                },
                "{changed:?}"
            );
            assert!(!error.to_string().contains(SYNTHETIC_SECRET));
        }
        // Control: the same snapshot embeds.
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        assert!(embed_integration_secrets(
            &mut exported,
            &[stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET)]
        )
        .is_ok());
    }

    #[test]
    fn export_treats_one_row_per_execution_as_one_secret() {
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        let bindings = embed(
            &mut exported,
            &[
                stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET),
                stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET),
            ],
        );
        assert_eq!(bindings, vec![binding(SOURCE_ENDPOINT)]);
    }

    #[test]
    fn import_without_permission_installs_no_archive_credential() {
        let mut exported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        let bindings =
            embed(&mut exported, &[stored(ALIAS_SCANOVATE, SYNTHETIC_SECRET)]);
        for bindings in [None, Some(bindings)] {
            let mut imported = exported.clone();
            let outcome = reconcile_imported_integration_secrets(
                &mut imported,
                bindings.as_deref(),
                IntegrationSecretImportPermission::NOT_PERMITTED,
            );
            assert_eq!(
                outcome,
                vec![ImportedIntegrationSecret::Omitted {
                    config_alias: ALIAS_SCANOVATE.to_string(),
                    reason: IntegrationSecretOmission::NotAuthorized,
                }]
            );
            assert_eq!(
                setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
                None
            );
            assert_eq!(
                setting(&imported, ALIAS_SCANOVATE, "liveness-url"),
                Some(&json!(SOURCE_ENDPOINT))
            );
        }
    }

    #[test]
    fn import_removes_a_secret_with_duplicate_bindings() {
        let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
        let outcome = reconcile(
            &mut imported,
            Some(&[
                binding(SOURCE_ENDPOINT),
                binding("https://other.example.test"),
            ]),
        );
        assert_eq!(
            outcome,
            vec![ImportedIntegrationSecret::Omitted {
                config_alias: ALIAS_SCANOVATE.to_string(),
                reason: IntegrationSecretOmission::BindingMismatch,
            }]
        );
    }

    #[test]
    fn import_removes_a_bound_secret_from_a_configuration_no_scanovate_execution_uses(
    ) {
        let mut imported = realm(SYNTHETIC_SECRET, SOURCE_ENDPOINT);
        imported["authenticationFlows"][0]["authenticationExecutions"][1]
            ["authenticator"] = json!("attacker-authenticator");
        let outcome =
            reconcile(&mut imported, Some(&[binding(SOURCE_ENDPOINT)]));
        assert_eq!(
            outcome,
            vec![ImportedIntegrationSecret::Omitted {
                config_alias: ALIAS_SCANOVATE.to_string(),
                reason: IntegrationSecretOmission::ProviderMismatch,
            }]
        );
        assert_eq!(
            setting(&imported, ALIAS_SCANOVATE, "liveness-secret"),
            None
        );
    }

    #[test]
    fn import_removes_a_usable_secret_from_configurations_scanovate_does_not_use(
    ) {
        // An orphan configuration and another provider's, each carrying a
        // real value under the Scanovate key, even for a permitted importer.
        let mut imported = realm(KEYCLOAK_MASKED_SECRET, SOURCE_ENDPOINT);
        let configs = imported["authenticatorConfig"].as_array_mut().unwrap();
        configs.push(json!({"alias": "orphan", "config": {
            "liveness-url": SOURCE_ENDPOINT, "liveness-secret": SYNTHETIC_SECRET}}));
        configs[1]["config"]["liveness-secret"] = json!(SYNTHETIC_SECRET);
        let outcome = reconcile(&mut imported, None);
        let omitted =
            |alias: &str, reason| ImportedIntegrationSecret::Omitted {
                config_alias: alias.to_string(),
                reason,
            };
        assert_eq!(
            outcome,
            vec![
                omitted(ALIAS_SCANOVATE, IntegrationSecretOmission::Masked),
                omitted(
                    "has-temporary-password",
                    IntegrationSecretOmission::ProviderMismatch
                ),
                omitted("orphan", IntegrationSecretOmission::ProviderMismatch),
            ]
        );
        for alias in ["has-temporary-password", "orphan"] {
            assert_eq!(
                setting(&imported, alias, "liveness-secret"),
                None,
                "{alias}"
            );
        }
    }

    #[test]
    fn the_export_policy_defaults_to_omitted() {
        assert_eq!(
            IntegrationSecretsExport::default(),
            IntegrationSecretsExport::OMITTED
        );
        assert_eq!(
            serde_json::to_value(IntegrationSecretsExport::INCLUDED).unwrap(),
            json!("INCLUDED")
        );
    }
}
