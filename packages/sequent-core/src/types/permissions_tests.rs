// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;

/// The realms administrators log in to: the development tenant and the one
/// the janitor writes for a deployment.
const REALMS: [&str; 2] = [
    "../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json",
    "windmill/external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs",
];

fn realm(path: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|why| panic!("{}: {why}", path.display()))
}

fn group_roles(realm: &Value, group: &str) -> Vec<String> {
    realm["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|candidate| candidate["name"] == group)
        .unwrap_or_else(|| panic!("no {group} group"))["realmRoles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| role.as_str().map(str::to_string))
        .collect()
}

#[test]
fn monitoring_is_viewed_and_configured_under_its_own_permissions() {
    assert_eq!(
        Permissions::from_str("monitoring-view"),
        Ok(Permissions::MONITORING_VIEW)
    );
    assert_eq!(
        Permissions::from_str("monitoring-configure"),
        Ok(Permissions::MONITORING_CONFIGURE)
    );
    assert_eq!(Permissions::MONITORING_VIEW.to_string(), "monitoring-view");
}

/// Administrators see and configure the dashboards; a light administrator
/// only sees them.
#[test]
fn every_realm_grants_monitoring_to_its_administrators() {
    let view = Permissions::MONITORING_VIEW.to_string();
    let configure = Permissions::MONITORING_CONFIGURE.to_string();
    for path in REALMS {
        let realm = realm(path);
        let roles: Vec<&str> = realm["roles"]["realm"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role["name"].as_str())
            .collect();
        for permission in [&view, &configure] {
            assert!(
                roles.contains(&permission.as_str()),
                "{path}: no {permission} role"
            );
        }
        let admin = group_roles(&realm, "admin");
        assert!(
            admin.contains(&view) && admin.contains(&configure),
            "{path}: admin"
        );
        let light = group_roles(&realm, "admin-light");
        assert!(
            light.contains(&view) && !light.contains(&configure),
            "{path}: admin-light"
        );
    }
}
