// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

fn entry(path: &str, members: Vec<FileEntry>) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        size: 1,
        sha256: format!("sha-{path}"),
        members,
    }
}

fn leaf(path: &str) -> FileEntry {
    entry(path, vec![])
}

/// A delivery as `archive::delivery` writes one with every section, named
/// literally so a renamed member fails here.
fn complete_delivery() -> Vec<FileEntry> {
    vec![
        entry(
            "admin_portal_settings.zip",
            vec![
                leaf("admin_users.csv"),
                leaf("export_permissions-t1.csv"),
                leaf("export_templates-t1.csv"),
                leaf("keycloak_admin-t1.json"),
                leaf("templates/templates.json"),
                leaf("trustees_public.json"),
            ],
        ),
        leaf("blueprint.json"),
        leaf("historical_conversion.json"),
        leaf("keycloak_event_realm_patch.json"),
        entry(
            "official_election_setup.zip",
            vec![
                leaf("export_election_event-e1.json"),
                leaf("export_scheduled_events-e1.csv"),
                leaf("export_voters-e1.csv"),
            ],
        ),
        leaf("trustees_list.json"),
    ]
}

fn sections(sections: &[Section]) -> Requirements {
    Requirements {
        sections: sections.to_vec(),
        census: CensusRequirement::Optional,
    }
}

fn disposition_of(coverage: &Coverage, path: &str) -> Disposition {
    coverage
        .members
        .iter()
        .find(|m| m.path == path)
        .unwrap()
        .disposition
}

#[test]
fn every_member_of_a_complete_delivery_has_its_documented_disposition() {
    let coverage =
        coverage(&complete_delivery(), &sections(&Section::APPLY_ORDER));
    let expected = [
        (
            "admin_users.csv",
            Disposition::Applied(Section::AdminAccounts),
        ),
        (
            "export_permissions-t1.csv",
            Disposition::Applied(Section::RolePermissions),
        ),
        ("export_templates-t1.csv", Disposition::Manual),
        (
            "keycloak_admin-t1.json",
            Disposition::Applied(Section::TenantSettings),
        ),
        ("templates/templates.json", Disposition::Manual),
        (
            "trustees_public.json",
            Disposition::Applied(Section::Trustees),
        ),
        ("blueprint.json", Disposition::RecordOnly),
        ("historical_conversion.json", Disposition::RecordOnly),
        ("keycloak_event_realm_patch.json", Disposition::Manual),
        (
            "export_election_event-e1.json",
            Disposition::Applied(Section::ElectionEvent),
        ),
        (
            "export_scheduled_events-e1.csv",
            Disposition::Applied(Section::ElectionEvent),
        ),
        (
            "export_voters-e1.csv",
            Disposition::Applied(Section::ElectionEvent),
        ),
        ("trustees_list.json", Disposition::RecordOnly),
    ];
    assert_eq!(coverage.members.len(), expected.len());
    for (path, disposition) in expected {
        assert_eq!(disposition_of(&coverage, path), disposition, "{path}");
    }
    assert!(coverage.missing().is_empty());
}

#[test]
fn a_required_section_that_is_absent_is_missing_and_an_unrequired_one_is_not() {
    // Control: the complete delivery has nothing missing. Remove trustees and
    // administrators; only the required one is reported.
    let mut delivery = complete_delivery();
    delivery[0].members.retain(|m| {
        m.path != "trustees_public.json" && m.path != "admin_users.csv"
    });
    let coverage = coverage(
        &delivery,
        &sections(&[Section::Trustees, Section::ElectionEvent]),
    );
    assert_eq!(coverage.missing(), vec![Section::Trustees]);
    assert_eq!(
        coverage.sections[&Section::AdminAccounts],
        SectionState::NotIncluded
    );
}

#[test]
fn unknown_members_are_shown_not_dropped_or_applied() {
    let mut delivery = complete_delivery();
    delivery.push(leaf("future_feature.json"));
    delivery[0].members.push(leaf("future_portal_setting.csv"));
    delivery.push(entry("other.zip", vec![leaf("inner.json")]));
    let coverage = coverage(&delivery, &sections(&[]));
    assert_eq!(
        disposition_of(&coverage, "future_feature.json"),
        Disposition::Unknown
    );
    assert_eq!(
        disposition_of(&coverage, "future_portal_setting.csv"),
        Disposition::Unknown
    );
    // An unrecognised nested zip is one unknown member, not expanded into
    // members that might be mistaken for configuration.
    assert_eq!(disposition_of(&coverage, "other.zip"), Disposition::Unknown);
    assert!(coverage.members.iter().all(|m| m.path != "inner.json"));
}

#[test]
fn a_realm_patch_is_never_mistaken_for_tenant_settings() {
    // `keycloak_admin_realm_patch.json` shares a prefix with the importer's
    // `keycloak_admin` match; at the root it is a manual patch, inside the
    // portal zip it is not tenant settings either.
    assert_eq!(
        root_disposition("keycloak_admin_realm_patch.json"),
        Disposition::Manual
    );
    assert_eq!(
        nested_disposition(
            "admin_portal_settings.zip",
            "keycloak_admin_realm_patch.json"
        ),
        Disposition::Unknown
    );
}

#[test]
fn signature_members_are_verification_only() {
    for path in ["manifest.json", "manifest.sig", "manifest.chain.pem"] {
        assert_eq!(root_disposition(path), Disposition::Verification);
    }
}

#[test]
fn the_apply_order_puts_tenant_configuration_before_the_event() {
    assert_eq!(
        Section::APPLY_ORDER,
        [
            Section::TenantSettings,
            Section::RolePermissions,
            Section::AdminAccounts,
            Section::Trustees,
            Section::ElectionEvent
        ]
    );
}

#[test]
fn dispositions_serialize_as_a_stable_wire_shape() {
    assert_eq!(
        serde_json::to_value(Disposition::Applied(Section::RolePermissions))
            .unwrap(),
        serde_json::json!({"kind": "applied", "section": "role-permissions"})
    );
    assert_eq!(
        serde_json::to_value(Disposition::RecordOnly).unwrap(),
        serde_json::json!({"kind": "record-only"})
    );
}

#[test]
fn an_event_zip_without_its_event_document_is_not_an_event() {
    let mut delivery = complete_delivery();
    delivery[4]
        .members
        .retain(|m| !m.path.starts_with("export_election_event-"));
    let coverage = coverage(&delivery, &sections(&[Section::ElectionEvent]));
    assert_eq!(coverage.missing(), vec![Section::ElectionEvent]);
}

#[test]
fn an_unconsumed_member_of_the_event_zip_is_unknown_not_applied() {
    // Control: the event document and census are applied. A sidecar the event
    // importer does not dispatch on is ignored by it, so it must not look
    // configured.
    let mut delivery = complete_delivery();
    delivery[4].members.push(leaf("future_event_sidecar.json"));
    delivery[4].members.push(leaf("images/document_x_logo.png"));
    let coverage = coverage(&delivery, &sections(&[]));
    assert_eq!(
        disposition_of(&coverage, "future_event_sidecar.json"),
        Disposition::Unknown
    );
    assert_eq!(
        disposition_of(&coverage, "images/document_x_logo.png"),
        Disposition::Applied(Section::ElectionEvent)
    );
    assert_eq!(
        disposition_of(&coverage, "export_voters-e1.csv"),
        Disposition::Applied(Section::ElectionEvent)
    );
}

#[test]
fn a_required_census_is_missing_when_the_event_has_no_voters_file() {
    let required = Requirements {
        sections: vec![Section::ElectionEvent],
        census: CensusRequirement::Required,
    };
    // Control: with the voters file the package is complete.
    let complete = coverage(&complete_delivery(), &required);
    assert!(complete.is_complete());
    assert_eq!(
        complete.census,
        SectionState::Present {
            members: vec!["export_voters-e1.csv".into()]
        }
    );
    let mut delivery = complete_delivery();
    delivery[4]
        .members
        .retain(|m| !m.path.starts_with("export_voters-"));
    let without = coverage(&delivery, &required);
    assert_eq!(without.census, SectionState::Missing);
    assert!(without.missing().is_empty(), "the event itself is present");
    assert!(!without.is_complete());
    // Optional census: absent is simply not included.
    assert_eq!(
        coverage(&delivery, &sections(&[])).census,
        SectionState::NotIncluded
    );
}
