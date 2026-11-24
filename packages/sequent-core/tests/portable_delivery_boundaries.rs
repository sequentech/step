// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "election_config_archive")]

use sequent_core::election_config::{
    archive::{delivery, Artifact, Layout},
    portable_messaging::{PortableMessagingAccounts, FORMAT, MEMBER},
    tenant_package,
};
use serde_json::{json, Value};

fn messaging_accounts(sender: Value) -> Value {
    json!({"format":FORMAT,"accounts":[
        {"reference":"demo","name":"Demo sender","sender":sender}
    ]})
}

#[test]
fn sender_metadata_is_bounded_and_optional_null_is_not_a_declaration() {
    let messenger = |username: &str| {
        messaging_accounts(json!({
            "provider":"MESSENGER_SEND_API","page_id":"123",
            "page_username":username,"api_version":"v23.0"
        }))
    };
    assert!(PortableMessagingAccounts::parse(
        messenger("demo.page_1").to_string().as_bytes()
    )
    .is_ok());
    for invalid in ["", "demo page", "https://example.invalid"] {
        assert!(PortableMessagingAccounts::parse(
            messenger(invalid).to_string().as_bytes()
        )
        .is_err());
    }
    assert!(PortableMessagingAccounts::parse(
        messenger(&"x".repeat(65)).to_string().as_bytes()
    )
    .is_err());
    assert!(
        PortableMessagingAccounts::parse(&vec![b' '; 64 * 1024 + 1]).is_err()
    );

    let bad_email = messaging_accounts(json!({
        "provider":"AWS_SES","from_address":"missing-domain"
    }));
    assert!(
        PortableMessagingAccounts::parse(bad_email.to_string().as_bytes())
            .is_err()
    );

    let tenant = tenant_package::read(&json!({
        "format":"sequent.tenant-package/1","messagingAccounts":null
    }));
    assert!(tenant.unwrap().messaging_accounts.is_none());
}

#[test]
fn portable_delivery_needs_one_declaration_and_one_well_typed_event() {
    let definitions = Artifact {
        name: MEMBER.into(),
        bytes: messaging_accounts(json!({"provider":"AWS_SNS"}))
            .to_string()
            .into_bytes(),
    };
    let event = |id: &str, bytes: &[u8]| Artifact {
        name: format!("export_election_event-{id}.json"),
        bytes: bytes.to_vec(),
    };
    let valid = br#"{"messaging_config":{"version":1,"channels":[{"channel":"SMS","account_id":"package:demo"}]}}"#;
    let mut layout = Layout {
        importable: vec![event("one", valid)],
        auxiliary: vec![definitions.clone()],
        archive_name: "test.zip".into(),
    };
    assert!(delivery(&layout).is_ok());

    layout.auxiliary.push(definitions);
    assert!(delivery(&layout).is_err(), "ambiguous account declarations");
    layout.auxiliary.pop();

    layout.importable.push(event("two", valid));
    assert!(
        delivery(&layout).is_err(),
        "account references are scoped to one event"
    );
    layout.importable.pop();

    layout.importable[0] = event("bad-json", b"not json");
    assert!(
        delivery(&layout).is_err(),
        "cannot link accounts to an unreadable event"
    );

    layout.importable[0] = event(
        "bad-type",
        br#"{"messaging_config":"CANARY-NOT-FOR-OUTPUT"}"#,
    );
    let err = delivery(&layout).err().expect("invalid messaging shape");
    assert!(format!("{err:?}").contains("messaging_config"));
    assert!(!format!("{err:?}").contains("CANARY-NOT-FOR-OUTPUT"));
}
