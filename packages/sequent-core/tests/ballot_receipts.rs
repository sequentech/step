// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box's Received signature must bind the exact ballot, voter key,
//! election and time, and the Ballot ID must follow from all of them. Each
//! rejection starts from a receipt that verifies.

#![cfg(feature = "default_features")]

use sequent_core::ballot::{
    BallotBoxKey, BallotStyle, ElectionEventPresentation, ReceiptsPolicy,
    ReceiptsPresentation,
};
use sequent_core::ballot_receipt::{
    ballot_box_key, ballot_id, format_ballot_id, normalize_ballot_id,
    sign_received_ballot, verify_received_ballot, BallotReceiptError,
    ReceivedBallot, ReceivedStatement,
};
use sequent_core::encrypt::hash_ballot_style;
use sequent_core::fixtures::ballot_codec::get_writein_ballot_style;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

/// PKCS#8 v1 Ed25519 keys whose seeds are 32 bytes of 0x01 and of 0x02.
const BALLOT_BOX_SK: &str =
    "MC4CAQAwBQYDK2VwBCIEIAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB";
const VOTER_SK: &str =
    "MC4CAQAwBQYDK2VwBCIEIAICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgIC";

const BALLOT_BOX_KEY_ID: &str = "fd110d301d2f077d";
const LATER: &str = "2028-05-08T03:00:00.001Z";
const RECEIVED_SIGNATURE: &str = "+RR9tbkpia9wUTsGALqnqU3S+af2JmMGKIk0BNDlOZrsE6cBP/PxU0vcxWKVIx1oHPN57Mx5zZEocsY4I0nkAQ==";
const BALLOT_ID: &str = "MSY2-HTDT";

fn ballot_box_sk() -> StrandSignatureSk {
    StrandSignatureSk::from_der_b64_string(BALLOT_BOX_SK).unwrap()
}

fn published_key() -> BallotBoxKey {
    ballot_box_key(&StrandSignaturePk::from_sk(&ballot_box_sk()).unwrap())
        .unwrap()
}

fn statement() -> ReceivedStatement {
    let voter_sk = StrandSignatureSk::from_der_b64_string(VOTER_SK).unwrap();
    let voter_pk = StrandSignaturePk::from_sk(&voter_sk).unwrap();
    ReceivedStatement {
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
        election_event_id: "33f18502-a67c-4853-8333-a58630663559".into(),
        election_id: "f2f1065e-b784-46d1-b81a-c71bfeb9ad55".into(),
        ballot_hash:
            "0b1f6a5c2d7e4f809a1b2c3d4e5f60718293a4b5c6d7e8f9000102030405060"
                .into(),
        voter_signing_pk: voter_pk.to_der_b64_string().unwrap(),
        voter_ballot_signature: voter_sk
            .sign(b"fixture-ballot")
            .unwrap()
            .to_b64_string()
            .unwrap(),
        received_at: "2028-05-08T03:00:00.000Z".into(),
        key_id: BALLOT_BOX_KEY_ID.into(),
    }
}

fn received() -> ReceivedBallot {
    sign_received_ballot(&ballot_box_sk(), statement()).unwrap()
}

#[test]
fn the_shared_vector_is_reproduced() {
    let key = published_key();
    assert_eq!(key.key_id, BALLOT_BOX_KEY_ID);

    let received = received();
    assert_eq!(received.received_signature, RECEIVED_SIGNATURE);
    assert_eq!(received.ballot_id, BALLOT_ID);
    assert_eq!(verify_received_ballot(&key, &received).unwrap(), BALLOT_ID);
}

/// `ballot_style_hash` is part of what the voter's key signs, so neither the
/// setting nor the published key may change it for ballots already cast.
#[test]
fn the_setting_and_the_published_key_leave_the_ballot_style_hash_unchanged() {
    let mut style = get_writein_ballot_style();
    style.election_event_presentation =
        Some(style.election_event_presentation.unwrap_or_default());
    let hash = hash_ballot_style(&style).unwrap();

    let mut with_receipts = style.clone();
    with_receipts.ballot_box_key = Some(published_key());
    with_receipts.election_event_presentation =
        Some(ElectionEventPresentation {
            receipts: Some(ReceiptsPresentation {
                policy: Some(ReceiptsPolicy::SIGNED_BY_BALLOT_BOX),
            }),
            ..style
                .election_event_presentation
                .clone()
                .unwrap_or_default()
        });

    assert_eq!(hash_ballot_style(&with_receipts).unwrap(), hash);
}

#[test]
fn a_ballot_style_published_before_receipts_reads_as_receipts_off() {
    let style = get_writein_ballot_style();
    let json = serde_json::to_value(&style).unwrap();
    assert!(json.get("ballot_box_key").is_none());

    let read: BallotStyle = serde_json::from_value(json).unwrap();
    assert_eq!(read.ballot_box_key, None);
    assert_eq!(
        ElectionEventPresentation::default().receipts_policy(),
        ReceiptsPolicy::DISABLED
    );
    let presentation: ElectionEventPresentation =
        serde_json::from_value(serde_json::json!({"receipts": {}})).unwrap();
    assert_eq!(presentation.receipts_policy(), ReceiptsPolicy::DISABLED);
}

#[test]
fn the_published_key_and_the_setting_reach_the_voters_device() {
    let mut style = get_writein_ballot_style();
    style.ballot_box_key = Some(published_key());
    style.election_event_presentation = Some(ElectionEventPresentation {
        receipts: Some(ReceiptsPresentation {
            policy: Some(ReceiptsPolicy::SIGNED_BY_BALLOT_BOX),
        }),
        ..Default::default()
    });

    let json = serde_json::to_value(&style).unwrap();
    assert_eq!(json["ballot_box_key"]["key_id"], BALLOT_BOX_KEY_ID);
    assert_eq!(
        json["election_event_presentation"]["receipts"]["policy"],
        "signed-by-ballot-box"
    );
    let read: BallotStyle = serde_json::from_value(json).unwrap();
    assert_eq!(read, style);
}

#[test]
fn a_ballot_id_is_eight_unambiguous_characters_in_two_groups() {
    let id = received().ballot_id;
    let (first, second) = id.split_once('-').unwrap();

    assert_eq!((first.len(), second.len()), (4, 4));
    assert!(id.chars().filter(|character| *character != '-').all(
        |character| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(character)
    ));
    assert_eq!(format_ballot_id(&[0xff; 5]), "ZZZZ-ZZZZ");
    assert_eq!(format_ballot_id(&[0x00; 5]), "0000-0000");
    assert_eq!(
        format_ballot_id(&[0x00, 0x44, 0x32, 0x14, 0xc7]),
        "0123-4567"
    );
}

#[test]
fn typed_ballot_ids_are_read_the_way_people_write_them() {
    for typed in ["FTBE-MHRX", "ftbe-mhrx", "ftbemhrx", " FTBE MHRX "] {
        assert_eq!(normalize_ballot_id(typed).as_deref(), Some("FTBE-MHRX"));
    }
    assert_eq!(
        normalize_ballot_id("o0iL-1lOo").as_deref(),
        Some("0011-1100")
    );
    for malformed in ["", "FTBE-MHR", "FTBE-MHRXX", "FTBE-MHRU", "FTBE_MHRX"] {
        assert_eq!(normalize_ballot_id(malformed), None, "{malformed}");
    }
}

#[test]
fn every_signed_field_changes_the_verdict() {
    let key = published_key();
    let valid = received();
    verify_received_ballot(&key, &valid).unwrap();

    let other_voter = StrandSignatureSk::generate().unwrap();
    let mutations: Vec<(&str, Box<dyn Fn(&mut ReceivedBallot)>)> = vec![
        ("tenant", Box::new(|r| r.statement.tenant_id.push('0'))),
        (
            "event",
            Box::new(|r| r.statement.election_event_id.push('0')),
        ),
        ("election", Box::new(|r| r.statement.election_id.push('0'))),
        (
            "ballot hash",
            Box::new(|r| r.statement.ballot_hash.push('0')),
        ),
        (
            "voter key",
            Box::new(move |r| {
                r.statement.voter_signing_pk =
                    StrandSignaturePk::from_sk(&other_voter)
                        .unwrap()
                        .to_der_b64_string()
                        .unwrap()
            }),
        ),
        (
            "voter signature",
            Box::new(|r| {
                r.statement.voter_ballot_signature = ballot_box_sk()
                    .sign(b"another-ballot")
                    .unwrap()
                    .to_b64_string()
                    .unwrap()
            }),
        ),
        ("time", Box::new(|r| r.statement.received_at = LATER.into())),
    ];

    for (field, mutate) in mutations {
        let mut altered = valid.clone();
        mutate(&mut altered);
        assert_eq!(
            verify_received_ballot(&key, &altered),
            Err(BallotReceiptError::InvalidSignature),
            "{field}"
        );
    }
}

#[test]
fn a_receipt_from_another_key_is_not_the_ballot_boxs() {
    let key = published_key();
    let forger = StrandSignatureSk::generate().unwrap();
    let forger_key =
        ballot_box_key(&StrandSignaturePk::from_sk(&forger).unwrap()).unwrap();

    let mut statement = statement();
    statement.key_id = forger_key.key_id.clone();
    let forged = sign_received_ballot(&forger, statement).unwrap();
    assert_eq!(
        verify_received_ballot(&key, &forged),
        Err(BallotReceiptError::UnknownKey)
    );

    let relabelled = sign_received_ballot(&forger, self::statement()).unwrap();
    assert_eq!(
        verify_received_ballot(&key, &relabelled),
        Err(BallotReceiptError::InvalidSignature)
    );

    let mislabelled_key = BallotBoxKey {
        key_id: key.key_id.clone(),
        public_key: forger_key.public_key,
    };
    assert_eq!(
        verify_received_ballot(&mislabelled_key, &relabelled),
        Err(BallotReceiptError::UnknownKey)
    );
}

#[test]
fn a_ballot_id_that_does_not_follow_from_the_receipt_is_refused() {
    let key = published_key();
    let mut received = received();
    received.ballot_id = "0000-0000".into();

    assert_eq!(
        verify_received_ballot(&key, &received),
        Err(BallotReceiptError::BallotIdMismatch)
    );
}

#[test]
fn signing_one_millisecond_later_gives_another_ballot_id() {
    let first = received();
    let mut later = statement();
    later.received_at = LATER.into();
    let second = sign_received_ballot(&ballot_box_sk(), later).unwrap();

    assert_ne!(first.ballot_id, second.ballot_id);
    assert_eq!(
        ballot_id(&second.statement, &second.received_signature).unwrap(),
        second.ballot_id
    );
}

#[test]
fn malformed_keys_and_signatures_are_errors_not_panics() {
    let key = published_key();

    let mut bad_signature = received();
    bad_signature.received_signature = "not base64".into();
    assert!(matches!(
        verify_received_ballot(&key, &bad_signature),
        Err(BallotReceiptError::Malformed(_))
    ));

    let mut bad_voter_key = statement();
    bad_voter_key.voter_signing_pk = "not base64".into();
    assert!(matches!(
        sign_received_ballot(&ballot_box_sk(), bad_voter_key),
        Err(BallotReceiptError::Malformed(_))
    ));

    let bad_key = BallotBoxKey {
        key_id: key.key_id,
        public_key: "not base64".into(),
    };
    assert!(matches!(
        verify_received_ballot(&bad_key, &received()),
        Err(BallotReceiptError::Malformed(_))
    ));
}

#[test]
fn a_malformed_field_is_named_in_the_error() {
    let mut bad_voter_signature = statement();
    bad_voter_signature.voter_ballot_signature = "not base64".into();
    let error = sign_received_ballot(&ballot_box_sk(), bad_voter_signature)
        .unwrap_err()
        .to_string();
    assert!(
        error.starts_with("Malformed ballot receipt: voter_ballot_signature: ")
    );

    let error = ballot_id(&statement(), "not base64")
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("Malformed ballot receipt: received_signature: "));

    let mut bad_voter_key = statement();
    bad_voter_key.voter_signing_pk = "not base64".into();
    let error = ballot_id(&bad_voter_key, RECEIVED_SIGNATURE)
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("Malformed ballot receipt: voter_signing_pk: "));

    bad_voter_signature = statement();
    bad_voter_signature.voter_ballot_signature = "not base64".into();
    assert!(matches!(
        ballot_id(&bad_voter_signature, RECEIVED_SIGNATURE),
        Err(BallotReceiptError::Malformed(_))
    ));
}

#[test]
fn refusals_say_what_was_wrong_with_the_receipt() {
    assert_eq!(
        BallotReceiptError::UnknownKey.to_string(),
        "The receipt was not signed with the published ballot box key"
    );
    assert_eq!(
        BallotReceiptError::InvalidSignature.to_string(),
        "The ballot box signature does not verify"
    );
    assert_eq!(
        BallotReceiptError::BallotIdMismatch.to_string(),
        "The Ballot ID does not follow from the receipt"
    );
}
