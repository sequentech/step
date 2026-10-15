// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The readiness operator permission must be provisioned on new and deployed
//! tenants and must remain tenant scoped at the authorization boundary.
use sequent_core::types::permissions::Permissions;
use serde_json::Value;

const PERMISSION: &str = "readiness-check";

#[test]
fn readiness_permission_is_named_and_provisioned_for_administrators() {
    let permission: Permissions = PERMISSION
        .parse()
        .expect("readiness-check permission exists");
    assert_eq!(permission.to_string(), PERMISSION);
    for (name, text) in [
        ("tenant realm", include_str!("../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json")),
        ("deployed realm", include_str!("../../windmill/external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs")),
    ] {
        let realm: Value = serde_json::from_str(text).unwrap();
        assert!(realm["roles"]["realm"].as_array().unwrap().iter().any(|role| role["name"] == PERMISSION), "{name}: missing readiness role");
        let groups: Vec<&str> = realm["groups"].as_array().unwrap().iter().filter(|group| {
            group["realmRoles"].as_array().is_some_and(|roles| roles.iter().any(|role| role == PERMISSION))
        }).map(|group| group["name"].as_str().unwrap()).collect();
        assert_eq!(groups, ["admin"], "{name}: readiness is an operator action");
    }
}

#[cfg(all(feature = "default_features", feature = "keycloak"))]
#[path = "support/claims.rs"]
#[allow(dead_code)]
mod claims;

#[cfg(all(feature = "default_features", feature = "keycloak"))]
#[test]
fn readiness_requires_its_role_and_the_matching_tenant() {
    use sequent_core::services::authorization::authorize;
    let permission: Permissions = PERMISSION
        .parse()
        .expect("readiness-check permission exists");
    let operator = claims::Claims::new("tenant-a", "operator")
        .roles([PERMISSION])
        .build();
    assert!(authorize(
        &operator,
        false,
        Some("tenant-a".into()),
        vec![permission.clone()]
    )
    .is_ok());
    assert!(authorize(
        &operator,
        false,
        Some("tenant-b".into()),
        vec![permission.clone()]
    )
    .is_err());
    let observer = claims::Claims::new("tenant-a", "observer")
        .roles(["election-event-read"])
        .build();
    assert!(authorize(
        &observer,
        false,
        Some("tenant-a".into()),
        vec![permission]
    )
    .is_err());
}
