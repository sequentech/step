// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Persisted string wrappers must retain their Borsh length prefix and UTF-8
//! payload. Expected bytes are assembled independently of the serializer.

use electoral_log::messages::newtypes::*;

macro_rules! string_wire_cases {
    ($($name:ident => $wrapper:ident),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                for text in ["", "élection-\u{1f5f3}", "a\0b"] {
                    let value = $wrapper(text.to_owned());
                    let mut expected = (text.len() as u32).to_le_bytes().to_vec();
                    expected.extend_from_slice(text.as_bytes());
                    assert_eq!(borsh::to_vec(&value).unwrap(), expected);
                    let decoded: $wrapper = borsh::from_slice(&expected).unwrap();
                    assert_eq!(decoded.0, text);

                    // Appended or truncated data cannot be accepted as a
                    // different signed identifier through prefix decoding.
                    let mut trailing = expected.clone();
                    trailing.push(0);
                    assert!(borsh::from_slice::<$wrapper>(&trailing).is_err());
                    assert!(borsh::from_slice::<$wrapper>(&expected[..expected.len() - 1]).is_err());
                    assert_eq!(serde_json::to_value(&value).unwrap(), serde_json::json!(text));
                    assert_eq!(serde_json::from_value::<$wrapper>(serde_json::json!(text)).unwrap().0, text);
                }
            }
        )+
    };
}

string_wire_cases! {
    event_identifier => EventIdString,
    contest_identifier => ContestIdString,
    tenant_identifier => TenantIdString,
    administrator_identifier => AdminUserIdString,
    trustee_name => TrusteeNameString,
    ballot_publication_identifier => BallotPublicationIdString,
    public_key_encoding => PublicKeyDerB64,
    voter_address => VoterIpString,
    voter_country => VoterCountryString,
    voting_channel => VotingChannelString,
    cast_vote_error => CastVoteErrorString,
    identity_error => ErrorMessageString,
    identity_event_type => KeycloakEventTypeString,
    reconciliation_sequence => ExternalReconciliationSequenceString,
    reconciliation_timestamp => ExternalReconciliationGeneratedAtString,
    reconciliation_input_hash => ExternalReconciliationInputHashString,
    phone_number => PhoneE164String,
    results_publication_identifier => ResultsPublicationIdString,
    results_route_scope => ResultsPublicationRouteScopeString,
    results_access => ResultsPublicationAccessString,
    results_visibility_scope => ResultsPublicationVisibilityScopeString,
}

#[test]
fn absent_election_and_present_empty_identifier_have_different_wire_encodings() {
    assert_eq!(borsh::to_vec(&ElectionIdString(None)).unwrap(), vec![0]);
    assert_eq!(
        borsh::to_vec(&ElectionIdString(Some(String::new()))).unwrap(),
        vec![1, 0, 0, 0, 0]
    );
    assert!(borsh::from_slice::<ElectionIdString>(&[2]).is_err());
}

macro_rules! action_wire_cases {
    ($($name:ident => $type:ident { $($variant:ident = $tag:literal),+ }),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                $(
                    let value = $type::$variant;
                    assert_eq!(borsh::to_vec(&value).unwrap(), vec![$tag]);
                    let decoded: $type = borsh::from_slice(&[$tag]).unwrap();
                    assert_eq!(decoded, value);
                    assert_eq!(value.to_string(), stringify!($variant));
                    assert_eq!(serde_json::to_value(&value).unwrap(), serde_json::json!(stringify!($variant)));
                )+
                assert!(borsh::from_slice::<$type>(&[255]).is_err());
            }
        )+
    };
}

// Numeric tags are persisted and signed. Reordering an enum must fail these
// golden cases even if serialization and deserialization change together.
action_wire_cases! {
    certificate_actions => CertificateAuthEventAction {Import = 0, Delete = 1},
    request_directions => ExtApiRequestDirection {Inbound = 0, Outbound = 1},
    api_names => ExtApiName {Datafix = 0, Other = 1},
    reconciliation_actions => ExternalReconciliationKind {PatchGenerated = 0, ChangesApplied = 1},
    blacklist_actions => PhoneBlacklistAction {CreateEntry = 0, DeleteEntry = 1},
    results_actions => ResultsPublicationAction {Publish = 0, Revoke = 1},
}

#[test]
fn empty_lists_and_absent_lists_remain_distinct() {
    assert_eq!(borsh::to_vec(&ElectionsIdsString(None)).unwrap(), vec![0]);
    assert_eq!(
        borsh::to_vec(&ElectionsIdsString(Some(vec![]))).unwrap(),
        vec![1, 0, 0, 0, 0]
    );
    assert_eq!(
        borsh::to_vec(&CertificateSubjectDnsString(vec![])).unwrap(),
        vec![0, 0, 0, 0]
    );
    assert_eq!(
        borsh::to_vec(&ResolutionIdsString(vec![])).unwrap(),
        vec![0, 0, 0, 0]
    );
    assert_eq!(
        borsh::to_vec(&ExternalReconciliationOutputHashString(None)).unwrap(),
        vec![0]
    );
    assert_eq!(
        borsh::to_vec(&ExternalApiSubject {
            user_id: None,
            username: None
        })
        .unwrap(),
        vec![0, 0]
    );
}
