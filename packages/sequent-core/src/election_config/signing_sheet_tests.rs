// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic signing policies only: enums and numbers, no credentials.

use super::*;
use crate::election_config::sheet::Workbook;
use serde_json::json;

fn rule(value: Value) -> SigningRule {
    serde_json::from_value(value).unwrap()
}

/// A restrictive, non-default configuration as a platform could have stored
/// it, including a trustee rule whose stored count differs from its effective
/// one, an explicit "never expires" and a revision.
fn restrictive() -> PlannedSigning {
    PlannedSigning {
        rules: Some(vec![
            rule(json!({"action": "open-voting", "requirement": "required",
                "signatures": 3, "requester_signing": "not-allowed",
                "expires_minutes": 15, "revision": 2})),
            rule(json!({"action": "close-voting", "requirement": "required",
                "signatures": 2, "requester_signing": "allowed",
                "expires_minutes": null, "revision": 4})),
            rule(json!({"action": "key-ceremony", "requirement": "required",
                "signatures": 5, "requester_signing": "not-allowed",
                "expires_minutes": 60})),
        ]),
        checks: Some(
            serde_json::from_value(json!({"revocation_check": "check",
                "crl_unavailable": "refuse", "registration": "security-officer-only",
                "post_binding": "any-post", "revision": 3}))
            .unwrap(),
        ),
        approval_matrix: Some(json!({"synthetic": {"levels": [1, 2]}})),
        trust_not_carried: Vec::new(),
    }
}

#[cfg(feature = "election_config_xlsx")]
fn through_xlsx(signing: &PlannedSigning) -> Workbook {
    let workbook = Workbook::new(signing.sheets()).unwrap();
    let bytes =
        crate::election_config::xlsx_write::write_xlsx(&workbook).unwrap();
    crate::election_config::xlsx::read_xlsx(&bytes).unwrap()
}

#[cfg(feature = "election_config_xlsx")]
#[test]
fn a_configuration_survives_a_real_xlsx_round_trip_as_stored() {
    let signing = restrictive();
    let read = PlannedSigning::from_workbook(&through_xlsx(&signing)).unwrap();
    assert_eq!(read, signing);
    // Stored, not effective: the trustee rule keeps its 5 and not-allowed.
    let trustee = &read.rules.as_ref().unwrap()[2];
    assert_eq!(trustee.signatures, 5);
    assert_eq!(trustee.required(), 1);
}

#[cfg(feature = "election_config_xlsx")]
#[test]
fn each_field_keeps_its_own_presence() {
    let none = PlannedSigning::default();
    assert_eq!(
        PlannedSigning::from_workbook(&through_xlsx(&none)).unwrap(),
        none
    );

    let empty_rules = PlannedSigning {
        rules: Some(vec![]),
        ..Default::default()
    };
    assert_eq!(
        PlannedSigning::from_workbook(&through_xlsx(&empty_rules)).unwrap(),
        empty_rules,
        "a header-only SigningRules sheet is 'no rules', not 'nothing said'"
    );

    let checks_only = PlannedSigning {
        checks: restrictive().checks,
        ..Default::default()
    };
    assert_eq!(
        PlannedSigning::from_workbook(&through_xlsx(&checks_only)).unwrap(),
        checks_only
    );

    let rules_only = PlannedSigning {
        rules: restrictive().rules,
        ..Default::default()
    };
    assert_eq!(
        PlannedSigning::from_workbook(&through_xlsx(&rules_only)).unwrap(),
        rules_only
    );
}

#[test]
fn export_presence_is_field_by_field_and_null_is_absent() {
    let read =
        |document: Value| PlannedSigning::from_export(&document).unwrap();
    assert_eq!(read(json!({})), PlannedSigning::default());
    assert_eq!(
        read(json!({"signing_rules": null, "signing_checks": null})),
        PlannedSigning::default()
    );
    assert_eq!(read(json!({"signing_rules": []})).rules, Some(vec![]));
    assert!(read(json!({"signing_rules": []})).checks.is_none());
}

#[test]
fn a_blank_expiry_is_the_platform_default_and_none_is_no_limit() {
    let grid = vec![
        RULE_COLUMNS
            .iter()
            .map(|c| Cell::text(*c))
            .collect::<Vec<_>>(),
        vec![
            Cell::text("open-voting"),
            Cell::text("required"),
            Cell::Int(2),
            Cell::text("allowed"),
            Cell::Blank,
            Cell::Blank,
        ],
        vec![
            Cell::text("close-voting"),
            Cell::text("required"),
            Cell::Int(2),
            Cell::text("allowed"),
            Cell::text("None"),
            Cell::Blank,
        ],
        vec![
            Cell::text("key-ceremony"),
            Cell::text("required"),
            Cell::Int(1),
            Cell::text("allowed"),
            Cell::text("null"),
            Cell::Blank,
        ],
    ];
    let workbook =
        Workbook::new(vec![Sheet::from_grid("SigningRules", &grid).unwrap()])
            .unwrap();
    let rules = PlannedSigning::from_workbook(&workbook)
        .unwrap()
        .rules
        .unwrap();
    assert_eq!(
        rules[0].expires_minutes,
        Some(crate::signing::DEFAULT_EXPIRES_MINUTES)
    );
    assert_eq!(rules[1].expires_minutes, None);
    assert_eq!(
        rules[2].expires_minutes, None,
        "the null token is never-expires, not 60"
    );
}

#[test]
fn a_row_that_does_not_read_is_refused_with_its_place_never_skipped() {
    let grid = vec![
        RULE_COLUMNS
            .iter()
            .map(|c| Cell::text(*c))
            .collect::<Vec<_>>(),
        vec![
            Cell::text("open-voting"),
            Cell::text("required"),
            Cell::Int(2),
            Cell::text("allowed"),
            Cell::Int(30),
            Cell::Blank,
        ],
        vec![
            Cell::text("open-the-vault"),
            Cell::text("required"),
            Cell::Int(2),
            Cell::text("allowed"),
            Cell::Int(30),
            Cell::Blank,
        ],
        vec![
            Cell::text("close-voting"),
            Cell::text("required"),
            Cell::text("two"),
            Cell::text("allowed"),
            Cell::Int(30),
            Cell::Blank,
        ],
    ];
    let checks = vec![
        CHECK_COLUMNS
            .iter()
            .map(|c| Cell::text(*c))
            .collect::<Vec<_>>(),
        vec![Cell::text("check"), Cell::text("refuse")],
        vec![Cell::text("check"), Cell::text("refuse")],
    ];
    let workbook = Workbook::new(vec![
        Sheet::from_grid("SigningRules", &grid).unwrap(),
        Sheet::from_grid("SigningChecks", &checks).unwrap(),
    ])
    .unwrap();
    let problems = PlannedSigning::from_workbook(&workbook).unwrap_err();
    let paths: Vec<&str> = problems.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "SigningRules[1]",
            "SigningRules[2].signatures",
            "SigningChecks"
        ],
        "{problems:?}"
    );
}

#[test]
fn a_column_the_sheet_does_not_define_is_refused_not_defaulted() {
    let mut header: Vec<Cell> =
        RULE_COLUMNS.iter().map(|c| Cell::text(*c)).collect();
    header[4] = Cell::text("expiry_minutes");
    let grid = vec![
        header,
        vec![
            Cell::text("open-voting"),
            Cell::text("required"),
            Cell::Int(2),
            Cell::text("allowed"),
            Cell::Int(5),
            Cell::Blank,
        ],
    ];
    let workbook =
        Workbook::new(vec![Sheet::from_grid("SigningRules", &grid).unwrap()])
            .unwrap();
    let problems = PlannedSigning::from_workbook(&workbook).unwrap_err();
    assert!(
        problems
            .iter()
            .any(|p| p.id.as_deref() == Some("signing.unknown-column")
                && p.path == "SigningRules.expiry_minutes"),
        "{problems:?}"
    );
}

#[cfg(feature = "election_config_xlsx")]
#[test]
fn the_approval_matrix_keeps_absent_apart_from_empty() {
    let empty = PlannedSigning {
        approval_matrix: Some(json!({})),
        ..Default::default()
    };
    assert_eq!(
        PlannedSigning::from_workbook(&through_xlsx(&empty)).unwrap(),
        empty
    );
    let read =
        |document: Value| PlannedSigning::from_export(&document).unwrap();
    assert_eq!(
        read(json!({"approval_matrix": {}})).approval_matrix,
        Some(json!({}))
    );
    assert_eq!(read(json!({"approval_matrix": null})).approval_matrix, None);
}
