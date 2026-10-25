// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use serde_json::json;

// Synthetic, structurally valid base64 stand-ins; the platform checks the key
// material itself at application.
const SIGNING: &str = "c3ludGhldGljLXNpZ25pbmc=";
const SHARE: &str = "c3ludGhldGljLXNoYXJl";

fn complete() -> Value {
    json!({
        "format": "sequent.tenant-package/1",
        "tenantConfiguration": {
            "annotations": {"css": ".x{}"},
            "votingChannels": {"online": true, "kiosk": false},
            "settings": {"language_conf": {"default_language_code": "en"}}
        },
        "tenantSettings": {"displayName": "Example Commission"},
        "roleMappings": [
            {"group": "admin", "roles": ["election-event-read", "election-event-write"]},
            {"group": "auditor", "roles": ["election-event-read"]}
        ],
        "adminAccounts": [
            {"username": "operator.one", "email": "one@example.test", "group": "admin"}
        ],
        "trustees": [
            {"name": "trustee-a", "signingPublicKey": SIGNING, "shareEncryptionPublicKey": SHARE}
        ],
        "provenance": {"roleMappings": "example-template sha256 0000"}
    })
}

fn paths(report: &Report) -> Vec<(Code, String)> {
    report
        .problems
        .iter()
        .map(|p| (p.code, p.path.clone()))
        .collect()
}

#[test]
fn a_complete_package_writes_each_section_in_its_importers_format() {
    let package = read(&complete()).unwrap();
    let written = artifacts(&package, "t1").unwrap();
    let by_name: BTreeMap<&str, String> = written
        .iter()
        .map(|a| (a.name.as_str(), String::from_utf8(a.bytes.clone()).unwrap()))
        .collect();
    assert_eq!(
        by_name.keys().copied().collect::<Vec<_>>(),
        vec![
            "admin_users.csv",
            "export_permissions-t1.csv",
            "keycloak_admin-t1.json",
            "tenant_platform-t1.json",
            "trustees_public.json"
        ]
    );
    let platform: Value =
        serde_json::from_str(&by_name["tenant_platform-t1.json"]).unwrap();
    assert_eq!(
        platform,
        json!({
            "annotations": {"css": ".x{}"},
            "votingChannels": {"online": true, "kiosk": false},
            "settings": {"language_conf": {"default_language_code": "en"}}
        })
    );
    // The roles file is exactly what read_roles_config_file parses.
    assert_eq!(
        by_name["export_permissions-t1.csv"],
        "role,permissions\nadmin,election-event-read|election-event-write\nauditor,election-event-read\n"
    );
    // The admin file uses import_users' own column names and no credential column.
    assert_eq!(
        by_name["admin_users.csv"],
        "username,email,first_name,last_name,group_name\noperator.one,one@example.test,,,admin\n"
    );
    // A RealmRepresentation subset: only the fields the tenant importer applies.
    let realm: Value =
        serde_json::from_str(&by_name["keycloak_admin-t1.json"]).unwrap();
    assert_eq!(realm, json!({"displayName": "Example Commission"}));
    let trustees: Value =
        serde_json::from_str(&by_name["trustees_public.json"]).unwrap();
    assert_eq!(
        trustees,
        json!([{"name": "trustee-a", "signingPublicKey": SIGNING, "shareEncryptionPublicKey": SHARE}])
    );
}

#[test]
fn an_absent_section_writes_nothing_rather_than_a_default() {
    let package = read(&json!({"format": "sequent.tenant-package/1"})).unwrap();
    assert!(artifacts(&package, "t1").unwrap().is_empty());
}

#[test]
fn credentials_and_private_keys_are_refused_by_name_wherever_they_appear() {
    let mut doc = complete();
    doc["adminAccounts"][0]["password"] = json!("x");
    doc["trustees"][0]["signingPrivateKey"] = json!("y");
    let report = read(&doc).unwrap_err();
    assert_eq!(
        paths(&report),
        vec![
            (
                Code::NotApproved,
                "tenant package.adminAccounts[0].password".to_string()
            ),
            (
                Code::NotApproved,
                "tenant package.trustees[0].signingPrivateKey".to_string()
            ),
        ]
    );
    // The reason, never the value.
    assert!(report
        .problems
        .iter()
        .all(|p| !p.message.contains('x') || !p.message.contains("\"x\"")));
}

#[test]
fn an_unknown_field_or_format_is_refused() {
    let mut doc = complete();
    doc["adminAccounts"][0]["isSuperuser"] = json!(true);
    assert_eq!(read(&doc).unwrap_err().problems[0].code, Code::InvalidValue);
    let mut doc = complete();
    doc["format"] = json!("sequent.tenant-package/9");
    assert_eq!(read(&doc).unwrap_err().problems[0].code, Code::InvalidValue);
}

#[test]
fn every_record_problem_is_reported() {
    let mut doc = complete();
    doc["roleMappings"][1]["group"] = json!("admin");
    doc["roleMappings"][0]["roles"] = json!(["a|b", "a|b"]);
    doc["adminAccounts"] = json!([
        {"username": "x", "group": "missing-group"},
        {"username": "X", "group": "admin"}
    ]);
    doc["trustees"][0]["signingPublicKey"] = json!("not base64!");
    let report = read(&doc).unwrap_err();
    assert_eq!(
        paths(&report),
        vec![
            (Code::InvalidValue, "roleMappings[0].roles[0]".to_string()),
            (Code::InvalidValue, "roleMappings[0].roles[1]".to_string()),
            (Code::DuplicateId, "roleMappings[1].group".to_string()),
            (
                Code::DanglingReference,
                "adminAccounts[0].group".to_string()
            ),
            (Code::DuplicateId, "adminAccounts[1].username".to_string()),
            (
                Code::InvalidValue,
                "trustees[0].signingPublicKey".to_string()
            ),
        ]
    );
}

#[test]
fn administrators_may_name_a_target_group_when_the_package_maps_none() {
    // Without role mappings the group is the target's; it is checked against
    // the target when the package is applied.
    let doc = json!({
        "format": "sequent.tenant-package/1",
        "adminAccounts": [{"username": "a", "group": "existing"}]
    });
    assert!(read(&doc).is_ok());
}

#[test]
fn a_file_the_workbook_already_wrote_is_a_conflict_not_an_overwrite() {
    let package = read(&complete()).unwrap();
    let mut layout = super::super::archive::Layout {
        importable: vec![],
        archive_name: "d.zip".into(),
        auxiliary: vec![Artifact {
            name: "admin_users.csv".into(),
            bytes: b"username\n".to_vec(),
        }],
    };
    let problem = add_to_layout(&mut layout, &package, "t1").unwrap_err();
    assert_eq!(problem.code, Code::DuplicateId);
    assert_eq!(problem.path, "admin_users.csv");
}

#[test]
fn a_complete_delivery_nests_tenant_sections_and_reports_missing_ones() {
    use super::super::package_contract::{
        CensusRequirement, Disposition, Requirements, Section, SectionState,
    };
    let layout = super::super::archive::Layout {
        importable: vec![Artifact {
            name: "export_election_event-e1.json".into(),
            bytes: b"{}".to_vec(),
        }],
        archive_name: "delivery.zip".into(),
        auxiliary: vec![],
    };
    // Settings and roles only: administrators and trustees are required but
    // not supplied, the gap an operator must see.
    let mut doc = complete();
    doc.as_object_mut().unwrap().remove("adminAccounts");
    doc.as_object_mut().unwrap().remove("trustees");
    let package = read(&doc).unwrap();
    let record = historical_record(&json!({"preservedVoters": 3}));
    let (delivery, coverage) = complete_delivery(
        layout,
        Some(&package),
        "t1",
        Some(record),
        &Requirements {
            sections: Section::APPLY_ORDER.to_vec(),
            census: CensusRequirement::Optional,
        },
        None,
    )
    .unwrap();
    assert_eq!(delivery.name, "delivery.zip");
    assert_eq!(
        coverage.missing(),
        vec![Section::AdminAccounts, Section::Trustees]
    );
    assert_eq!(
        coverage.sections[&Section::RolePermissions],
        SectionState::Present {
            members: vec!["export_permissions-t1.csv".into()]
        }
    );
    let record = coverage
        .members
        .iter()
        .find(|m| m.path == "historical_conversion.json")
        .unwrap();
    assert_eq!(record.container, None);
    assert_eq!(record.disposition, Disposition::RecordOnly);
    let settings = coverage
        .members
        .iter()
        .find(|m| m.path == "keycloak_admin-t1.json")
        .unwrap();
    assert_eq!(
        settings.container.as_deref(),
        Some("admin_portal_settings.zip")
    );
}

#[test]
fn a_census_count_that_differs_from_the_conversion_is_refused() {
    use super::super::package_contract::{CensusRequirement, Requirements};
    let layout = |rows: &str| super::super::archive::Layout {
        importable: vec![
            Artifact {
                name: "export_election_event-e1.json".into(),
                bytes: b"{}".to_vec(),
            },
            Artifact {
                name: "export_voters-e1.csv".into(),
                bytes: format!("username,area_name\n{rows}").into_bytes(),
            },
        ],
        archive_name: "delivery.zip".into(),
        auxiliary: vec![],
    };
    let required = Requirements {
        sections: vec![],
        census: CensusRequirement::Required,
    };
    // Control: two rows, two preserved.
    let (_, covered) = complete_delivery(
        layout("a,x\nb,x\n"),
        None,
        "t1",
        None,
        &required,
        Some(2),
    )
    .unwrap();
    assert!(covered.is_complete());
    let problem = complete_delivery(
        layout("a,x\n"),
        None,
        "t1",
        None,
        &required,
        Some(2),
    )
    .unwrap_err();
    assert_eq!(problem.code, Code::InvalidValue);
    assert_eq!(problem.path, "census");
}

#[test]
fn a_package_never_sets_the_tenants_identity_or_its_service_account_key() {
    // Control: the complete package reads.
    assert!(read(&complete()).is_ok());
    for field in ["slug", "isActive", "id", "test"] {
        let mut doc = complete();
        doc["tenantConfiguration"][field] = json!("x");
        assert_eq!(
            read(&doc).unwrap_err().problems[0].code,
            Code::InvalidValue,
            "{field}"
        );
    }
    let mut doc = complete();
    doc["tenantConfiguration"]["settings"]["gapi_key"] = json!({"k": "v"});
    let report = read(&doc).unwrap_err();
    assert_eq!(
        report.problems[0].path,
        "tenant package.tenantConfiguration.settings.gapi_key"
    );
    assert_eq!(report.problems[0].code, Code::NotApproved);
}
