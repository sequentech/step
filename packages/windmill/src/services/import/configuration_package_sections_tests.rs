// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn realm_roles() -> BTreeSet<String> {
    set(&["election-event-read", "election-event-write", "tally"])
}

#[test]
fn the_roles_file_is_read_as_the_importer_reads_it() {
    let rows = read_group_roles(b"role,permissions\nadmin,tally|election-event-read\nempty,\n").unwrap();
    assert_eq!(
        rows,
        vec![
            GroupRoles {
                group: "admin".into(),
                roles: set(&["election-event-read", "tally"])
            },
            GroupRoles {
                group: "empty".into(),
                roles: set(&[])
            },
        ]
    );
}

#[test]
fn a_listed_group_gets_exactly_its_roles() {
    let wanted = vec![
        GroupRoles {
            group: "admin".into(),
            roles: set(&["election-event-read", "tally"]),
        },
        GroupRoles {
            group: "auditor".into(),
            roles: set(&["election-event-read"]),
        },
    ];
    let current = BTreeMap::from([
        ("admin".to_string(), set(&["election-event-read", "election-event-write"])),
        ("unlisted".to_string(), set(&["tally"])),
    ]);
    let preview = role_changes(&wanted, &current, &realm_roles());
    assert!(preview.needs_apply);
    assert!(preview.blockers.is_empty());
    assert_eq!(
        preview.changes,
        json!([
            {"group": "admin", "action": "update", "add": ["tally"], "remove": ["election-event-write"]},
            {"group": "auditor", "action": "create", "add": ["election-event-read"], "remove": []},
        ])
    );
}

#[test]
fn matching_groups_need_nothing() {
    let wanted = vec![GroupRoles {
        group: "admin".into(),
        roles: set(&["tally"]),
    }];
    let current = BTreeMap::from([("admin".to_string(), set(&["tally"]))]);
    let preview = role_changes(&wanted, &current, &realm_roles());
    assert!(!preview.needs_apply);
    assert_eq!(preview.changes[0]["action"], "unchanged");
}

#[test]
fn an_unknown_role_blocks_before_any_group_changes() {
    // The importer would fail on this row after changing earlier groups; the
    // preview stops the whole section first.
    let wanted = vec![GroupRoles {
        group: "admin".into(),
        roles: set(&["tally", "not-a-role"]),
    }];
    let preview = role_changes(&wanted, &BTreeMap::new(), &realm_roles());
    assert_eq!(
        preview.blockers,
        vec!["group 'admin' requests roles the realm does not define: not-a-role"]
    );
}

#[test]
fn tenant_settings_compare_only_the_fields_the_importer_applies() {
    let wanted = json!({"displayName": "Commission"});
    let same = tenant_settings_changes(Some("Commission"), None, &wanted);
    assert!(!same.needs_apply);
    let other = tenant_settings_changes(Some("Old"), None, &wanted);
    assert!(other.needs_apply);
    assert_eq!(
        other.changes,
        json!([{"field": "displayName", "action": "update", "from": "Old", "to": "Commission"}])
    );
}

#[test]
fn existing_administrators_are_left_alone_and_only_absent_ones_created() {
    let csv = b"username,email,first_name,last_name,group_name\nOne,,,,admin\ntwo,,,,admin\n";
    let rows = read_admin_rows(csv).unwrap();
    let existing = set(&["one"]);
    let preview = admin_changes(&rows, &existing, &set(&["admin"]));
    assert!(preview.needs_apply);
    assert_eq!(preview.changes[0]["action"], "exists");
    assert_eq!(preview.changes[1]["action"], "create");
    assert_eq!(
        String::from_utf8(absent_admin_rows(csv, &existing).unwrap()).unwrap(),
        "username,email,first_name,last_name,group_name\ntwo,,,,admin\n"
    );
    // Converged once both exist.
    assert!(!admin_changes(&rows, &set(&["one", "two"]), &set(&["admin"])).needs_apply);
}

#[test]
fn an_administrator_in_an_undefined_group_blocks() {
    let rows = read_admin_rows(b"username,group_name\na,ghost\n").unwrap();
    let preview = admin_changes(&rows, &set(&[]), &set(&["admin"]));
    assert_eq!(preview.blockers.len(), 1);
    assert!(preview.blockers[0].contains("ghost"));
}

fn native_trustee(name: &str) -> TrusteePublic {
    let keys = protocol_board::TrusteeSecrets::generate().public_keys().unwrap();
    TrusteePublic {
        name: name.into(),
        signing_public_key: keys.signing_public_key,
        share_encryption_public_key: keys.share_encryption_public_key,
    }
}

fn registered(trustee: &TrusteePublic) -> Trustee {
    serde_json::from_value(json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "tenant_id": "00000000-0000-4000-8000-000000000002",
        "name": trustee.name,
        "public_key": trustee.signing_public_key,
        "share_encryption_public_key": trustee.share_encryption_public_key,
    }))
    .unwrap()
}

#[test]
fn trustees_are_registered_once_and_never_rekeyed() {
    let a = native_trustee("a");
    let b = native_trustee("b");
    let fresh = trustee_changes(&[a.clone(), b.clone()], &[]);
    assert!(fresh.needs_apply);
    assert!(fresh.blockers.is_empty(), "{:?}", fresh.blockers);
    let done = trustee_changes(&[a.clone(), b.clone()], &[registered(&a), registered(&b)]);
    assert!(!done.needs_apply);
    // Same name, other keys: a conflict, not an update.
    let other = native_trustee("a");
    let conflict = trustee_changes(&[a], &[registered(&other)]);
    assert_eq!(conflict.changes[0]["action"], "conflict");
    assert_eq!(conflict.blockers.len(), 1);
}

#[test]
fn structurally_valid_but_non_native_keys_block() {
    let fake = TrusteePublic {
        name: "a".into(),
        signing_public_key: "AAAA".into(),
        share_encryption_public_key: "AAAA".into(),
    };
    let preview = trustee_changes(&[fake], &[]);
    assert!(preview.blockers[0].starts_with("trustee public keys are not valid native keys"));
}

#[test]
fn an_imported_event_is_not_imported_again() {
    assert!(event_changes("e1", false, vec![]).needs_apply);
    let existing = event_changes("e1", true, vec![]);
    assert!(!existing.needs_apply);
    assert_eq!(existing.changes[0]["action"], "exists");
}
