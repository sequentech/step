// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Verify the public event constructors using real sender and system keys.
//! Outer search metadata and artifacts are deliberately outside the existing
//! signature contract; these tests never treat them as authenticated claims.

use anyhow::Result;
use electoral_log::messages::message::{Message, SigningData, GENERIC_EVENT};
use electoral_log::messages::newtypes::*;
use electoral_log::messages::statement::{StatementBody, StatementHead};
use electoral_log::ElectoralLogMessage;
use strand::hash::STRAND_HASH_LENGTH_BYTES;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

/// Simulate a file or transport that fills up after a chosen payload prefix.
struct FailingWriter {
    limit: usize,
    written: Vec<u8>,
}

impl std::io::Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.written.len() == self.limit {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
        }
        let count = bytes.len().min(self.limit - self.written.len());
        self.written.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

const EVENT: &str = "synthetic-event";
const ELECTION: &str = "synthetic-election";
const ACTOR: &str = "synthetic-actor";

fn event() -> EventIdString {
    EventIdString(EVENT.into())
}
fn election() -> ElectionIdString {
    ElectionIdString(Some(ELECTION.into()))
}
fn actor() -> Option<String> {
    Some(ACTOR.into())
}
fn channel() -> VotingChannelString {
    VotingChannelString("online".into())
}
fn pseudonym() -> PseudonymHash {
    PseudonymHash::new([7; STRAND_HASH_LENGTH_BYTES])
}
fn vote_hash() -> CastVoteHash {
    CastVoteHash::new([9; STRAND_HASH_LENGTH_BYTES])
}

fn signer() -> Result<(SigningData, StrandSignaturePk)> {
    let system = StrandSignatureSk::generate()?;
    let public = StrandSignaturePk::from_sk(&system)?;
    Ok((
        SigningData::new(StrandSignatureSk::generate()?, "test-signer", system),
        public,
    ))
}

/// Check persistence and independent verification, not merely that a constructor
/// returns Ok. Expected event kind and election scope come from the test case.
fn assert_record(
    message: &Message,
    public: &StrandSignaturePk,
    kind: &str,
    scope: Option<&str>,
) -> Result<()> {
    assert_eq!(message.statement.head.kind.to_string(), kind);
    assert_eq!(message.election_id.as_deref(), scope);
    assert!(!message.statement.head.description.is_empty());
    message.verify(public)?;

    let stored = ElectoralLogMessage::try_from(message)?;
    assert_eq!(stored.statement_kind, kind);
    assert_eq!(
        stored.statement_timestamp,
        message.statement.head.timestamp as i64
    );
    assert_eq!(stored.user_id, message.user_id);
    assert_eq!(stored.username, message.username);
    // The JSON display is also an interchange format. Decode it through
    // serde and verify that it reconstructs exactly the signed Borsh bytes.
    let from_json: Message = serde_json::from_str(&message.to_string())?;
    from_json.verify(public)?;
    assert_eq!(from_json.strand_serialize()?, stored.message);
    // A reader using Borsh directly must understand the stored payload;
    // compatibility must not depend on both sides sharing our Strand adapter.
    let restored: Message = borsh::from_slice(&stored.message)?;
    // Every truncated prefix must fail on both sides of the persistence
    // boundary. A round-trip alone could miss matching encoder/decoder bugs,
    // and a serializer must propagate a writer failure from any nested field.
    for limit in 0..stored.message.len() {
        let mut writer = FailingWriter {
            limit,
            written: Vec::new(),
        };
        let error = borsh::to_writer(&mut writer, message).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::StorageFull);
        assert_eq!(writer.written, stored.message[..limit]);
        assert!(borsh::from_slice::<Message>(&stored.message[..limit]).is_err());
    }

    restored.verify(public)?;
    assert_eq!(
        serde_json::to_value(&restored)?,
        serde_json::to_value(message)?
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&message.to_string())?,
        serde_json::to_value(message)?
    );
    Ok(())
}

#[test]
fn sender_and_system_signatures_both_reject_changed_statement_bytes() -> Result<()> {
    let (data, public) = signer()?;
    let original = Message::keygen_message(event(), &data, actor(), actor(), None)?;
    let encoded = original.strand_serialize()?;
    original.verify(&public)?;

    let wrong_key = StrandSignatureSk::generate()?;
    assert!(original
        .verify(&StrandSignaturePk::from_sk(&wrong_key)?)
        .is_err());

    // Change each part of the signed statement independently. Re-signing only
    // one side must not hide the other signature's failure.
    let mut changed = Message::strand_deserialize(&encoded)?;
    changed.statement.head.event = EventIdString("other-event".into());
    assert!(changed.verify(&public).is_err());
    changed.sender_signature = wrong_key.sign(&changed.statement.strand_serialize()?)?;
    changed.sender.pk = StrandSignaturePk::from_sk(&wrong_key)?;
    assert!(changed.verify(&public).is_err());

    let mut changed = Message::strand_deserialize(&encoded)?;
    changed.statement.body = StatementBody::KeyInsertionStart;
    assert!(changed.verify(&public).is_err());
    let mut changed = Message::strand_deserialize(&encoded)?;
    changed.statement.head.timestamp += 1;
    assert!(changed.verify(&public).is_err());
    let mut changed = Message::strand_deserialize(&encoded)?;
    changed.system_signature = wrong_key.sign(b"unrelated statement")?;
    assert!(changed.verify(&public).is_err());
    Ok(())
}

#[test]
fn election_lifecycle_keeps_single_election_and_event_wide_actions_distinct() -> Result<()> {
    let (data, public) = signer()?;
    let published = Message::election_published_message(
        event(),
        election(),
        BallotPublicationIdString("publication".into()),
        &data,
        actor(),
        actor(),
    )?;
    assert_record(&published, &public, "ElectionPublish", Some(ELECTION))?;
    assert!(
        matches!(published.statement.body, StatementBody::ElectionPublish(_, BallotPublicationIdString(ref id)) if id == "publication")
    );

    for selected in [Some(election()), None] {
        let scope = selected.as_ref().and_then(|election| election.0.as_deref());
        let opened = Message::election_open_message(
            event(),
            selected.clone(),
            Some(vec![ELECTION.into()]),
            channel(),
            &data,
            actor(),
            actor(),
        )?;
        let paused = Message::election_pause_message(
            event(),
            selected.clone(),
            channel(),
            &data,
            actor(),
            actor(),
        )?;
        let closed = Message::election_close_message(
            event(),
            selected.clone(),
            Some(vec![ELECTION.into()]),
            channel(),
            &data,
            actor(),
            actor(),
        )?;
        let kinds = if selected.is_some() {
            [
                "ElectionVotingPeriodOpen",
                "ElectionVotingPeriodPause",
                "ElectionVotingPeriodClose",
            ]
        } else {
            [
                "ElectionEventVotingPeriodOpen",
                "ElectionEventVotingPeriodPause",
                "ElectionEventVotingPeriodClose",
            ]
        };
        for (message, kind) in [opened, paused, closed].iter().zip(kinds) {
            assert_record(message, &public, kind, scope)?;
            assert_eq!(message.statement.head.event.0, EVENT);
            assert!(message
                .statement
                .head
                .description
                .contains("online channel"));
        }
    }
    Ok(())
}

#[test]
fn voting_events_preserve_receipt_and_scope_and_keep_failures_separate() -> Result<()> {
    let (data, public) = signer()?;
    let cast = Message::cast_vote_message(
        event(),
        election(),
        pseudonym(),
        vote_hash(),
        &data,
        VoterIpString("192.0.2.1".into()),
        VoterCountryString("CA".into()),
        actor(),
        actor(),
        "area".into(),
    )?;
    let with_channel = Message::cast_vote_with_channel_message(
        event(),
        election(),
        pseudonym(),
        vote_hash(),
        &data,
        VoterIpString("192.0.2.1".into()),
        VoterCountryString("CA".into()),
        channel(),
        actor(),
        actor(),
        "area".into(),
    )?;
    for (message, version) in [(&cast, "1"), (&with_channel, "2")] {
        assert_record(message, &public, "CastVote", Some(ELECTION))?;
        assert_eq!(
            message.ballot_id.as_deref(),
            Some(hex::encode([9; STRAND_HASH_LENGTH_BYTES / 2]).as_str())
        );
        assert_eq!(message.area_id.as_deref(), Some("area"));
        assert_eq!(ElectoralLogMessage::try_from(message)?.version, version);
    }
    let failed = Message::cast_vote_error_message(
        event(),
        election(),
        pseudonym(),
        CastVoteErrorString("invalid proof".into()),
        &data,
        VoterIpString("192.0.2.1".into()),
        VoterCountryString("CA".into()),
        actor(),
        "area".into(),
    )?;
    assert_record(&failed, &public, "CastVoteError", Some(ELECTION))?;
    assert_eq!(failed.statement.head.log_type.to_string(), "ERROR");
    assert!(failed.ballot_id.is_none());
    assert!(failed.username.is_none());
    Ok(())
}

#[test]
fn key_and_tally_ceremonies_keep_their_distinct_event_kinds_and_resolution_ids() -> Result<()> {
    let (data, public) = signer()?;
    let cases = [
        (
            Message::keygen_message(event(), &data, actor(), actor(), Some(ELECTION.into()))?,
            "KeyGeneration",
        ),
        (
            Message::key_insertion_start(event(), &data, actor(), actor(), Some(ELECTION.into()))?,
            "KeyInsertionStart",
        ),
        (
            Message::key_insertion_message(
                event(),
                TrusteeNameString("trustee-é".into()),
                &data,
                actor(),
                actor(),
                Some(ELECTION.into()),
            )?,
            "KeyInsertionCeremony",
        ),
        (
            Message::tally_open_message(event(), election(), &data, actor(), actor())?,
            "TallyOpen",
        ),
        (
            Message::tally_close_message(event(), election(), &data, actor(), actor())?,
            "TallyClose",
        ),
        (
            Message::tally_resumed_with_resolution(
                event(),
                election(),
                vec!["resolution".into()],
                &data,
            )?,
            "TallyResumedWithResolution",
        ),
        (
            Message::tally_paused_pending_resolutions(
                event(),
                election(),
                vec!["resolution".into()],
                &data,
            )?,
            "TallyPausedPendingResolution",
        ),
        (
            Message::tally_tie_resolved(
                event(),
                election(),
                ContestIdString("contest".into()),
                "resolution".into(),
                &data,
                actor(),
                actor(),
            )?,
            "TallyTieResolved",
        ),
        (
            Message::tally_tie_resolution_updated(
                event(),
                election(),
                ContestIdString("contest".into()),
                "resolution".into(),
                &data,
                actor(),
                actor(),
            )?,
            "TallyTieResolutionUpdated",
        ),
    ];
    for (message, kind) in cases {
        assert_record(&message, &public, kind, Some(ELECTION))?;
        match message.statement.body {
            StatementBody::TallyResumedWithResolution(_, ids)
            | StatementBody::TallyPausedPendingResolution(_, ids) => {
                assert_eq!(ids.0, vec!["resolution"])
            }
            StatementBody::TallyTieResolved(_, contest, ids)
            | StatementBody::TallyTieResolutionUpdated(_, contest, ids) => {
                assert_eq!(contest.0, "contest");
                assert_eq!(ids.0, vec!["resolution"]);
                assert_eq!(message.statement.head.event_type.to_string(), "USER");
            }
            _ => {}
        }
    }
    Ok(())
}

#[test]
fn identity_events_expose_only_the_intended_description_and_actor_scope() -> Result<()> {
    let (data, public) = signer()?;
    for (event_type, detail, expected, level) in [
        ("LOGIN:", "private details", "LOGIN", "INFO"),
        (
            "LOGIN_ERROR",
            "invalid_grant private details",
            "LOGIN_ERROR invalid_grant",
            "ERROR",
        ),
        ("LOGIN_ERROR", "", "LOGIN_ERROR ", "ERROR"),
    ] {
        let message = Message::keycloak_user_event(
            event(),
            KeycloakEventTypeString(event_type.into()),
            ErrorMessageString(detail.into()),
            actor(),
            actor(),
            &data,
            Some("area".into()),
        )?;
        assert_record(&message, &public, "KeycloakUserEvent", None)?;
        assert_eq!(message.statement.head.description, expected);
        assert_eq!(message.statement.head.log_type.to_string(), level);
        assert_eq!(message.statement.head.event_type.to_string(), "USER");
    }

    let voter = Message::voter_public_key_message(
        TenantIdString("tenant".into()),
        event(),
        pseudonym(),
        PublicKeyDerB64("public-key".into()),
        &data,
        actor(),
        actor(),
        Some("area".into()),
    )?;
    assert_record(&voter, &public, "VoterPublicKey", None)?;
    assert_eq!(voter.statement.head.event.0, EVENT);
    let admin = Message::admin_public_key_message(
        TenantIdString("tenant".into()),
        actor(),
        actor(),
        PublicKeyDerB64("public-key".into()),
        &data,
        None,
        Some("area".into()),
    )?;
    assert_record(&admin, &public, "AdminPublicKey", None)?;
    assert_eq!(admin.statement.head.event.0, GENERIC_EVENT);
    Ok(())
}

#[test]
fn certificate_phone_and_communication_events_preserve_the_requested_action() -> Result<()> {
    let (data, public) = signer()?;
    for (action, subjects, description) in [
        (
            CertificateAuthEventAction::Import,
            vec!["CN=Example".into()],
            "CA certificate imported. Subject: CN=Example",
        ),
        (
            CertificateAuthEventAction::Delete,
            vec!["CN=One".into(), "CN=Two".into()],
            "CA certificates deleted. Subjects: CN=One; CN=Two",
        ),
    ] {
        let message = Message::certificate_auth_event_message(
            event(),
            action,
            subjects,
            &data,
            actor(),
            actor(),
        )?;
        assert_record(&message, &public, "CertificateAuthEvent", None)?;
        assert_eq!(message.statement.head.description, description);
    }
    for (message, action) in [
        (
            Message::phone_blacklist_entry_created_message(
                event(),
                PhoneE164String("+12025550123".into()),
                &data,
                actor(),
                actor(),
            )?,
            "added to",
        ),
        (
            Message::phone_blacklist_entry_deleted_message(
                event(),
                PhoneE164String("+12025550123".into()),
                &data,
                actor(),
                actor(),
            )?,
            "deleted from",
        ),
    ] {
        assert_record(&message, &public, "PhoneBlacklistUpdated", None)?;
        assert_eq!(
            message.statement.head.description,
            format!("Phone +12025550123 {action} the phone blacklist")
        );
    }
    let sent = Message::send_template(
        event(),
        election(),
        &data,
        actor(),
        actor(),
        Some("template body".into()),
        Some("area".into()),
    )?;
    assert_record(&sent, &public, "SendCommunications", None)?;
    assert!(
        matches!(sent.statement.body, StatementBody::SendCommunications(Some(ref text)) if text == "template body")
    );
    let legacy = StatementHead::from_body(event(), &StatementBody::SendTemplate);
    assert_eq!(legacy.kind.to_string(), "SendTemplate");
    assert_eq!(legacy.description, "Template sent to user.");
    Ok(())
}

#[test]
fn external_requests_bind_subjects_while_descriptions_omit_private_details() -> Result<()> {
    let (data, public) = signer()?;
    for (direction, name) in [
        (ExtApiRequestDirection::Inbound, "Inbound"),
        (ExtApiRequestDirection::Outbound, "Outbound"),
    ] {
        let message = Message::external_api_request_message(
            event(),
            election(),
            &data,
            actor(),
            actor(),
            direction,
            ExtApiName::Other,
            "SetVoted Failed: private details".into(),
        )?;
        assert_record(&message, &public, "ExternalApiRequest", Some(ELECTION))?;
        assert_eq!(
            message.statement.head.description,
            format!("{name} request SetVoted Failed.")
        );
        match message.statement.body {
            StatementBody::ExternalApiRequest(_, subject, _, _, operation) => {
                assert_eq!(subject.user_id, actor());
                assert_eq!(subject.username, actor());
                assert_eq!(operation, "SetVoted Failed: private details");
            }
            _ => panic!("wrong external request body"),
        }
    }
    Ok(())
}

#[test]
fn reconciliation_preserves_signed_hash_references_and_the_separate_artifact() -> Result<()> {
    let (data, public) = signer()?;
    for (kind, output, action) in [
        (
            ExternalReconciliationKind::PatchGenerated,
            Some("output-hash".into()),
            "External patch generated",
        ),
        (
            ExternalReconciliationKind::ChangesApplied,
            None,
            "Sequent-side changes applied",
        ),
    ] {
        let message = Message::external_reconciliation_message(
            event(),
            kind,
            ExternalReconciliationSequenceString("42".into()),
            ExternalReconciliationGeneratedAtString("1700000000".into()),
            ExternalReconciliationInputHashString("input-hash".into()),
            ExternalReconciliationOutputHashString(output.clone()),
            Some(b"synthetic change list".to_vec()),
            &data,
            actor(),
            actor(),
        )?;
        assert_record(&message, &public, "ExternalReconciliation", None)?;
        assert_eq!(
            message.artifact.as_deref(),
            Some(b"synthetic change list".as_slice())
        );
        assert_eq!(
            message.statement.head.description,
            format!(
                "{action} for reconciliation Sequence 42 (input input-hash, output {}).",
                output.as_deref().unwrap_or("none")
            )
        );
    }
    Ok(())
}
