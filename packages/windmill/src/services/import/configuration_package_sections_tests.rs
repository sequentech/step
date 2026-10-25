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
    let absent = event_changes("e1", EventState::Absent, 3);
    assert!(absent.needs_apply);
    assert!(absent.blockers.is_empty());
    let done = event_changes("e1", EventState::Present { voters: 3 }, 3);
    assert!(!done.needs_apply);
    assert!(done.blockers.is_empty());
    assert_eq!(done.changes[0]["action"], "exists");
    let short = event_changes("e1", EventState::Present { voters: 2 }, 3);
    assert!(!short.needs_apply, "never imported twice");
    assert_eq!(
        short.blockers,
        vec!["event e1 holds 2 voters but the package's census has 3"]
    );
    let partial = event_changes("e1", EventState::RealmOnly, 3);
    assert!(!partial.needs_apply);
    assert_eq!(partial.changes[0]["action"], "partial");
    assert_eq!(partial.blockers.len(), 1);
}

#[test]
fn the_census_is_counted_from_the_event_zip() {
    use std::io::Write;
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("export_election_event-e1.json", &b"{}"[..]),
        ("export_voters-e1.csv", &b"username,area\na,x\nb,x\n"[..]),
    ] {
        out.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(bytes).unwrap();
    }
    let bytes = out.finish().unwrap().into_inner();
    assert_eq!(census_rows(&bytes).unwrap(), 2);
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
fn the_merged_tenant_is_written_in_the_importers_csv_format() {
    // Read back through the same parser the tenant import uses.
    let merged = merged_tenant(&tenant(), &TenantConfiguration::default());
    let csv = crate::services::export::export_tenant::tenant_csv(merged.clone()).unwrap();
    let mut reader = csv::Reader::from_reader(csv.as_slice());
    let header: Vec<String> = reader
        .headers()
        .unwrap()
        .iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        header,
        [
            "id",
            "slug",
            "created_at",
            "updated_at",
            "labels",
            "annotations",
            "is_active",
            "voting_channels",
            "settings",
            "test"
        ]
    );
    let record = reader.records().next().unwrap().unwrap();
    let (parsed, key) =
        crate::services::import::import_tenant::parse_tenant_record(&merged.id, &record).unwrap();
    assert!(key.is_none());
    // The slug is not compared: the exporter writes it JSON-quoted and the
    // parser keeps the quotes, and update_tenant never writes the slug, so the
    // target's own is kept either way.
    assert_eq!(parsed.settings, merged.settings);
    assert_eq!(parsed.annotations, merged.annotations);
    assert_eq!(parsed.voting_channels, merged.voting_channels);
    assert_eq!(parsed.is_active, merged.is_active);
    assert_eq!(parsed.test, merged.test);
}
