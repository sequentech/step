// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keycloak's partial export, which election-event exports use, writes
//! `**********` in place of secrets. Keycloak reads the mask as "unchanged"
//! only when it updates an existing realm: a new realm stores it as the value.

use keycloak::types::{ComponentExportRepresentation, RealmRepresentation};
use std::collections::HashMap;
use std::fmt;

/// The value Keycloak's partial export writes in place of a secret.
const KEYCLOAK_SECRET_MASK: &str = "**********";

/// Where in a realm a masked secret was found.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaskedSecretLocation {
    SmtpServer,
    Client { client_id: String },
    IdentityProvider { alias: String },
    AuthenticatorConfig { alias: String },
    Component { provider_type: String, name: String },
}

/// A realm setting left unset because the export held Keycloak's secret mask
/// instead of its value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaskedSecret {
    pub location: MaskedSecretLocation,
    pub key: String,
}

impl fmt::Display for MaskedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = &self.key;
        match &self.location {
            MaskedSecretLocation::SmtpServer => write!(f, "SMTP server setting '{key}'"),
            MaskedSecretLocation::Client { client_id } => {
                write!(f, "Client '{client_id}' setting '{key}'")
            }
            MaskedSecretLocation::IdentityProvider { alias } => {
                write!(f, "Identity provider '{alias}' setting '{key}'")
            }
            MaskedSecretLocation::AuthenticatorConfig { alias } => {
                write!(f, "Authenticator config '{alias}' setting '{key}'")
            }
            MaskedSecretLocation::Component {
                provider_type,
                name,
            } => write!(f, "Component '{name}' ({provider_type}) setting '{key}'"),
        }?;
        write!(
            f,
            " not imported: the export holds Keycloak's secret mask instead of its value. \
             Re-enter it in Keycloak."
        )
    }
}

fn is_masked(value: &str) -> bool {
    value.trim() == KEYCLOAK_SECRET_MASK
}

/// Removes the masked entries of `config`, recording each under `location`.
fn remove_masked_values(
    config: &mut HashMap<String, String>,
    location: &MaskedSecretLocation,
    removed: &mut Vec<MaskedSecret>,
) {
    config.retain(|key, value| {
        let masked = is_masked(value);
        if masked {
            removed.push(MaskedSecret {
                location: location.clone(),
                key: key.clone(),
            });
        }
        !masked
    });
}

fn remove_masked_component_values(
    components: &mut HashMap<String, Vec<ComponentExportRepresentation>>,
    removed: &mut Vec<MaskedSecret>,
) {
    for (provider_type, of_type) in components.iter_mut() {
        for component in of_type.iter_mut() {
            let location = MaskedSecretLocation::Component {
                provider_type: provider_type.clone(),
                name: component
                    .name
                    .clone()
                    .or_else(|| component.provider_id.clone())
                    .unwrap_or_default(),
            };
            if let Some(config) = component.config.as_mut() {
                config.retain(|key, values| {
                    let masked = values.iter().any(|value| is_masked(value));
                    if masked {
                        removed.push(MaskedSecret {
                            location: location.clone(),
                            key: key.clone(),
                        });
                    }
                    !masked
                });
            }
            if let Some(sub_components) = component.sub_components.as_mut() {
                remove_masked_component_values(sub_components, removed);
            }
        }
    }
}

/// Removes every masked setting of the SMTP server, clients, identity
/// providers, authenticator configs and components, so Keycloak leaves them
/// unset instead of storing the mask, and lists what it removed. Client
/// secrets, rotated secrets and registration access tokens are not among them:
/// `remove_keycloak_realm_secrets` sets or clears those first.
pub(super) fn remove_masked_secrets(realm: &mut RealmRepresentation) -> Vec<MaskedSecret> {
    let mut removed = Vec::new();
    if let Some(smtp_server) = realm.smtp_server.as_mut() {
        remove_masked_values(smtp_server, &MaskedSecretLocation::SmtpServer, &mut removed);
    }
    for client in realm.clients.iter_mut().flatten() {
        let location = MaskedSecretLocation::Client {
            client_id: client.client_id.clone().unwrap_or_default(),
        };
        if let Some(attributes) = client.attributes.as_mut() {
            remove_masked_values(attributes, &location, &mut removed);
        }
    }
    for provider in realm.identity_providers.iter_mut().flatten() {
        let location = MaskedSecretLocation::IdentityProvider {
            alias: provider.alias.clone().unwrap_or_default(),
        };
        if let Some(config) = provider.config.as_mut() {
            remove_masked_values(config, &location, &mut removed);
        }
    }
    for authenticator_config in realm.authenticator_config.iter_mut().flatten() {
        let location = MaskedSecretLocation::AuthenticatorConfig {
            alias: authenticator_config.alias.clone().unwrap_or_default(),
        };
        if let Some(config) = authenticator_config.config.as_mut() {
            remove_masked_values(config, &location, &mut removed);
        }
    }
    if let Some(components) = realm.components.as_mut() {
        remove_masked_component_values(components, &mut removed);
    }
    removed.sort();
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const VAULT_REFERENCE: &str = "${vault.custom_idp_secret}";
    const USER_STORAGE_PROVIDER: &str = "org.keycloak.storage.UserStorageProvider";
    const LDAP_MAPPER: &str = "org.keycloak.storage.ldap.mappers.LDAPStorageMapper";

    fn realm(value: serde_json::Value) -> RealmRepresentation {
        serde_json::from_value(value).unwrap()
    }

    fn masked_realm() -> RealmRepresentation {
        realm(json!({
            "smtpServer": {
                "host": "smtp.example.test",
                "user": "mailer",
                "auth": "true",
                "password": KEYCLOAK_SECRET_MASK
            },
            "clients": [{
                "clientId": "saml-client",
                "attributes": {
                    "saml.signing.private.key": KEYCLOAK_SECRET_MASK,
                    "saml.signature.algorithm": "RSA_SHA256"
                }
            }],
            "identityProviders": [
                {
                    "alias": "custom-oidc",
                    "providerId": "oidc",
                    "config": {"clientId": "step", "clientSecret": KEYCLOAK_SECRET_MASK}
                },
                {
                    "alias": "vault-oidc",
                    "providerId": "oidc",
                    "config": {"clientId": "step", "clientSecret": VAULT_REFERENCE}
                }
            ],
            "authenticatorConfig": [
                {
                    "alias": "inetum-registration",
                    "config": {
                        "api-key": KEYCLOAK_SECRET_MASK,
                        "base-url": "https://inetum.example.test"
                    }
                },
                {"alias": "already-validated-alias"}
            ],
            "components": {
                USER_STORAGE_PROVIDER: [{
                    "name": "ldap",
                    "providerId": "ldap",
                    "config": {
                        "bindCredential": [KEYCLOAK_SECRET_MASK],
                        "bindDn": ["cn=admin"]
                    },
                    "subComponents": {
                        LDAP_MAPPER: [{
                            "providerId": "user-attribute-ldap-mapper",
                            "config": {"token": [KEYCLOAK_SECRET_MASK]}
                        }]
                    }
                }]
            }
        }))
    }

    fn location_of(removed: &[MaskedSecret], key: &str) -> Option<MaskedSecretLocation> {
        removed
            .iter()
            .find(|secret| secret.key == key)
            .map(|secret| secret.location.clone())
    }

    #[test]
    fn removes_and_reports_every_masked_setting() {
        let mut realm = masked_realm();
        let removed = remove_masked_secrets(&mut realm);

        assert_eq!(removed.len(), 6, "{removed:?}");
        assert_eq!(
            location_of(&removed, "password"),
            Some(MaskedSecretLocation::SmtpServer)
        );
        assert_eq!(
            location_of(&removed, "saml.signing.private.key"),
            Some(MaskedSecretLocation::Client {
                client_id: "saml-client".to_string()
            })
        );
        assert_eq!(
            location_of(&removed, "clientSecret"),
            Some(MaskedSecretLocation::IdentityProvider {
                alias: "custom-oidc".to_string()
            })
        );
        assert_eq!(
            location_of(&removed, "api-key"),
            Some(MaskedSecretLocation::AuthenticatorConfig {
                alias: "inetum-registration".to_string()
            })
        );
        assert_eq!(
            location_of(&removed, "bindCredential"),
            Some(MaskedSecretLocation::Component {
                provider_type: USER_STORAGE_PROVIDER.to_string(),
                name: "ldap".to_string()
            })
        );
        assert_eq!(
            location_of(&removed, "token"),
            Some(MaskedSecretLocation::Component {
                provider_type: LDAP_MAPPER.to_string(),
                name: "user-attribute-ldap-mapper".to_string()
            })
        );
        assert!(!serde_json::to_string(&realm)
            .unwrap()
            .contains(KEYCLOAK_SECRET_MASK));
    }

    #[test]
    fn keeps_the_other_settings_and_vault_references() {
        let mut realm = masked_realm();
        remove_masked_secrets(&mut realm);

        let smtp_server = realm.smtp_server.unwrap();
        assert_eq!(smtp_server.get("host").unwrap(), "smtp.example.test");
        assert_eq!(smtp_server.get("user").unwrap(), "mailer");
        assert!(!smtp_server.contains_key("password"));

        let clients = realm.clients.unwrap();
        let attributes = clients[0].attributes.as_ref().unwrap();
        assert!(!attributes.contains_key("saml.signing.private.key"));
        assert_eq!(
            attributes.get("saml.signature.algorithm").unwrap(),
            "RSA_SHA256"
        );

        let providers = realm.identity_providers.unwrap();
        let config_of = |alias: &str| {
            providers
                .iter()
                .find(|provider| provider.alias.as_deref() == Some(alias))
                .and_then(|provider| provider.config.clone())
                .unwrap()
        };
        assert!(!config_of("custom-oidc").contains_key("clientSecret"));
        assert_eq!(config_of("custom-oidc").get("clientId").unwrap(), "step");
        assert_eq!(
            config_of("vault-oidc").get("clientSecret").unwrap(),
            VAULT_REFERENCE
        );

        let inetum = realm.authenticator_config.unwrap()[0]
            .config
            .clone()
            .unwrap();
        assert!(!inetum.contains_key("api-key"));
        assert_eq!(
            inetum.get("base-url").unwrap(),
            "https://inetum.example.test"
        );

        let components = realm.components.unwrap();
        let ldap = &components[USER_STORAGE_PROVIDER][0];
        let ldap_config = ldap.config.as_ref().unwrap();
        assert!(!ldap_config.contains_key("bindCredential"));
        assert_eq!(ldap_config["bindDn"], vec!["cn=admin".to_string()]);
    }

    #[test]
    fn leaves_a_realm_without_masked_values_unchanged() {
        let original = realm(json!({
            "smtpServer": {"host": "smtp.example.test", "password": "real-password"},
            "clients": [{
                "clientId": "voting-portal",
                "secret": "generated-secret",
                "attributes": {"pkce.code.challenge.method": "S256"}
            }],
            "identityProviders": [{
                "alias": "vault-oidc",
                "config": {"clientSecret": VAULT_REFERENCE}
            }],
            "authenticatorConfig": [{"alias": "otp-sms", "config": {"length": "6"}}]
        }));
        let mut realm = original.clone();

        assert!(remove_masked_secrets(&mut realm).is_empty());
        assert_eq!(realm, original);
    }

    #[test]
    fn handles_a_realm_without_secret_bearing_settings() {
        let mut realm = RealmRepresentation::default();
        assert!(remove_masked_secrets(&mut realm).is_empty());
        assert_eq!(realm, RealmRepresentation::default());
    }

    #[test]
    fn names_the_setting_and_how_to_restore_it() {
        let secret = MaskedSecret {
            location: MaskedSecretLocation::SmtpServer,
            key: "password".to_string(),
        };
        let message = secret.to_string();
        assert!(message.starts_with("SMTP server setting 'password' not imported"));
        assert!(message.contains("Re-enter it in Keycloak"));
    }
}
