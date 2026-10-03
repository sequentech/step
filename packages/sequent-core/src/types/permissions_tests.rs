// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::Value;
use std::collections::BTreeSet;
use std::str::FromStr;

/// The realms administrators log in to: the template every new tenant
/// realm is made from, and the one the janitor writes for a deployment.
/// Embedded, so a moved file fails the build rather than the test.
const REALMS: [(&str, &str); 2] = [
    (
        "tenant realm template",
        include_str!("../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"),
    ),
    (
        "COMELEC realm template",
        include_str!("../../../windmill/external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs"),
    ),
];

/// The groups whose realm roles include `permission`.
fn groups_granting(realm: &Value, permission: &str) -> BTreeSet<String> {
    realm["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|group| {
            group["realmRoles"].as_array().is_some_and(|roles| {
                roles.iter().any(|role| role == permission)
            })
        })
        .filter_map(|group| group["name"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn monitoring_is_viewed_and_configured_under_its_own_permissions() {
    for (name, permission) in [
        ("monitoring-view", Permissions::MONITORING_VIEW),
        ("monitoring-configure", Permissions::MONITORING_CONFIGURE),
    ] {
        assert_eq!(Permissions::from_str(name), Ok(permission.clone()));
        assert_eq!(permission.to_string(), name);
    }
}

/// Administrators see and configure the dashboards; a light administrator
/// only sees them; no other group does either.
#[test]
fn every_realm_grants_monitoring_to_its_administrators_only() {
    let view = Permissions::MONITORING_VIEW.to_string();
    let configure = Permissions::MONITORING_CONFIGURE.to_string();
    for (name, text) in REALMS {
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let roles: Vec<&str> = realm["roles"]["realm"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role["name"].as_str())
            .collect();
        for permission in [&view, &configure] {
            assert!(
                roles.contains(&permission.as_str()),
                "{name}: no {permission} role"
            );
        }
        let names = |groups: &[&str]| -> BTreeSet<String> {
            groups.iter().map(|group| group.to_string()).collect()
        };
        assert_eq!(
            groups_granting(&realm, &view),
            names(&["admin", "admin-light"]),
            "{name}: {view}"
        );
        assert_eq!(
            groups_granting(&realm, &configure),
            names(&["admin"]),
            "{name}: {configure}"
        );
    }
}

#[test]
fn messaging_permissions_have_stable_names() {
    for (name, permission) in [
        (
            "messaging-account-read",
            Permissions::MESSAGING_ACCOUNT_READ,
        ),
        (
            "messaging-account-write",
            Permissions::MESSAGING_ACCOUNT_WRITE,
        ),
        (
            "messaging-config-write",
            Permissions::MESSAGING_CONFIG_WRITE,
        ),
    ] {
        assert_eq!(Permissions::from_str(name), Ok(permission.clone()));
        assert_eq!(permission.to_string(), name);
    }
}

/// Administrators manage sending accounts and event messaging; a light
/// administrator may only see the accounts.
#[test]
fn every_realm_grants_messaging_to_its_administrators_only() {
    let names = |groups: &[&str]| -> BTreeSet<String> {
        groups.iter().map(|group| group.to_string()).collect()
    };
    for (name, text) in REALMS {
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let roles: Vec<&str> = realm["roles"]["realm"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role["name"].as_str())
            .collect();
        for (permission, groups) in [
            (
                Permissions::MESSAGING_ACCOUNT_READ,
                names(&["admin", "admin-light"]),
            ),
            (Permissions::MESSAGING_ACCOUNT_WRITE, names(&["admin"])),
            (Permissions::MESSAGING_CONFIG_WRITE, names(&["admin"])),
        ] {
            let permission = permission.to_string();
            assert!(
                roles.contains(&permission.as_str()),
                "{name}: no {permission} role"
            );
            assert_eq!(
                groups_granting(&realm, &permission),
                groups,
                "{name}: {permission}"
            );
        }
    }
}
