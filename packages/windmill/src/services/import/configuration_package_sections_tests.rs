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
fn a_census_with_a_blank_or_repeated_username_is_refused_before_anything_applies() {
    let repeated = event_zip(&[(
        "export_voters-e1.csv",
        b"area,username\nx,Ada\nx,bob\nx,ada\n",
    )]);
    assert_eq!(
        census_usernames(&repeated).unwrap(),
        Err("census row 4 repeats an earlier username (compared ignoring case)".to_string())
    );
    let blank = event_zip(&[("export_voters-e1.csv", b"area,username\nx,ada\nx,  \n")]);
    assert_eq!(
        census_usernames(&blank).unwrap(),
        Err("census row 3 has no username".to_string())
    );
    // The refusal names the row, never the voter.
    assert!(!census_usernames(&repeated)
        .unwrap()
        .unwrap_err()
        .contains("ada"));
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
    // The locked write keeps the installed trust even if it changed after the
    // preview, and never adds one the tenant does not have.
    let merged = merged_tenant(&tenant(), &other);
    assert_eq!(
        merged.settings.unwrap()["configuration_signing"],
        json!({"policy": "required"})
    );
    let mut untrusting = tenant();
    untrusting.settings = Some(json!({"i18n": {}}));
    let merged = merged_tenant(&untrusting, &other);
    assert_eq!(merged.settings, Some(json!({"i18n": {}})));
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
    // Do not silently strip credentials and make an altered request: reject
    // the whole invalid roster before it can be staged for import.
    assert!(absent_admin_rows(csv, &BTreeMap::new()).is_err());
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

#[test]
fn a_format_1_package_already_imported_is_admitted_only_into_this_applications_event() {
    use sequent_core::election_config::manifest::ManifestFormat::V1;
    use sequent_core::election_config::package_verify::{already_imported, Freshness};
    let derived = "00000000-0000-4000-8000-0000000000e1";
    assert_eq!(admission_problem(None, derived, V1, 1, false), None);
    assert_eq!(
        admission_problem(Some((Ok(Freshness::New), "other")), derived, V1, 2, false),
        None
    );
    // Resuming: this application imported it, into its own event (format 1, unchanged).
    assert_eq!(
        admission_problem(
            Some((Ok(Freshness::AlreadyImported), derived)),
            derived,
            V1,
            2,
            false
        ),
        None
    );
    // Imported standalone, into another event: the event section would be
    // refused after the tenant sections had changed.
    let elsewhere = admission_problem(
        Some((Ok(Freshness::AlreadyImported), "e-standalone")),
        derived,
        V1,
        2,
        false,
    )
    .unwrap();
    assert!(elsewhere.contains("already imported into event e-standalone"));
    let older =
        admission_problem(Some((Err(already_imported(3)), "e")), derived, V1, 3, false).unwrap();
    assert!(older.starts_with("manifest.configuration.revision: revision 3"));
}

#[test]
fn a_format_2_package_is_imported_exactly_once() {
    use sequent_core::election_config::manifest::ManifestFormat::V2;
    use sequent_core::election_config::package_verify::Freshness;
    let derived = "00000000-0000-4000-8000-0000000000e2";
    // Never imported, or a newer revision: admitted.
    assert_eq!(admission_problem(None, derived, V2, 1, false), None);
    assert_eq!(
        admission_problem(Some((Ok(Freshness::New), derived)), derived, V2, 2, false),
        None
    );
    // The same package again: refused into its OWN event (no idempotent replay)...
    let same = admission_problem(
        Some((Ok(Freshness::AlreadyImported), derived)),
        derived,
        V2,
        2,
        false,
    )
    .unwrap();
    assert!(
        same.starts_with(
            "manifest.configuration.revision: revision 2 of this configuration is already imported"
        ),
        "{same}"
    );
    assert!(
        same.contains("a format-2 package is imported exactly once"),
        "{same}"
    );
    // Only THIS invocation's own successful event import lets its convergence
    // re-preview pass; the same package into another event never does.
    assert_eq!(
        admission_problem(
            Some((Ok(Freshness::AlreadyImported), derived)),
            derived,
            V2,
            2,
            true
        ),
        None
    );
    assert!(admission_problem(
        Some((Ok(Freshness::AlreadyImported), "e-other")),
        derived,
        V2,
        2,
        true
    )
    .is_some());
    // ...and into any other event.
    let other = admission_problem(
        Some((Ok(Freshness::AlreadyImported), "e-other")),
        derived,
        V2,
        2,
        false,
    )
    .unwrap();
    assert!(
        other.contains("is already imported (into event e-other)"),
        "{other}"
    );
    // A rollback (another package, older revision) stays its own refusal.
    let rollback = sequent_core::election_config::problem::Problem::error(
        sequent_core::election_config::problem::Code::Rollback,
        "manifest.configuration.revision",
        "revision 1 is not newer than revision 2, the last one imported",
    );
    let refused = admission_problem(Some((Err(rollback), derived)), derived, V2, 1, false).unwrap();
    assert!(
        refused.contains("is not newer than revision 2"),
        "{refused}"
    );
    assert!(!refused.contains("already imported"), "{refused}");
}

#[test]
fn messaging_configuration_without_its_permission_blocks() {
    use crate::services::messaging::config::MessagingConfigPermission;
    assert!(
        messaging_problem(true, &MessagingConfigPermission::NOT_PERMITTED)
            .unwrap()
            .contains("messaging-config-write")
    );
    assert_eq!(
        messaging_problem(true, &MessagingConfigPermission::PERMITTED),
        None
    );
    assert_eq!(
        messaging_problem(false, &MessagingConfigPermission::NOT_PERMITTED),
        None
    );
}

#[test]
fn realm_only_messaging_channels_need_the_permission_when_the_import_publishes_them() {
    use serde_json::json;
    let realm = |published: &str| json!({"attributes": {"sequent.messaging": published}});
    let election = "33333333-3333-4333-8333-333333333333".to_string();
    let publishable = format!(
        r#"{{"version":1,"channels":[],"election_labels":{{}},"election_channels":{{"{election}":[]}}}}"#
    );
    let config = json!({"version": 1, "channels": []});
    assert!(carries_messaging(
        None,
        Some(&realm(&publishable)),
        std::slice::from_ref(&election)
    ));
    assert!(carries_messaging(Some(&config), None, &[]));
    // Channels the import would only note as not imported don't need it.
    assert!(!carries_messaging(None, Some(&realm(&publishable)), &[]));
    assert!(!carries_messaging(None, Some(&realm("not json")), &[]));
    assert!(!carries_messaging(None, Some(&realm(" ")), &[]));
    assert!(!carries_messaging(
        None,
        Some(&json!({"attributes": {}})),
        &[]
    ));
    assert!(!carries_messaging(None, None, &[]));
}

#[test]
fn a_file_never_decides_the_tenants_configuration_signing() {
    let installed = json!({"configuration_signing": {"policy": "required"}, "i18n": {}});
    // A file naming another policy gets the installed one back.
    assert_eq!(
        keep_reserved_settings(
            Some(json!({"configuration_signing": {"policy": "optional"}, "theme": "dark"})),
            Some(&installed)
        ),
        Some(json!({"configuration_signing": {"policy": "required"}, "theme": "dark"}))
    );
    // A file without it, or without settings at all, does not remove it.
    assert_eq!(
        keep_reserved_settings(Some(json!({"theme": "dark"})), Some(&installed)),
        Some(json!({"configuration_signing": {"policy": "required"}, "theme": "dark"}))
    );
    assert_eq!(
        keep_reserved_settings(None, Some(&installed)),
        Some(json!({"configuration_signing": {"policy": "required"}}))
    );
    assert_eq!(
        keep_reserved_settings(Some(Value::Null), Some(&installed)),
        Some(json!({"configuration_signing": {"policy": "required"}}))
    );
    // Where none is installed, a file can't add one.
    assert_eq!(
        keep_reserved_settings(
            Some(json!({"configuration_signing": {"policy": "optional"}, "theme": "dark"})),
            Some(&json!({"i18n": {}}))
        ),
        Some(json!({"theme": "dark"}))
    );
    assert_eq!(keep_reserved_settings(None, None), None);
    assert_eq!(
        keep_reserved_settings(Some(json!({"theme": "dark"})), None),
        Some(json!({"theme": "dark"}))
    );
}

#[test]
fn admin_roster_supported_attributes_survive_and_secrets_are_rejected() {
    let csv = b"username,group_name,permission_labels,trustee,title,sequent.read-only.mobile-number\nnew,admin,area-a|area-b,native-trustee,Officer,+123456\n";
    assert!(admin_column_problems(csv).unwrap().is_empty());
    let rows = read_admin_rows(csv).unwrap();
    assert_eq!(rows[0].fields["title"], "Officer");
    assert_eq!(absent_admin_rows(csv, &BTreeMap::new()).unwrap(), csv);
    for column in ["password", "hashed_password", "private_key", "unknown"] {
        let csv = format!("username,group_name,{column}\na,admin,\n");
        assert!(!admin_column_problems(csv.as_bytes()).unwrap().is_empty());
    }
}

#[test]
fn admin_roster_existing_attributes_and_privileges_are_checked() {
    use super::super::import_users::{
        GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
    };
    let rows = read_admin_rows(b"username,group_name,permission_labels,trustee,title\none,admin,label-a,native-trustee,Officer\n").unwrap();
    let denied = ImportUsersPrivileges::default();
    let allowed = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        permission_labels: PermissionLabelPolicy::Allowed,
    };
    assert!(
        !admin_detail_problems(&rows, &BTreeMap::new(), &BTreeSet::new(), &denied, false)
            .is_empty()
    );
    assert!(admin_detail_problems(
        &rows,
        &BTreeMap::new(),
        &set(&["native-trustee"]),
        &allowed,
        false
    )
    .is_empty());
    let mut current = BTreeMap::from([(
        "one".to_string(),
        ExistingAdmin {
            service_account: false,
            fields: BTreeMap::from([
                ("permission_labels".into(), set(&["label-a"])),
                ("trustee".into(), set(&["native-trustee"])),
                ("title".into(), set(&["Officer"])),
            ]),
        },
    )]);
    assert!(
        admin_detail_problems(&rows, &current, &set(&["native-trustee"]), &allowed, false)
            .is_empty()
    );
    current
        .get_mut("one")
        .unwrap()
        .fields
        .insert("title".into(), set(&["Different"]));
    let problems =
        admin_detail_problems(&rows, &current, &set(&["native-trustee"]), &allowed, false);
    assert!(problems.iter().any(|p| p.contains("title")));
    assert!(!problems.iter().any(|p| p.contains("Different")));
}

#[test]
fn admin_roster_account_flags_are_typed_and_existing_values_must_match() {
    use super::super::import_users::{
        GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
    };
    let csv = b"username,group_name,enabled,email_verified\nOne,admin,TRUE,false\n";
    let rows = read_admin_rows(csv).unwrap();
    assert_eq!(rows[0].fields["enabled"], "true");
    assert_eq!(rows[0].fields["email_verified"], "false");
    let privileges = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        permission_labels: PermissionLabelPolicy::Allowed,
    };
    let mut current = BTreeMap::from([(
        "one".into(),
        ExistingAdmin {
            service_account: false,
            fields: BTreeMap::from([
                ("enabled".into(), set(&["true"])),
                ("email_verified".into(), set(&["false"])),
            ]),
        },
    )]);
    assert!(
        admin_detail_problems(&rows, &current, &BTreeSet::new(), &privileges, false).is_empty()
    );
    current.get_mut("one").unwrap().service_account = true;
    assert!(
        !admin_detail_problems(&rows, &current, &BTreeSet::new(), &privileges, false).is_empty()
    );
    for csv in [
        "username,group_name,enabled\na,admin,yes\n",
        "username,group_name,email_verified\na,admin,\n",
        "username,group_name\na,admin\nA,admin\n",
        "username,group_name,group_name\na,admin,admin\n",
        "username,group_name,permission_labels\na,admin,a||b\n",
    ] {
        assert!(read_admin_rows(csv.as_bytes()).is_err(), "{csv}");
    }
}

#[test]
fn admin_roster_permission_labels_are_reconciled_not_blocked() {
    use super::super::import_users::{
        GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
    };
    let rows =
        read_admin_rows(b"username,group_name,permission_labels\nadmin,admin,area-a|area-b\n")
            .unwrap();
    let current = BTreeMap::from([("admin".into(), ExistingAdmin::default())]);
    let allowed = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        permission_labels: PermissionLabelPolicy::Allowed,
    };
    assert!(admin_detail_problems(&rows, &current, &BTreeSet::new(), &allowed, false).is_empty());
}

#[test]
fn admin_label_reconciliation_is_exact_idempotent_and_privileged() {
    use super::super::import_users::{
        GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
    };
    let allowed = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        permission_labels: PermissionLabelPolicy::Allowed,
    };
    let denied = ImportUsersPrivileges {
        permission_labels: PermissionLabelPolicy::Forbidden,
        ..allowed
    };
    let current = BTreeMap::from([(
        "admin".into(),
        ExistingAdmin {
            fields: BTreeMap::from([("permission_labels".into(), set(&["old", "keep"]))]),
            ..Default::default()
        },
    )]);
    let rows =
        read_admin_rows(b"username,group_name,permission_labels\nAdmin,admin,new|keep\n").unwrap();
    assert_eq!(
        admin_permission_label_updates(&rows, &current)["admin"],
        set(&["keep", "new"])
    );
    assert!(!admin_detail_problems(&rows, &current, &BTreeSet::new(), &denied, false).is_empty());
    let mut preview = admin_changes(
        &rows,
        &BTreeMap::from([("admin".into(), set(&["admin"]))]),
        &set(&["admin"]),
    );
    preview_admin_permission_labels(&mut preview, &rows, &current);
    assert!(preview.needs_apply);
    assert_eq!(preview.changes[0]["action"], "update-permission-labels");
    let same =
        read_admin_rows(b"username,group_name,permission_labels\nadmin,admin,keep|old\n").unwrap();
    assert!(admin_permission_label_updates(&same, &current).is_empty());
    let omitted = read_admin_rows(b"username,group_name\nadmin,admin\n").unwrap();
    assert!(admin_permission_label_updates(&omitted, &current).is_empty());
    let clear = read_admin_rows(b"username,group_name,permission_labels\nadmin,admin,\n").unwrap();
    assert!(admin_permission_label_updates(&clear, &current)["admin"].is_empty());
    assert!(!admin_detail_problems(&clear, &current, &BTreeSet::new(), &denied, false).is_empty());
    assert!(admin_detail_problems(&clear, &current, &BTreeSet::new(), &allowed, false).is_empty());
    let mut service = current.clone();
    service.get_mut("admin").unwrap().service_account = true;
    assert!(!admin_detail_problems(&rows, &service, &BTreeSet::new(), &allowed, false).is_empty());
    let mut conflict = admin_changes(
        &rows,
        &BTreeMap::from([("admin".into(), set(&["other"]))]),
        &set(&["admin"]),
    );
    preview_admin_permission_labels(&mut conflict, &rows, &current);
    assert!(!conflict.blockers.is_empty());
    assert_eq!(conflict.changes[0]["action"], "conflict");
    let changed =
        read_admin_rows(b"username,group_name,permission_labels,title\nadmin,admin,new,Changed\n")
            .unwrap();
    assert!(
        admin_detail_problems(&changed, &current, &BTreeSet::new(), &allowed, false)
            .iter()
            .any(|p| p.contains("title"))
    );
}

#[test]
fn verified_existing_admin_email_is_advisory_and_other_conflicts_still_block() {
    use super::super::import_users::{
        GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
    };
    let allowed = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        permission_labels: PermissionLabelPolicy::Allowed,
    };
    let rows = read_admin_rows(b"username,group_name,email,email_verified,enabled\nAdmin,admin,new@example.test,true,true\n").unwrap();
    let members = BTreeMap::from([("admin".into(), set(&["admin"]))]);
    let mut current = BTreeMap::from([(
        "admin".into(),
        ExistingAdmin {
            fields: BTreeMap::from([
                ("email".into(), set(&["old@example.test"])),
                ("email_verified".into(), set(&["false"])),
                ("enabled".into(), set(&["true"])),
            ]),
            ..Default::default()
        },
    )]);
    let before = current["admin"].fields.clone();
    let updates = admin_email_updates(&rows, &current, true);
    assert_eq!(
        updates["admin"],
        AdminEmailUpdate {
            email: Some("new@example.test".into()),
            email_verified: Some(true)
        }
    );
    let mut preview = admin_changes(&rows, &members, &set(&["admin"]));
    preview_admin_email(&mut preview, &updates);
    assert!(preview.needs_apply);
    assert_eq!(preview.changes[0]["action"], "update-email");
    assert_eq!(preview.changes[0]["warnings"].as_array().unwrap().len(), 2);
    assert_eq!(current["admin"].fields, before);
    assert!(admin_detail_problems(&rows, &current, &BTreeSet::new(), &allowed, true).is_empty());
    assert_eq!(
        admin_detail_problems(&rows, &current, &BTreeSet::new(), &allowed, false).len(),
        2
    );
    assert!(admin_email_updates(&rows, &current, false).is_empty());

    current
        .get_mut("admin")
        .unwrap()
        .fields
        .insert("enabled".into(), set(&["false"]));
    let blocked = admin_detail_problems(&rows, &current, &BTreeSet::new(), &allowed, true);
    assert_eq!(blocked.len(), 1);
    assert!(blocked[0].contains("'enabled'"));
    current.get_mut("admin").unwrap().service_account = true;
    assert!(admin_email_updates(&rows, &current, true).is_empty());
}

#[test]
fn changing_email_without_explicit_flag_never_carries_verification_forward() {
    let rows =
        read_admin_rows(b"username,group_name,email\nadmin,admin,new@example.test\n").unwrap();
    let current = BTreeMap::from([(
        "admin".into(),
        ExistingAdmin {
            fields: BTreeMap::from([
                ("email".into(), set(&["old@example.test"])),
                ("email_verified".into(), set(&["true"])),
            ]),
            ..Default::default()
        },
    )]);
    assert_eq!(
        admin_email_updates(&rows, &current, true)["admin"].email_verified,
        Some(false)
    );
    assert!(admin_email_updates(&rows, &BTreeMap::new(), true).is_empty());
}

#[test]
fn blank_or_equivalent_admin_email_preserves_verification_and_requires_a_real_address() {
    let current = BTreeMap::from([(
        "alice".into(),
        ExistingAdmin {
            fields: BTreeMap::from([
                ("email".into(), set(&["  ALICE@example.test "])),
                ("email_verified".into(), set(&["true"])),
            ]),
            ..Default::default()
        },
    )]);
    for csv in [
        "username,group_name,email\nalice,admin,\n",
        "username,group_name,email\nalice,admin,   \n",
        "username,group_name,email\nalice,admin,  ALICE@EXAMPLE.TEST \n",
    ] {
        assert!(
            admin_email_updates(&read_admin_rows(csv.as_bytes()).unwrap(), &current, true)
                .is_empty()
        );
    }
    let same_address_explicit_flag = read_admin_rows(
        b"username,group_name,email,email_verified\nalice,admin,Alice@example.test,false\n",
    )
    .unwrap();
    assert_eq!(
        admin_email_updates(&same_address_explicit_flag, &current, true)["alice"],
        AdminEmailUpdate {
            email: None,
            email_verified: Some(false)
        }
    );
    assert!(read_admin_rows(b"username,group_name,email\nalice,admin,not-an-email\n").is_err());
}

#[test]
fn signature_does_not_authorize_existing_user_email_by_itself_even_for_default_group() {
    use super::super::import_users::{
        GroupAssignmentPolicy, ImportUsersPrivileges, PermissionLabelPolicy,
    };
    let rows = read_admin_rows(
        b"username,group_name,email,email_verified\nalice,voter,new@example.test,true\n",
    )
    .unwrap();
    let current = BTreeMap::from([(
        "alice".into(),
        ExistingAdmin {
            fields: BTreeMap::from([
                ("email".into(), set(&["old@example.test"])),
                ("email_verified".into(), set(&["false"])),
            ]),
            ..Default::default()
        },
    )]);
    let denied = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::DefaultGroupOnly,
        permission_labels: PermissionLabelPolicy::Forbidden,
    };
    assert!(!email_write_stamped(&denied));
    assert_eq!(
        admin_detail_problems(&rows, &current, &BTreeSet::new(), &denied, true).len(),
        2
    );
    let allowed = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        permission_labels: PermissionLabelPolicy::Forbidden,
    };
    assert!(email_write_stamped(&allowed));
    assert!(admin_detail_problems(&rows, &current, &BTreeSet::new(), &allowed, true).is_empty());
}
