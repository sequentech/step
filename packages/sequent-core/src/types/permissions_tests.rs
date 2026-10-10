// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::Value;
use std::collections::BTreeSet;
use std::str::FromStr;
use strum::IntoEnumIterator;

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

/// Only administrators save new versions of the enrollment approval matrix.
#[test]
fn every_realm_grants_the_approval_matrix_to_its_administrators_only() {
    let write = Permissions::APPROVAL_MATRIX_WRITE.to_string();
    assert_eq!(write, "approval-matrix-write");
    assert_eq!(
        Permissions::from_str(&write),
        Ok(Permissions::APPROVAL_MATRIX_WRITE)
    );
    for (name, text) in REALMS {
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        assert!(
            realm["roles"]["realm"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|role| role["name"] == write.as_str()),
            "{name}: no {write} role"
        );
        assert_eq!(
            groups_granting(&realm, &write),
            BTreeSet::from(["admin".to_string()]),
            "{name}: {write}"
        );
    }
}

/// The permissions of Election Event > Signatures and of signing each
/// protected action, with their realm role names.
const SIGNING_PERMISSIONS: [(&str, Permissions); 21] = [
    (
        "election-event-signatures-tab",
        Permissions::ELECTION_EVENT_SIGNATURES_TAB,
    ),
    ("signing-rules-read", Permissions::SIGNING_RULES_READ),
    ("signing-rules-write", Permissions::SIGNING_RULES_WRITE),
    (
        "signing-certificates-read",
        Permissions::SIGNING_CERTIFICATES_READ,
    ),
    ("signing-issuers-write", Permissions::SIGNING_ISSUERS_WRITE),
    ("signing-checks-write", Permissions::SIGNING_CHECKS_WRITE),
    (
        "signing-certificates-register",
        Permissions::SIGNING_CERTIFICATES_REGISTER,
    ),
    (
        "signing-certificates-revoke",
        Permissions::SIGNING_CERTIFICATES_REVOKE,
    ),
    ("signing-requests-read", Permissions::SIGNING_REQUESTS_READ),
    (
        "signing-requests-cancel",
        Permissions::SIGNING_REQUESTS_CANCEL,
    ),
    (
        "signing-requests-export",
        Permissions::SIGNING_REQUESTS_EXPORT,
    ),
    (
        "sign-initialize-voting",
        Permissions::SIGN_INITIALIZE_VOTING,
    ),
    ("sign-open-voting", Permissions::SIGN_OPEN_VOTING),
    ("sign-close-voting", Permissions::SIGN_CLOSE_VOTING),
    (
        "sign-generate-election-returns",
        Permissions::SIGN_GENERATE_ELECTION_RETURNS,
    ),
    ("sign-generate-reports", Permissions::SIGN_GENERATE_REPORTS),
    ("sign-transmit-results", Permissions::SIGN_TRANSMIT_RESULTS),
    ("sign-approve-voter", Permissions::SIGN_APPROVE_VOTER),
    (
        "sign-approve-configuration",
        Permissions::SIGN_APPROVE_CONFIGURATION,
    ),
    ("sign-key-ceremony", Permissions::SIGN_KEY_CEREMONY),
    ("sign-tally-key", Permissions::SIGN_TALLY_KEY),
];

#[test]
fn signing_permissions_have_their_realm_role_names() {
    for (name, permission) in SIGNING_PERMISSIONS {
        assert_eq!(Permissions::from_str(name), Ok(permission.clone()));
        assert_eq!(permission.to_string(), name);
    }
}

/// The roles of a realm template's `roles.realm`, by name.
fn realm_roles(realm: &Value) -> Vec<&str> {
    realm["roles"]["realm"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| role["name"].as_str())
        .collect()
}

/// A group's realm roles.
fn group_roles(realm: &Value, group: &str) -> Option<BTreeSet<String>> {
    realm["groups"]
        .as_array()?
        .iter()
        .find(|candidate| candidate["name"] == group)?["realmRoles"]
        .as_array()
        .map(|roles| {
            roles
                .iter()
                .filter_map(|role| role.as_str().map(str::to_string))
                .collect()
        })
}

/// Every signing permission, for a group that holds them all.
const ALL_SIGNING: &[&str] = &["*"];

/// The groups of each realm template that hold signing permissions, and
/// which. Groups not listed hold none. Every new tenant realm gives them to
/// its administrators only; the COMELEC template assigns the ticket's
/// preset to its own groups (tenant data, not product code).
fn signing_presets(
) -> [(&'static str, Vec<(&'static str, &'static [&'static str])>); 2] {
    let sbei: &'static [&'static str] = &[
        "sign-initialize-voting",
        "sign-open-voting",
        "sign-close-voting",
        "sign-generate-election-returns",
        "sign-generate-reports",
        "sign-transmit-results",
        "sign-approve-voter",
    ];
    [
        ("tenant realm template", vec![("admin", ALL_SIGNING)]),
        (
            "COMELEC realm template",
            vec![
                ("admin", ALL_SIGNING),
                (
                    "configuration-manager",
                    &[
                        "election-event-signatures-tab",
                        "signing-rules-read",
                        "signing-rules-write",
                        "sign-approve-configuration",
                    ],
                ),
                (
                    "security-officer",
                    &[
                        "election-event-signatures-tab",
                        "signing-certificates-read",
                        "signing-issuers-write",
                        "signing-checks-write",
                        "signing-certificates-register",
                        "signing-certificates-revoke",
                        "sign-approve-configuration",
                    ],
                ),
                (
                    "ofov",
                    &[
                        "election-event-signatures-tab",
                        "signing-requests-read",
                        "signing-requests-cancel",
                        "signing-requests-export",
                        "sign-approve-voter",
                    ],
                ),
                (
                    "auditor",
                    &[
                        "election-event-signatures-tab",
                        "signing-rules-read",
                        "signing-certificates-read",
                        "signing-requests-read",
                        "signing-requests-export",
                    ],
                ),
                ("sbei", sbei),
                ("trustee", &["sign-key-ceremony", "sign-tally-key"]),
            ],
        ),
    ]
}

/// Each realm template has every signing permission once, and exactly the
/// groups its preset names hold each one. The expectation comes from the
/// preset table, so a template that gave one group's preset to another
/// (or the same preset to every realm) fails.
#[test]
fn every_realm_template_grants_signing_by_its_preset() {
    for ((name, text), (preset_name, preset)) in
        REALMS.into_iter().zip(signing_presets())
    {
        assert_eq!(name, preset_name);
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let roles = realm_roles(&realm);
        for (permission, _) in SIGNING_PERMISSIONS {
            assert_eq!(
                roles.iter().filter(|role| **role == permission).count(),
                1,
                "{name}: {permission} role"
            );
            let expected: BTreeSet<String> = preset
                .iter()
                .filter(|(_, held)| {
                    *held == ALL_SIGNING || held.contains(&permission)
                })
                .map(|(group, _)| group.to_string())
                .collect();
            assert_eq!(
                groups_granting(&realm, permission),
                expected,
                "{name}: {permission}"
            );
        }
        for (group, held) in &preset {
            assert!(
                group_roles(&realm, group).is_some(),
                "{name}: no {group} group"
            );
            for permission in held.iter().filter(|held| **held != "*") {
                assert!(
                    SIGNING_PERMISSIONS
                        .iter()
                        .any(|(known, _)| known == permission),
                    "{name}: {group} preset names unknown {permission}"
                );
            }
        }
    }
}

/// What the Security Officer holds beyond its Signatures preset, to change
/// who signs in Users and Roles: the menu entry, the Roles tab
/// (`role-read`), the permission grid (`user-permission-read`) and
/// `role-write`, which changes only the sign permissions without
/// `user-permission-write`; and the user list that registering a
/// certificate picks the person from (`user-read`).
const SECURITY_OFFICER_EXTRA: [&str; 5] = [
    "users-menu",
    "role-read",
    "user-permission-read",
    "role-write",
    "user-read",
];

/// What the Configuration Manager holds beyond its Signatures preset: the
/// role list (`role-read`) the rule editor picks the signing roles from.
const CONFIGURATION_MANAGER_EXTRA: [&str; 1] = ["role-read"];

/// What every group the COMELEC template adds for the Signatures tab reads:
/// election events (`election-event-read`), and the Posts (`election-read`)
/// and countries (`area-read`) the tab names rules and requests by.
const SIGNATURE_GROUP_BASELINE: [&str; 3] =
    ["election-event-read", "election-read", "area-read"];

/// The groups the COMELEC template adds for the Signatures tab hold their
/// preset, [`SIGNATURE_GROUP_BASELINE`], and for the Configuration Manager
/// [`CONFIGURATION_MANAGER_EXTRA`] and the Security Officer
/// [`SECURITY_OFFICER_EXTRA`]; nothing else. In particular
/// none holds `admin-user`, Hasura's tenant-wide role with writes, nor
/// `user-permission-write`, with which the Security Officer could grant
/// itself any permission.
#[test]
fn the_comelec_signature_groups_hold_their_preset_and_the_event_baseline() {
    let (name, text) = REALMS[1];
    let realm: Value = serde_json::from_str(text)
        .unwrap_or_else(|why| panic!("{name}: {why}"));
    let baseline = SIGNATURE_GROUP_BASELINE;
    let [_, (_, preset)] = signing_presets();
    for (group, extra) in [
        ("configuration-manager", &CONFIGURATION_MANAGER_EXTRA[..]),
        ("security-officer", &SECURITY_OFFICER_EXTRA[..]),
        ("ofov", &[][..]),
        ("auditor", &[][..]),
    ] {
        let (_, held) = preset
            .iter()
            .find(|(candidate, _)| *candidate == group)
            .unwrap_or_else(|| panic!("{name}: no {group} preset"));
        let expected: BTreeSet<String> = baseline
            .iter()
            .chain(extra)
            .chain(held.iter())
            .map(|role| role.to_string())
            .collect();
        let roles = group_roles(&realm, group);
        for forbidden in ["admin-user", "user-permission-write"] {
            assert!(
                !roles.iter().flatten().any(|role| role == forbidden),
                "{name}: {group} holds {forbidden}"
            );
        }
        assert_eq!(roles, Some(expected), "{name}: {group}");
        let group_json = realm["groups"]
            .as_array()
            .and_then(|groups| {
                groups.iter().find(|candidate| candidate["name"] == group)
            })
            .unwrap_or_else(|| panic!("{name}: no {group} group"));
        assert_eq!(group_json["path"], format!("/{group}"), "{name}: {group}");
    }
    // Every role those groups hold exists in the realm. Besides the signing
    // permissions, they are roles every tenant realm already has, so the
    // migration of existing realms, which adds only the signing roles, needs
    // none of them.
    let roles = realm_roles(&realm);
    let (template_name, template_text) = REALMS[0];
    let template: Value = serde_json::from_str(template_text)
        .unwrap_or_else(|why| panic!("{template_name}: {why}"));
    let template_roles = realm_roles(&template);
    for role in baseline
        .iter()
        .chain(&CONFIGURATION_MANAGER_EXTRA)
        .chain(&SECURITY_OFFICER_EXTRA)
    {
        assert!(roles.contains(role), "{name}: no {role} role");
        assert!(
            template_roles.contains(role),
            "{template_name}: no {role} role"
        );
    }
}

/// Keycloak refuses an import whose ids repeat; new roles and groups need
/// fresh ones.
#[test]
fn realm_template_role_and_group_ids_are_unique() {
    for (name, text) in REALMS {
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        for (kind, items) in [
            ("role", &realm["roles"]["realm"]),
            ("group", &realm["groups"]),
        ] {
            let ids: Vec<&str> = items
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["id"].as_str())
                .collect();
            let unique: BTreeSet<&str> = ids.iter().copied().collect();
            assert_eq!(ids.len(), unique.len(), "{name}: repeated {kind} id");
            let names: Vec<&str> = items
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["name"].as_str())
                .collect();
            let unique: BTreeSet<&str> = names.iter().copied().collect();
            assert_eq!(
                names.len(),
                unique.len(),
                "{name}: repeated {kind} name"
            );
        }
    }
}

/// The COMELEC SBEIs start the Post actions they sign in Post > Publish
/// (the ticket's "Started in"): Initialize voting (`admin-ceremony`, the
/// initialization report), Open voting (Start voting) and Close voting
/// (Stop voting). Publish shows the Start and Stop voting buttons only with
/// `publish-start-voting` and `publish-stop-voting`, and the Post's status
/// route needs `election-state-write`. Pausing is not a signed action, and
/// the group stays without anything more.
#[test]
fn the_comelec_sbei_group_starts_the_post_actions_it_signs() {
    let (name, text) = REALMS[1];
    let realm: Value = serde_json::from_str(text)
        .unwrap_or_else(|why| panic!("{name}: {why}"));
    let roles = group_roles(&realm, "sbei")
        .unwrap_or_else(|| panic!("{name}: no sbei group"));
    for permission in [
        Permissions::ADMIN_CEREMONY,
        Permissions::PUBLISH_START_VOTING,
        Permissions::PUBLISH_STOP_VOTING,
        Permissions::ELECTION_STATE_WRITE,
    ] {
        assert!(
            roles.contains(&permission.to_string()),
            "{name}: sbei lacks {permission}"
        );
    }
    assert!(
        !roles.contains(&Permissions::PUBLISH_PAUSE_VOTING.to_string()),
        "{name}: sbei pauses voting"
    );
    let realm_roles = realm_roles(&realm);
    for role in &roles {
        assert!(
            realm_roles.contains(&role.as_str()),
            "{name}: no {role} role"
        );
    }
}

/// The user profile of a realm template, as Keycloak stores it.
fn user_profile(realm: &Value) -> Option<Value> {
    let config = realm["components"]
        ["org.keycloak.userprofile.UserProfileProvider"]
        .as_array()?
        .first()?["config"]["kc.user.profile.config"]
        .as_array()?
        .first()?
        .as_str()?;
    serde_json::from_str(config).ok()
}

/// A signer's `title` ("Chairperson") is an optional user-profile
/// attribute of every realm administrators log in to, so the Keycloak
/// console and admin API keep it: administrators edit it, users don't, and
/// nobody has to fill it in. The attributes the realms had stay.
#[test]
fn realm_templates_declare_the_optional_signer_title() {
    for (name, text) in REALMS {
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let profile = user_profile(&realm)
            .unwrap_or_else(|| panic!("{name}: no user profile"));
        let attributes = profile["attributes"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: no attributes"));
        let named = |wanted: &str| {
            attributes
                .iter()
                .find(|attribute| attribute["name"] == wanted)
                .cloned()
        };
        let title = named("title")
            .unwrap_or_else(|| panic!("{name}: no title attribute"));
        assert!(title.get("required").is_none(), "{name}: title is required");
        assert_eq!(title["multivalued"], false, "{name}");
        assert_eq!(
            title["permissions"]["edit"],
            serde_json::json!(["admin"]),
            "{name}: only administrators edit the title"
        );
        assert!(
            title["permissions"]["view"]
                .as_array()
                .is_some_and(|view| view.iter().any(|who| who == "admin")),
            "{name}: administrators see the title"
        );
        for kept in [
            "username",
            "email",
            "firstName",
            "lastName",
            "tenant-id",
            "trustee",
            "permission_labels",
        ] {
            assert!(named(kept).is_some(), "{name}: lost {kept}");
        }
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

const RESERVED_NAMES: [&str; 5] = [
    "admin",
    "service-account",
    "datafix-account",
    "super-admin-user",
    "cli-account-admin",
];

/// Every realm role the realm defines or grants to one of its groups.
fn realm_role_names(realm: &Value) -> Vec<&str> {
    let mut names: Vec<&str> = realm["roles"]["realm"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| role["name"].as_str())
        .collect();
    let mut groups: Vec<&Value> =
        realm["groups"].as_array().into_iter().flatten().collect();
    while let Some(group) = groups.pop() {
        names.extend(
            group["realmRoles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        );
        groups.extend(group["subGroups"].as_array().into_iter().flatten());
    }
    names
}

#[test]
fn reserved_roles_are_the_platform_managed_names() {
    let names: Vec<String> = ReservedHasuraRole::iter()
        .map(|role| role.to_string())
        .collect();

    assert_eq!(names, RESERVED_NAMES);
}

#[test]
fn reserved_names_are_rejected_in_any_case_with_surrounding_whitespace() {
    for name in RESERVED_NAMES {
        for spelling in
            [name.to_string(), name.to_uppercase(), format!(" {name}\n")]
        {
            assert!(
                matches!(
                    RealmRolePolicy::classify(&spelling),
                    RealmRolePolicy::Reserved(_)
                ),
                "{spelling:?}"
            );
            let error =
                RealmRolePolicy::require_ordinary(&spelling).unwrap_err();
            assert!(error.to_string().contains(name), "{error}");
        }
    }
}

#[test]
fn platform_account_permissions_are_reserved() {
    for permission in
        [Permissions::SERVICE_ACCOUNT, Permissions::DATAFIX_ACCOUNT]
    {
        assert!(
            RealmRolePolicy::require_ordinary(&permission.to_string()).is_err()
        );
    }
}

#[test]
fn other_names_are_ordinary() {
    for name in [
        "admin-user",
        "admin-light",
        "service-account-reader",
        "super-admin",
        "cli-account",
        "election-event-read",
        "user",
        "",
        "  ",
    ] {
        assert_eq!(
            RealmRolePolicy::classify(name),
            RealmRolePolicy::Ordinary,
            "{name:?}"
        );
        assert!(RealmRolePolicy::require_ordinary(name).is_ok());
    }
    assert!(RealmRolePolicy::require_ordinary(
        &Permissions::ADMIN_USER.to_string()
    )
    .is_ok());
}

#[test]
fn every_realm_template_only_holds_ordinary_roles() {
    for (name, text) in REALMS {
        let realm: Value = serde_json::from_str(text)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let names = realm_role_names(&realm);

        assert!(names.len() > 100, "{name}");
        for role in names {
            assert_eq!(
                RealmRolePolicy::classify(role),
                RealmRolePolicy::Ordinary,
                "{name}: {role}"
            );
        }
    }
}
