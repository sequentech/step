// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use keycloak::types::{RealmRepresentation, UserRepresentation};

const ADMIN_USERNAME: &str = "admin";
const UPDATE_PASSWORD_ACTION: &str = "UPDATE_PASSWORD";

pub(super) enum TenantBootstrapAdminPolicy {
    ServiceAccount,
    PasswordlessAdmin,
    DiscardHumanSeed,
}

impl TenantBootstrapAdminPolicy {
    /// New tenants seed only client-backed service accounts and a passwordless admin.
    pub(super) fn apply_to_realm(realm: &mut RealmRepresentation) {
        if let Some(users) = realm.users.as_mut() {
            users.retain_mut(|user| match Self::for_user(user) {
                Self::ServiceAccount => true,
                Self::PasswordlessAdmin => {
                    user.credentials = None;
                    let actions = user.required_actions.get_or_insert_with(Vec::new);
                    let update_password = UPDATE_PASSWORD_ACTION.to_string();
                    if !actions.contains(&update_password) {
                        actions.push(update_password);
                    }
                    true
                }
                Self::DiscardHumanSeed => false,
            });
        }
    }

    /// Classify client-backed principals before considering the admin username.
    fn for_user(user: &UserRepresentation) -> Self {
        if user
            .service_account_client_id
            .as_deref()
            .is_some_and(|id| !id.is_empty())
        {
            Self::ServiceAccount
        } else if user.username.as_deref() == Some(ADMIN_USERNAME) {
            Self::PasswordlessAdmin
        } else {
            Self::DiscardHumanSeed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Keep legitimate client principals and admin metadata while removing human seeds.
    #[test]
    fn tenant_bootstrap_keeps_service_accounts_and_a_passwordless_admin_only() {
        let mut realm: RealmRepresentation = serde_json::from_value(json!({
            "users": [
                {"username": "admin", "enabled": true, "groups": ["/admin"], "attributes": {"custom": ["preserved"]}, "requiredActions": ["VERIFY_EMAIL"], "credentials": [{"type": "password", "value": "synthetic-password"}]},
                {"username": "custom-service-principal", "serviceAccountClientId": "custom-client", "groups": ["/admin"]},
                {"username": "api-user", "groups": ["/admin"]},
                {"username": "trustee1", "groups": ["/trustee"]},
                {"username": "trustee2", "groups": ["/trustee"]},
                {"username": "trustee3", "groups": ["/trustee"]}
            ]
        })).unwrap();
        let original_service = serde_json::to_value(&realm.users.as_ref().unwrap()[1]).unwrap();

        TenantBootstrapAdminPolicy::apply_to_realm(&mut realm);

        let users = realm.users.unwrap();
        assert_eq!(users.len(), 2);
        let admin = &users[0];
        assert_eq!(admin.username.as_deref(), Some("admin"));
        assert!(admin.credentials.is_none());
        assert_eq!(
            admin.required_actions.as_deref(),
            Some(["VERIFY_EMAIL".into(), "UPDATE_PASSWORD".into()].as_slice())
        );
        assert_eq!(admin.groups.as_deref(), Some(["/admin".into()].as_slice()));
        assert_eq!(
            admin.attributes.as_ref().unwrap().get("custom").unwrap(),
            &["preserved"]
        );
        assert_eq!(serde_json::to_value(&users[1]).unwrap(), original_service);
    }

    /// Repeated policy application preserves existing actions and accepts empty realms.
    #[test]
    fn existing_admin_actions_are_not_duplicated_and_empty_realms_are_valid() {
        let mut realm: RealmRepresentation = serde_json::from_value(json!({
            "users": [{"username": "admin", "requiredActions": ["UPDATE_PASSWORD", "VERIFY_EMAIL"]}]
        }))
        .unwrap();
        let before = serde_json::to_value(&realm).unwrap();
        TenantBootstrapAdminPolicy::apply_to_realm(&mut realm);
        TenantBootstrapAdminPolicy::apply_to_realm(&mut realm);
        assert_eq!(serde_json::to_value(realm).unwrap(), before);
        TenantBootstrapAdminPolicy::apply_to_realm(&mut RealmRepresentation::default());
    }

    /// A real client binding remains valid even when its username resembles a seed.
    #[test]
    fn service_account_binding_takes_precedence_over_a_human_demo_username() {
        let mut realm: RealmRepresentation = serde_json::from_value(json!({
            "users": [{"username": "api-user", "serviceAccountClientId": "custom-client"}]
        }))
        .unwrap();
        let before = serde_json::to_value(&realm).unwrap();
        TenantBootstrapAdminPolicy::apply_to_realm(&mut realm);
        assert_eq!(serde_json::to_value(realm).unwrap(), before);
    }
}
