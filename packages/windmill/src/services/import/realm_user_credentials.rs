// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use keycloak::types::RealmRepresentation;

/// Imported users keep their metadata, but must not reuse template credentials.
pub(super) fn remove_user_credentials(realm: &mut RealmRepresentation) {
    if let Some(users) = realm.users.as_mut() {
        for user in users {
            user.credentials = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Strip password and OTP credentials without changing imported user metadata.
    #[test]
    fn imported_user_credentials_are_removed_without_losing_user_metadata() {
        let mut realm: RealmRepresentation = serde_json::from_value(json!({
            "users": [{
                "username": "custom-user",
                "enabled": true,
                "groups": ["/custom-group"],
                "attributes": {"locale": ["en"]},
                "requiredActions": ["VERIFY_EMAIL"],
                "federatedIdentities": [{"identityProvider": "custom-idp", "userId": "subject", "userName": "custom-user"}],
                "credentials": [
                    {"type": "password", "value": "synthetic-password"},
                    {"type": "otp", "secretData": "synthetic-otp"}
                ]
            }]
        })).unwrap();
        let mut expected = realm.clone();
        expected.users.as_mut().unwrap()[0].credentials = None;

        remove_user_credentials(&mut realm);

        assert_eq!(
            serde_json::to_value(realm).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    /// Passwordless service users and their client authentication remain unchanged.
    #[test]
    fn passwordless_users_and_service_client_configuration_are_preserved() {
        let mut realm: RealmRepresentation = serde_json::from_value(json!({
            "users": [{"username": "custom-service-account", "serviceAccountClientId": "custom-client", "groups": ["/admin"]}],
            "clients": [{"clientId": "custom-client", "serviceAccountsEnabled": true, "secret": "synthetic-client-secret"}]
        })).unwrap();
        let before = serde_json::to_value(&realm).unwrap();
        remove_user_credentials(&mut realm);
        assert_eq!(serde_json::to_value(realm).unwrap(), before);
        remove_user_credentials(&mut RealmRepresentation::default());
    }

    /// Prevent shipped realm templates from reintroducing user credentials.
    #[test]
    fn shipped_tenant_template_has_no_user_credentials() {
        let realm: RealmRepresentation = serde_json::from_str(include_str!(
            "../../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
        )).unwrap();
        assert!(realm
            .users
            .unwrap_or_default()
            .iter()
            .all(|user| user.credentials.as_ref().is_none_or(Vec::is_empty)));
    }
}
