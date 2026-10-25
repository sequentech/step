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
    let rows =
        read_group_roles(b"role,permissions\nadmin,tally|election-event-read\nempty,\n").unwrap();
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
        (
            "admin".to_string(),
            set(&["election-event-read", "election-event-write"]),
        ),
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

fn members(entries: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
    entries
        .iter()
        .map(|(user, groups)| (user.to_string(), set(groups)))
        .collect()
}

#[test]
fn existing_administrators_are_left_alone_and_only_absent_ones_created() {
    let csv = b"username,email,first_name,last_name,group_name\nOne,,,,admin\ntwo,,,,admin\n";
    let rows = read_admin_rows(csv).unwrap();
    let existing = members(&[("one", &["admin"])]);
    let preview = admin_changes(&rows, &existing, &set(&["admin"]));
    assert!(preview.needs_apply);
    assert!(preview.blockers.is_empty());
    assert_eq!(preview.changes[0]["action"], "exists");
    assert_eq!(preview.changes[1]["action"], "create");
    assert_eq!(
        String::from_utf8(absent_admin_rows(csv, &existing).unwrap()).unwrap(),
        "username,email,first_name,last_name,group_name\ntwo,,,,admin\n"
    );
    // Converged once both exist in their group.
    let both = members(&[("one", &["admin"]), ("two", &["admin", "other"])]);
    assert!(!admin_changes(&rows, &both, &set(&["admin"])).needs_apply);
}

#[test]
fn an_existing_account_outside_its_group_blocks_instead_of_counting_as_applied() {
    // An earlier import created `one` but did not assign it, or the name
    // belongs to someone else: neither is in place, neither is changed.
    let rows = read_admin_rows(b"username,group_name\none,admin\n").unwrap();
    for groups in [&[][..], &["auditor"][..]] {
        let preview = admin_changes(
            &rows,
            &members(&[("one", groups)]),
            &set(&["admin", "auditor"]),
        );
        assert!(!preview.needs_apply, "never re-created");
        assert_eq!(preview.changes[0]["action"], "conflict");
        assert_eq!(preview.blockers.len(), 1);
        assert!(preview.blockers[0].contains("not in group 'admin'"));
    }
}

#[test]
fn an_administrator_in_an_undefined_group_blocks() {
    let rows = read_admin_rows(b"username,group_name\na,ghost\n").unwrap();
    let preview = admin_changes(&rows, &BTreeMap::new(), &set(&["admin"]));
    assert_eq!(preview.blockers.len(), 1);
    assert!(preview.blockers[0].contains("ghost"));
}

fn native_trustee(name: &str) -> TrusteePublic {
    let keys = protocol_board::TrusteeSecrets::generate()
        .public_keys()
        .unwrap();
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
fn an_imported_event_is_confirmed_with_its_census_and_a_partial_one_blocks() {
    let census = set(&["a", "b", "c"]);
    let absent = event_changes("e1", &EventState::Absent, &census, vec![]);
    assert!(absent.needs_apply);
    assert!(absent.blockers.is_empty());
    let done = event_changes(
        "e1",
        &EventState::Present {
            voters: census.clone(),
        },
        &census,
        vec![],
    );
    assert!(!done.needs_apply);
    assert!(done.blockers.is_empty());
    assert_eq!(done.changes[0]["action"], "exists");
    // Same count, other people: still a difference.
    let swapped = event_changes(
        "e1",
        &EventState::Present {
            voters: set(&["a", "b", "x"]),
        },
        &census,
        vec![],
    );
    assert!(!swapped.needs_apply, "never imported twice");
    assert_eq!(swapped.changes[0]["action"], "census-differs");
    assert!(swapped.blockers[0].contains("1 of the census are not in the event and 1 people"));
    let partial = event_changes("e1", &EventState::RealmOnly, &census, vec![]);
    assert!(!partial.needs_apply);
    assert_eq!(partial.blockers.len(), 1);
    // The event's own preflight problems block an import that is otherwise due.
    let invalid = event_changes("e1", &EventState::Absent, &census, vec!["bad".into()]);
    assert_eq!(invalid.blockers, vec!["bad"]);
}

fn event_zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in members {
        out.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}

#[test]
fn the_census_is_read_as_the_importer_finds_it_and_ambiguity_is_refused() {
    let one = event_zip(&[
        ("export_election_event-e1.json", b"{}"),
        ("export_voters-e1.csv", b"area,username\nx,Ada\nx,bob\n"),
    ]);
    assert_eq!(census_usernames(&one).unwrap(), Ok(set(&["ada", "bob"])));
    // The importer reads a file under export_S3_files/export_voters/ as census
    // too, so with the CSV beside it the census is ambiguous.
    let with_s3 = event_zip(&[
        ("export_voters-e1.csv", b"username\na\n"),
        ("export_S3_files/export_voters/photo.png", b"png"),
    ]);
    assert!(census_usernames(&with_s3).unwrap().is_err());
    let none = event_zip(&[("export_election_event-e1.json", b"{}")]);
    assert_eq!(census_usernames(&none).unwrap(), Ok(set(&[])));
    // The importer reads every member containing the family, any extension.
    let two = event_zip(&[
        ("export_voters-e1.csv", b"username\na\n"),
        ("more_export_voters.tsv", b"username\nb\n"),
    ]);
    let refused = census_usernames(&two).unwrap().unwrap_err();
    assert!(refused.starts_with("the event carries 2 census files"));
    let tsv = event_zip(&[("export_voters-e1.tsv", b"username\na\n")]);
    assert!(census_usernames(&tsv).unwrap().is_err());
    assert_eq!(event_document(&one).unwrap(), b"{}");
}

#[test]
fn key_ceremony_trustees_must_resolve_to_one_current_or_planned_trustee() {
    let referenced = vec![
        "t1".to_string(),
        "t2".to_string(),
        "t3".to_string(),
        "t1".to_string(),
    ];
    let registered = vec!["t1".to_string(), "t3".to_string(), "t3".to_string()];
    let planned = set(&["t2"]);
    assert_eq!(
        trustee_reference_problems(&referenced, &registered, &planned),
        vec!["the key ceremony names trustee 't3', which 2 registered trustees share"]
    );
    let problems = trustee_reference_problems(&["t9".to_string()], &registered, &planned);
    assert_eq!(
        problems,
        vec!["the key ceremony names trustee 't9', which neither the tenant nor the package registers"]
    );
    // Control: all resolve.
    assert!(
        trustee_reference_problems(&["t1".into(), "t2".into()], &registered, &planned).is_empty()
    );
}

fn tenant() -> Tenant {
    serde_json::from_value(json!({
        "id": "00000000-0000-4000-8000-000000000002",
        "slug": "target-slug",
        "is_active": true,
        "annotations": {"css": "old", "keep": 1},
        "voting_channels": {"online": true},
        "settings": {"configuration_signing": {"policy": "required"}, "i18n": {}},
        "test": 0
    }))
    .unwrap()
}

#[test]
fn the_tenant_record_keeps_its_identity_and_unnamed_keys() {
    let wanted: TenantConfiguration = serde_json::from_value(json!({
        "annotations": {"css": "new"},
        "votingChannels": {"kiosk": true},
        "settings": {"i18n": {"en": {"x": "y"}}}
    }))
    .unwrap();
    let merged = merged_tenant(&tenant(), &wanted);
    assert_eq!(merged.id, tenant().id);
    assert_eq!(merged.slug, "target-slug");
    assert!(merged.is_active);
    assert_eq!(merged.test, Some(0));
    assert_eq!(merged.annotations, Some(json!({"css": "new", "keep": 1})));
    assert_eq!(
        merged.voting_channels,
        Some(json!({"online": true, "kiosk": true}))
    );
    // The signing trust the package does not name stays.
    assert_eq!(
        merged.settings,
        Some(json!({"configuration_signing": {"policy": "required"}, "i18n": {"en": {"x": "y"}}}))
    );
    let preview = tenant_configuration_changes(&tenant(), &wanted);
    assert!(preview.needs_apply);
    assert_eq!(
        preview.changes,
        json!([
            {"field": "annotations", "action": "update", "keys": ["css"]},
            {"field": "voting_channels", "action": "update", "keys": ["kiosk"]},
            {"field": "settings", "action": "update", "keys": ["i18n"]},
        ])
    );
    // Converged once applied.
    assert!(!tenant_configuration_changes(&merged, &wanted).needs_apply);
}

#[test]
fn a_package_may_not_change_the_tenants_signing_trust() {
    // Control: naming an equal value is no change and no blocker.
    let same: TenantConfiguration = serde_json::from_value(json!({
        "settings": {"configuration_signing": {"policy": "required"}}
    }))
    .unwrap();
    assert!(tenant_configuration_changes(&tenant(), &same)
        .blockers
        .is_empty());
    let other: TenantConfiguration = serde_json::from_value(json!({
        "settings": {"configuration_signing": {"policy": "optional"}}
    }))
    .unwrap();
    let preview = tenant_configuration_changes(&tenant(), &other);
    assert_eq!(preview.blockers.len(), 1);
    assert!(preview.blockers[0].contains("configuration_signing"));
}

#[test]
fn administrator_files_carry_only_the_producers_columns() {
    let csv = b"username,email,group_name,password,hashed_password\na,,admin,secret,h\n";
    assert_eq!(
        admin_column_problems(csv).unwrap(),
        vec![
            "the administrators file carries column 'password', which a package may not set",
            "the administrators file carries column 'hashed_password', which a package may not set",
        ]
    );
    // Even so, the import file never carries them.
    let written = String::from_utf8(absent_admin_rows(csv, &BTreeMap::new()).unwrap()).unwrap();
    assert_eq!(
        written,
        "username,email,first_name,last_name,group_name\na,,,,admin\n"
    );
    assert!(admin_column_problems(b"username,group_name\na,admin\n")
        .unwrap()
        .is_empty());
}

#[test]
fn blank_or_repeated_trustee_names_block() {
    let a = native_trustee("a");
    let mut blank = native_trustee("x");
    blank.name = " ".into();
    let preview = trustee_changes(&[a.clone(), a.clone(), blank], &[]);
    assert!(preview
        .blockers
        .contains(&"trustee 'a' appears more than once in the package".to_string()));
    assert!(preview
        .blockers
        .contains(&"a trustee in the package has no name".to_string()));
}
