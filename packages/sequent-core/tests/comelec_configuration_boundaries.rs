// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_signing"
))]

use sequent_core::election_config::{
    architect::{Blueprint, Trustee},
    demo_credentials::{
        default_demo_login_roster, extract_demo_workbook, is_ordinary_workbook,
        reject_ordinary_credentials, validate_demo_draft,
        verify_demo_login_verifiers, DemoLoginVerifiers,
    },
    paths::Cell,
    sheet::{Sheet, Workbook, SHEET_ADMIN_USERS},
    xlsx, xlsx_write,
};
use serde_json::{json, Value};

const TENANT: &str = "12345678-1234-4567-8123-123456789abc";
const OTHER: &str = "12345678-1234-4567-8123-123456789abe";
const CANARY: &str = "secret-synthetic-workbook-only-8711";

fn workbook(admins: &[&[&str]], trustees: &[&[&str]]) -> Vec<u8> {
    let mut sheets = vec![];
    for (name, rows) in [("AdminUsers", admins), ("Trustees", trustees)] {
        if rows.is_empty() {
            continue;
        }
        let grid = rows
            .iter()
            .map(|row| row.iter().map(|text| Cell::text(*text)).collect())
            .collect::<Vec<Vec<Cell>>>();
        sheets.push(Sheet::from_grid(name, &grid).unwrap());
    }
    xlsx_write::write_xlsx(&Workbook::new(sheets).unwrap()).unwrap()
}

fn refuse(bytes: &[u8], expected: &str) {
    let error =
        extract_demo_workbook(bytes, TENANT, &default_demo_login_roster())
            .err()
            .expect("unapproved workbook was accepted");
    let output = format!("{error:?}");
    assert!(
        output.contains(expected),
        "expected a specific value-free refusal"
    );
    assert!(!output.contains(CANARY), "must not echo credentials");
}

#[test]
fn workbook_rejects_ambiguous_or_unlinked_identities_before_a_verifier_can_be_created(
) {
    let admin_headers =
        &["username", "group_name", "trustee", "email", "Password"];
    let trustee_headers = &["name", "email", "Password"];
    let valid = &[
        "trustee1",
        "trustee",
        "trustee1",
        "trustee1@demo.example.invalid",
        CANARY,
    ];
    let empty = &[
        "",
        "trustee",
        "trustee1",
        "trustee1@demo.example.invalid",
        CANARY,
    ];
    refuse(
        &workbook(&[admin_headers, empty], &[]),
        "requires an approved login",
    );
    refuse(
        &workbook(&[admin_headers, valid, valid], &[]),
        "duplicate administrator identity",
    );
    let linked = &["trustee1", "trustee1@demo.example.invalid", CANARY];
    let wrong_name = &["trustee2", "trustee1@demo.example.invalid", CANARY];
    let wrong_email = &["trustee1", "wrong@demo.example.invalid", CANARY];
    let no_name = &["", "trustee1@demo.example.invalid", CANARY];
    for row in [wrong_name, no_name] {
        refuse(
            &workbook(&[admin_headers, valid], &[trustee_headers, row]),
            "requires a linked login",
        );
    }
    refuse(
        &workbook(&[admin_headers, valid], &[trustee_headers, wrong_email]),
        "ambiguous demo trustee login",
    );
    refuse(
        &workbook(&[admin_headers, valid], &[trustee_headers, linked, linked]),
        "ambiguous demo trustee login",
    );
    for (header, value, message) in [
        (
            "Password-Salt",
            CANARY,
            "ambiguous or unsupported demo password column",
        ),
        (
            "Credentials",
            CANARY,
            "ambiguous or unsupported demo password column",
        ),
        ("Password", "12344568.0", "demo password must be text"),
        (
            "Password",
            "short",
            "demo password must be between 12 and 1024 UTF-8 bytes",
        ),
    ] {
        refuse(
            &workbook(
                &[
                    &["username", "group_name", header],
                    &["sbei-bangkok-1", "sbei", value],
                ],
                &[],
            ),
            message,
        );
    }
    let numeric = &[
        "trustee1",
        "99.0",
        "trustee1",
        "trustee1@demo.example.invalid",
        CANARY,
    ];
    refuse(
        &workbook(&[admin_headers, numeric], &[]),
        "identity fields must be text",
    );
    refuse(b"not-an-XLSX", "could not read the demo workbook");
}

#[test]
fn ordinary_workbook_guard_finds_nested_credentials_but_accepts_empty_values() {
    let sample = |value: &str| {
        workbook(&[&["username", "profile"], &["fixture", value]], &[])
    };
    let ordinary =
        sample(r#"[{"profile":{"password":""}},{"credentials":null}]"#);
    assert!(is_ordinary_workbook(&ordinary));
    let nested = sample(&json!([{"profile":{"password":CANARY}}]).to_string());
    assert!(!is_ordinary_workbook(&nested));
    let refused =
        reject_ordinary_credentials(xlsx::read_xlsx(&nested).unwrap().sheets())
            .unwrap_err();
    assert!(!format!("{refused:?}").contains(CANARY));
}

fn fixture() -> (DemoLoginVerifiers, Blueprint) {
    let bytes = workbook(
        &[
            &["username", "group_name", "trustee", "email", "Password"],
            &[
                "trustee1",
                "trustee",
                "trustee1",
                "trustee1@demo.example.invalid",
                CANARY,
            ],
        ],
        &[],
    );
    let extracted =
        extract_demo_workbook(&bytes, TENANT, &default_demo_login_roster())
            .unwrap();
    let clean = xlsx::read_xlsx(&extracted.workbook).unwrap();
    let platform = vec![clean.sheet(SHEET_ADMIN_USERS).unwrap().clone()];
    (
        extracted.manifest.unwrap(),
        Blueprint {
            platform,
            trustees: vec![Trustee {
                name: "trustee1".into(),
                email: "trustee1@demo.example.invalid".into(),
            }],
            ..Default::default()
        },
    )
}

#[test]
fn signed_verifier_set_and_current_draft_stay_bound_to_the_same_principal() {
    let approved = default_demo_login_roster();
    let (manifest, plan) = fixture();
    assert!(validate_demo_draft(&plan, TENANT, &manifest, &approved).is_ok());
    assert!(verify_demo_login_verifiers(&manifest, OTHER, &approved).is_err());
    let mut other = serde_json::to_value(&manifest).unwrap();
    other["tenantId"] = "non-canonical-id".into();
    let same_noncanonical = serde_json::from_value(other).unwrap();
    assert!(verify_demo_login_verifiers(
        &same_noncanonical,
        "non-canonical-id",
        &approved
    )
    .is_err());
    let manifest_value = serde_json::to_value(&manifest).unwrap();
    let set = manifest_value["accounts"].as_array().unwrap();
    for replacement in [json!([]), json!([set[0], set[0]])] {
        let mut wrong = manifest_value.clone();
        wrong["accounts"] = replacement;
        assert!(verify_demo_login_verifiers(
            &serde_json::from_value(wrong).unwrap(),
            TENANT,
            &approved
        )
        .is_err());
    }
    for (field, value) in [
        ("username", json!("trustee2")),
        ("email", Value::Null),
        ("groupName", json!("sbei")),
        ("trustee", Value::Null),
        ("iterations", json!(1)),
        ("salt", json!("short")),
        ("verifier", json!("!".repeat(44))),
        ("verifier", json!("A".repeat(44))),
    ] {
        let mut wrong = manifest_value.clone();
        wrong["accounts"][0][field] = value;
        let refused = verify_demo_login_verifiers(
            &serde_json::from_value(wrong).unwrap(),
            TENANT,
            &approved,
        )
        .unwrap_err();
        assert!(!format!("{refused:?}").contains(CANARY));
    }
    let mut missing = plan.clone();
    missing.platform.clear();
    assert!(
        validate_demo_draft(&missing, TENANT, &manifest, &approved).is_err()
    );
    let mut duplicate = plan.clone();
    let row = duplicate.platform[0].rows[0].clone();
    duplicate.platform[0].rows.push(row);
    assert!(
        validate_demo_draft(&duplicate, TENANT, &manifest, &approved).is_err()
    );
    let mut changed = plan;
    changed.trustees[0].email = "wrong@demo.example.invalid".into();
    assert!(
        validate_demo_draft(&changed, TENANT, &manifest, &approved).is_err()
    );
}
