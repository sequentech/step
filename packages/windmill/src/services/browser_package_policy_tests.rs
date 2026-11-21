// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn policy(values: &[(&str, &str)]) -> BrowserPolicy {
    BrowserPolicy::from_sources(
        "00000000-0000-4000-8000-00000000000a",
        |name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        },
        |_| Err("no roster provisioned".into()),
    )
}

#[test]
fn preview_is_off_by_default() {
    assert!(matches!(policy(&[]), BrowserPolicy::Off));
    assert_eq!(recheck(&policy(&[]), None), Recheck::Proceed);
}

#[test]
fn configured_but_incomplete_policy_never_falls_back() {
    for values in [
        vec![(SWITCH, "true")],
        vec![(SWITCH, "")],
        vec![(SWITCH, "   ")],
        vec![(SWITCH, "<non-Unicode>")],
        vec![(SWITCH, SWITCH_VALUE), (TENANT, "not-a-uuid")],
        vec![
            (SWITCH, SWITCH_VALUE),
            (TENANT, "00000000-0000-0000-0000-000000014990"),
        ],
        vec![(SWITCH, SWITCH_VALUE)],
        vec![
            (SWITCH, SWITCH_VALUE),
            (TENANT, "00000000-0000-4000-8000-00000000000a"),
        ],
    ] {
        let policy = policy(&values);
        assert!(matches!(policy, BrowserPolicy::Unusable(_)));
        assert!(admit_with_policy(
            b"legacy upload",
            None,
            "00000000-0000-4000-8000-00000000000a",
            vec![],
            Utc::now(),
            &policy
        )
        .is_err());
        assert!(matches!(recheck(&policy, None), Recheck::Block(_)));
    }
}

#[test]
fn another_tenant_keeps_legacy_policy() {
    let policy = policy(&[
        (SWITCH, SWITCH_VALUE),
        (TENANT, "00000000-0000-4000-8000-00000000000b"),
    ]);
    assert!(matches!(policy, BrowserPolicy::OtherTenant));
    assert!(matches!(
        admit_with_policy(
            b"legacy upload",
            None,
            "00000000-0000-4000-8000-00000000000a",
            vec![],
            Utc::now(),
            &policy
        ),
        Ok(Admission::Unsigned)
    ));
}

#[test]
fn tenant_uuid_aliases_select_the_same_pinned_browser_policy() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let canonical = "00000000-0000-4000-8000-00000000000a";
    let key = |id: u8, role: &str| {
        let mut point = vec![id; 65];
        point[0] = 4;
        serde_json::json!({
            "key": format!("key-{id}"), "credential_id": URL_SAFE_NO_PAD.encode([id; 32]),
            "subject": format!("subject-{id}"), "role": role, "algorithm": "es256",
            "public_key": URL_SAFE_NO_PAD.encode(point), "record_sha256": "a".repeat(64),
        })
    };
    let text = serde_json::json!({
        "format": sequent_core::election_config::webauthn_proof::ROSTER_FORMAT,
        "rp_id": "ems.example", "origins": ["https://architect.ems.example"],
        "tenant": canonical, "keys": [key(1, "approver"), key(2, "final-signer")],
    })
    .to_string();
    let pin = sequent_core::election_config::webauthn_proof::Roster::sha256(&text);
    for (switch, value) in [(SWITCH, SWITCH_VALUE), (NORMAL_SWITCH, NORMAL_SWITCH_VALUE)] {
        for alias in [
            canonical,
            "0000000000004000800000000000000A",
            "{00000000-0000-4000-8000-00000000000A}",
        ] {
            let policy = BrowserPolicy::from_sources(
                alias,
                |name| match name {
                    name if name == switch => Some(value.into()),
                    TENANT => Some(canonical.into()),
                    ROSTER_FILE => Some("fixture-roster".into()),
                    ROSTER_SHA256 => Some(pin.clone()),
                    REQUIRED_APPROVALS => Some("1".into()),
                    _ => None,
                },
                |_| Ok(text.clone()),
            );
            assert!(
                matches!(policy, BrowserPolicy::BrowserOnly(_)),
                "{policy:?}"
            );
            assert!(admit_with_policy(
                b"legacy unsigned",
                None,
                alias,
                vec![],
                Utc::now(),
                &policy
            )
            .is_err());
            let event_json = br#" { "election_event": {} } "#;
            let refusing = admit_with_policy(event_json, None, alias, vec![], Utc::now(), &policy)
                .err()
                .expect("unsigned JSON must not enter this tenant");
            assert_eq!(
                refusing.errors().next().unwrap().id.as_deref(),
                Some("package.webauthn-required")
            );
            for legacy in [BrowserPolicy::Off, BrowserPolicy::OtherTenant] {
                assert!(matches!(
                    admit_with_policy(event_json, None, alias, vec![], Utc::now(), &legacy),
                    Ok(Admission::Unsigned)
                ));
            }
            assert!(matches!(recheck(&policy, None), Recheck::Block(_)));
        }
    }
}

#[test]
fn normal_site_policy_is_explicit_and_not_preview() {
    for values in [
        vec![(NORMAL_SWITCH, "true")],
        vec![(NORMAL_SWITCH, "")],
        vec![(NORMAL_SWITCH, NORMAL_SWITCH_VALUE)],
        vec![(NORMAL_SWITCH, NORMAL_SWITCH_VALUE), (SWITCH, SWITCH_VALUE)],
    ] {
        assert!(matches!(policy(&values), BrowserPolicy::Unusable(_)));
    }
    assert!(matches!(
        policy(&[
            (NORMAL_SWITCH, NORMAL_SWITCH_VALUE),
            (TENANT, "00000000-0000-4000-8000-00000000000b"),
        ]),
        BrowserPolicy::OtherTenant
    ));
    // This tenant still needs an independently provisioned roster and pin.
    assert!(matches!(
        policy(&[
            (NORMAL_SWITCH, NORMAL_SWITCH_VALUE),
            (TENANT, "00000000-0000-4000-8000-00000000000a"),
        ]),
        BrowserPolicy::Unusable(_)
    ));
}

#[test]
fn removing_mode_alone_never_reopens_legacy_admission() {
    for name in [TENANT, ROSTER_FILE, ROSTER_SHA256, REQUIRED_APPROVALS] {
        let policy = policy(&[(name, "")]);
        assert!(matches!(policy, BrowserPolicy::Unusable(_)));
        assert!(admit_with_policy(
            b"new legacy upload",
            None,
            "00000000-0000-4000-8000-00000000000a",
            vec![],
            Utc::now(),
            &policy
        )
        .is_err());
        assert!(matches!(recheck(&policy, None), Recheck::Block(_)));
    }
}

#[test]
fn managed_roster_mode_is_explicit_tenant_scoped_and_has_no_fallback() {
    let values = [
        (NORMAL_SWITCH, managed::MODE),
        (TENANT, "00000000-0000-4000-8000-00000000000a"),
        (managed::EXPECTED_RP_ID, "architect.ems.example"),
        (managed::EXPECTED_ORIGIN, "https://architect.ems.example"),
        (managed::ADMIN_SUBJECTS, "authorized-issuer-subject"),
    ];
    let mode = policy(&values);
    let BrowserPolicy::Managed(config) = &mode else {
        panic!("expected managed roster mode: {mode:?}");
    };
    assert_eq!(config.required, 2);
    assert!(config.authorize_subject("authorized-issuer-subject"));
    assert!(!config.authorize_subject("AUTHORIZED-ISSUER-SUBJECT"));
    assert!(matches!(recheck(&mode, None), Recheck::Block(_)));
    assert!(admit_with_policy(b"unsigned", None, values[1].1, vec![], Utc::now(), &mode).is_err());
    assert!(matches!(policy(&values[..4]), BrowserPolicy::Unusable(_)));
    let mut foreign_tenant = values;
    foreign_tenant[1].1 = "00000000-0000-4000-8000-00000000000b";
    assert!(matches!(
        policy(&foreign_tenant),
        BrowserPolicy::OtherTenant
    ));
    for (key, value) in [
        (SWITCH, SWITCH_VALUE),
        (ROSTER_FILE, ""),
        (ROSTER_SHA256, ""),
        (REQUIRED_APPROVALS, "0"),
        (REQUIRED_APPROVALS, "17"),
    ] {
        let mut configured = values.to_vec();
        configured.push((key, value));
        assert!(matches!(policy(&configured), BrowserPolicy::Unusable(_)));
    }
}

#[test]
fn persisted_managed_authority_is_a_fence_even_when_runtime_configuration_disappears() {
    let installed = || managed::Installed {
        raw: "{}".into(),
        sha256: "a".repeat(64),
        generation: 1,
        activated_by: "synthetic-administrator".into(),
        activated_at: Utc::now(),
    };
    for mode in [BrowserPolicy::Off, BrowserPolicy::OtherTenant] {
        assert!(matches!(
            mode.clone().resolve_installed(None),
            BrowserPolicy::Off | BrowserPolicy::OtherTenant
        ));
        assert!(matches!(
            mode.resolve_installed(Some(installed())),
            BrowserPolicy::Unusable(_)
        ));
    }
}
