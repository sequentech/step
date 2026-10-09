// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{
    alphabet,
    engine::{general_purpose, DecodePaddingMode, GeneralPurpose},
    Engine,
};
use keycloak::types::RealmRepresentation;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

// Certificate bundled with the development SimpleSAMLphp identity provider.
const DEMO_CERTIFICATE_DER_LENGTH: usize = 670;
const DEMO_CERTIFICATE_SHA256: [u8; 32] = [
    0x89, 0x2a, 0x36, 0x0b, 0xf0, 0xbf, 0x83, 0x94, 0x61, 0xba, 0x30, 0xcc, 0xac, 0x0b, 0x30, 0x07,
    0x11, 0xc3, 0x8f, 0x84, 0xc8, 0xcd, 0x9c, 0xc4, 0x2f, 0xcb, 0xa7, 0x8d, 0x27, 0x8b, 0xc9, 0x2d,
];

fn contains_demo_certificate(certificates: &str) -> bool {
    // Keycloak accepts a comma-separated list during certificate rotation.
    certificates.split(',').any(|certificate| {
        // Keycloak strips PEM labels then uses Java's MIME Base64 decoder,
        // which ignores all characters outside the Base64 alphabet.
        let normalized: String = certificate
            .split("-----")
            .filter(|part| !part.starts_with("BEGIN ") && !part.starts_with("END "))
            .flat_map(str::chars)
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '='))
            .collect();
        let decoder = GeneralPurpose::new(
            &alphabet::STANDARD,
            general_purpose::GeneralPurposeConfig::new()
                .with_decode_padding_mode(DecodePaddingMode::Indifferent)
                .with_decode_allow_trailing_bits(true),
        );
        // X.509 readers consume the first certificate and can ignore trailing bytes.
        decoder.decode(normalized).is_ok_and(|der| {
            der.get(..DEMO_CERTIFICATE_DER_LENGTH)
                .is_some_and(|certificate| {
                    Sha256::digest(certificate).as_slice() == DEMO_CERTIFICATE_SHA256
                })
        })
    })
}

/// Remove the development trust anchor from default and imported realms.
/// Match the certificate rather than an alias, preserving custom IdPs.
pub(super) fn remove_demo_identity_providers(realm: &mut RealmRepresentation) {
    let mut removed_aliases = HashSet::new();
    if let Some(providers) = realm.identity_providers.as_mut() {
        providers.retain(|provider| {
            let demo = provider.provider_id.as_deref() == Some("saml")
                && provider
                    .config
                    .as_ref()
                    .and_then(|config| config.get("signingCertificate"))
                    .is_some_and(|certificates| contains_demo_certificate(certificates));
            if demo {
                if let Some(alias) = &provider.alias {
                    removed_aliases.insert(alias.clone());
                }
            }
            !demo
        });
    }
    if let Some(mappers) = realm.identity_provider_mappers.as_mut() {
        mappers.retain(|mapper| {
            !mapper
                .identity_provider_alias
                .as_ref()
                .is_some_and(|alias| removed_aliases.contains(alias))
        });
    }
    if let Some(configs) = realm.authenticator_config.as_mut() {
        for config in configs {
            if let Some(values) = config.config.as_mut() {
                if values
                    .get("defaultProvider")
                    .is_some_and(|alias| removed_aliases.contains(alias))
                {
                    values.remove("defaultProvider");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DEMO_CERTIFICATE: &str = "MIICmjCCAYICCQDX5sKPsYV3+jANBgkqhkiG9w0BAQsFADAPMQ0wCwYDVQQDDAR0ZXN0MB4XDTE5MTIyMzA5MDI1MVoXDTIwMDEyMjA5MDI1MVowDzENMAsGA1UEAwwEdGVzdDCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAMdtDJ278DQTp84O5Nq5F8s5YOR34GFOGI2Swb/3pU7X7918lVljiKv7WVM65S59nJSyXV+fa15qoXLfsdRnq3yw0hTSTs2YDX+jl98kK3ksk3rROfYh1LIgByj4/4NeNpExgeB6rQk5Ay7YS+ARmMzEjXa0favHxu5BOdB2y6WvRQyjPS2lirT/PKWBZc04QZepsZ56+W7bd557tdedcYdY/nKI1qmSQClG2qgslzgqFOv1KCOw43a3mcK/TiiD8IXyLMJNC6OFW3xTL/BG6SOZ3dQ9rjQOBga+6GIaQsDjC4Xp7Kx+FkSvgaw0sJV8gt1mlZy+27Sza6d+hHD2pWECAwEAATANBgkqhkiG9w0BAQsFAAOCAQEAm2fk1+gd08FQxK7TL04O8EK1f0bzaGGUxWzlh98a3Dm8+OPhVQRi/KLsFHliLC86lsZQKunYdDB+qd0KUk2oqDG6tstG/htmRYD/S/jNmt8gyPAVi11dHUqW3IvQgJLwxZtoAv6PNs188hvT1WK3VWJ4YgFKYi5XQYnR5sv69Vsr91lYAxyrIlMKahjSW1jTD3ByRfAQghsSLk6fV0OyJHyhuF1TxOVBVf8XOdaqfmvD90JGIPGtfMLPUX4m35qaGAU48PwCL7L3cRHYs9wZWc0ifXZcBENLtHYCLi5txR8c5lyHB9d3AQHzKHMFNjLswn5HsckKg83RH7+eVqHqGw==";

    const CUSTOM_CERTIFICATE: &str = "MIIDCzCCAfOgAwIBAgIUOGnn8cp9At/qlKuASkX1V3F0se8wDQYJKoZIhvcNAQELBQAwFTETMBEGA1UEAwwKY3VzdG9tLWlkcDAeFw0yNjEwMDkxNTUzMjhaFw0yNjEwMTAxNTUzMjhaMBUxEzARBgNVBAMMCmN1c3RvbS1pZHAwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQCmQCGmKT4Z7Iqc9IL3Hby4Asei+xj1VK9qykl538ETn6twPfrGyfnbDeSoXahFbOp9sNwuCQ/XgtOw+skc1+dYyVb4iOasvU2TOYd3NpgFZ75xhPDim4IP5DbQY0ALF84BOmzdhqqQK01bydVU/UMRX80JgPnYSNTOIXjVqCJnlIm0GRQZ52aaR52splZgJEyxw3S7ONkvEb6JTPANex86QyEA7LVABehTRWYhaUWCpbP2LwxxnhWjLT5bD5eY+zxySapYyEBvFinpIZPyt7LBnHGBkBeF4cBhwAFB5kEFyOqP8arcfZ+WL+2qAqbfCmoifPM+fZSSOJwEPYLfvYTLAgMBAAGjUzBRMB0GA1UdDgQWBBSqAmXX+8ZUrDavREaO+U95pgiCpTAfBgNVHSMEGDAWgBSqAmXX+8ZUrDavREaO+U95pgiCpTAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUAA4IBAQBQaj0Oan6wpCihjs3U3ly/zUmhNnW5J96kaQT0BdsBwFf/IcHCriqtgBB98BvnkEvut8YzoRKjV2CYOtHb6FjBh8fNxnZ+Wk13ppFt5nKM9lQb8IN6w9KvuK3l9oUJfV/J3iEMiKbVOfDs5ov95N/OenSF2MC/mlENOyZez6AMJ8Zx5DGH2yKB4wFBjBYFMxAlObTD38H0INTVFdSGb7ONEmZ+2UlCLO8O2/+tTNfs2mKQsgXLM9VNEmwuUzIHnhdJ1qdEFcSGP8rJSMbfmnNCTsTS7HdVnFlqWYWs/7gr7Pqss5i4YkItJUCxhApnQ3KsZ7SoS+yAx8CzsUVO0KsH";

    fn provider(alias: &str, certificate: &str) -> serde_json::Value {
        json!({"alias": alias, "providerId": "saml", "enabled": true,
               "config": {"signingCertificate": certificate}})
    }

    #[test]
    fn removes_demo_certificate_under_any_alias_and_in_rotated_certificate_lists() {
        for certificate in [
            DEMO_CERTIFICATE.to_string(),
            DEMO_CERTIFICATE
                .chars()
                .map(|ch| format!("{ch}!@"))
                .collect::<String>(),
            format!(
                "-----BEGIN X509 CERTIFICATE-----\n{}\n-----END X509 CERTIFICATE-----",
                DEMO_CERTIFICATE
            ),
            format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
                DEMO_CERTIFICATE
            ),
            format!("{},{}", CUSTOM_CERTIFICATE, DEMO_CERTIFICATE),
            DEMO_CERTIFICATE.trim_end_matches('=').to_string(),
            general_purpose::STANDARD.encode(
                [
                    general_purpose::STANDARD.decode(DEMO_CERTIFICATE).unwrap(),
                    b"trailing-data".to_vec(),
                ]
                .concat(),
            ),
        ] {
            let mut realm: RealmRepresentation = serde_json::from_value(json!({
                "identityProviders": [provider("renamed-demo", &certificate), provider("yourcompany-idp", CUSTOM_CERTIFICATE)],
                "identityProviderMappers": [
                    {"name": "demo", "identityProviderAlias": "renamed-demo"},
                    {"name": "custom", "identityProviderAlias": "yourcompany-idp"}
                ],
                "authenticatorConfig": [
                    {"alias": "demo-redirect", "config": {"defaultProvider": "renamed-demo", "other": "kept"}},
                    {"alias": "custom-redirect", "config": {"defaultProvider": "yourcompany-idp"}}
                ]
            })).unwrap();
            remove_demo_identity_providers(&mut realm);
            let value = serde_json::to_value(realm).unwrap();
            assert_eq!(
                value["identityProviders"],
                json!([provider("yourcompany-idp", CUSTOM_CERTIFICATE)])
            );
            assert_eq!(
                value["identityProviderMappers"],
                json!([{ "name": "custom", "identityProviderAlias": "yourcompany-idp" }])
            );
            assert_eq!(
                value["authenticatorConfig"][0]["config"],
                json!({"other": "kept"})
            );
            assert_eq!(
                value["authenticatorConfig"][1]["config"],
                json!({"defaultProvider": "yourcompany-idp"})
            );
        }
    }

    #[test]
    fn custom_and_certificate_identity_providers_remain_unchanged() {
        let mut realm: RealmRepresentation = serde_json::from_value(json!({
            "identityProviders": [
                provider("simplesamlphp", CUSTOM_CERTIFICATE),
                provider("yourcompany-idp", ""),
                {"alias": "digital-certificates", "providerId": "keycloak-oidc", "enabled": true,
                 "config": {"clientId": "certificate-client", "clientSecret": "custom-secret"}}
            ]
        }))
        .unwrap();
        let before = serde_json::to_value(&realm).unwrap();
        remove_demo_identity_providers(&mut realm);
        assert_eq!(serde_json::to_value(realm).unwrap(), before);
        remove_demo_identity_providers(&mut RealmRepresentation::default());
    }

    #[test]
    fn shipped_event_template_has_no_demo_trust_anchor() {
        let mut realm: RealmRepresentation = serde_json::from_str(include_str!(
            "../../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json"
        )).unwrap();
        let before = serde_json::to_value(&realm).unwrap();
        remove_demo_identity_providers(&mut realm);
        assert_eq!(serde_json::to_value(realm).unwrap(), before);
    }
}
