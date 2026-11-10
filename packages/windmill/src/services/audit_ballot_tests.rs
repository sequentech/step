// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::{TimeZone, Utc};
use electoral_log::messages::statement::StatementBody;
use voting_crypto::RecordDeserialize;

const EVENT_ID: &str = "00000000-0000-4000-8000-00000000000e";
const ELECTION_ID: &str = "00000000-0000-4000-8000-000000000001";
const AREA_ID: &str = "00000000-0000-4000-8000-0000000000a1";
const RECEIVED_ID: &str = "00000000-0000-4000-8000-0000000000b1";

fn received(status: ReceivedStatus) -> ReceivedBallot {
    ReceivedBallot {
        id: RECEIVED_ID.into(),
        election_event_id: EVENT_ID.into(),
        election_id: ELECTION_ID.into(),
        area_id: AREA_ID.into(),
        voter_id: "voter".into(),
        ballot_id: "FTBE-MHRX".into(),
        ballot_hash: "ab".repeat(32),
        content: "{}".into(),
        voter_signing_pk: "voter-key".into(),
        voter_ballot_signature: "voter-signature".into(),
        received_at: Utc.timestamp_millis_opt(1_841_000_000_000).unwrap(),
        key_id: "fd110d301d2f077d".into(),
        received_signature: "ballot-box-signature".into(),
        status,
        cast_at: None,
        cast_signature: None,
        cast_receipt_signature: None,
    }
}

fn answer() -> AuditBallotOutput {
    AuditBallotOutput {
        ballot_id: "FTBE-MHRX".into(),
        ballot_hash: "ab".repeat(32),
    }
}

#[test]
fn only_a_ballot_waiting_to_be_cast_is_marked_and_an_audited_one_answers_again() {
    assert!(found_received(&received(ReceivedStatus::Received)).is_none());
    assert_eq!(
        found_received(&received(ReceivedStatus::Audited))
            .unwrap()
            .unwrap(),
        answer()
    );
    assert!(matches!(
        found_received(&received(ReceivedStatus::Cast)),
        Some(Err(CastVoteError::BallotAlreadyCast))
    ));
}

#[test]
fn a_ballot_cast_while_it_was_being_audited_stays_cast() {
    let ballot = received(ReceivedStatus::Received);
    assert_eq!(
        audit_answer(AuditOutcome::Audited, &ballot).unwrap(),
        answer()
    );
    assert_eq!(
        audit_answer(AuditOutcome::AlreadyAudited, &ballot).unwrap(),
        answer()
    );
    assert!(matches!(
        audit_answer(AuditOutcome::Cast, &ballot),
        Err(CastVoteError::BallotAlreadyCast)
    ));
    assert!(matches!(
        audit_answer(AuditOutcome::NotReceived, &ballot),
        Err(CastVoteError::BallotNotReceived)
    ));
}

#[test]
fn the_audit_record_names_the_voter_the_ballot_and_its_receipt() {
    let key = SigningKey::generate().unwrap();
    let ballot = received(ReceivedStatus::Received);
    let ip = Some("192.0.2.10".to_string());
    let country = Some("ES".to_string());
    let username = Some("voter.one".to_string());
    let entry = audit_record(
        "board",
        &ballot,
        PseudonymHash::new([1; 64]),
        CastVoteHash::new([0xab; 64]),
        &AuditRequest {
            voting_channel: VotingStatusChannel::ONLINE,
            voter_ip: &ip,
            voter_country: &country,
            username: &username,
        },
        &key,
    )
    .unwrap();

    assert_eq!(
        entry.delivery_id,
        format!("ballot-audit:{EVENT_ID}:{RECEIVED_ID}")
    );
    let stored = &entry.message;
    assert_eq!(stored.statement_kind, "BallotAudited");
    assert_eq!(stored.user_id.as_deref(), Some("voter"));
    assert_eq!(stored.username.as_deref(), Some("voter.one"));
    assert_eq!(stored.election_id.as_deref(), Some(ELECTION_ID));
    assert_eq!(stored.area_id.as_deref(), Some(AREA_ID));
    // The same ballot reference a cast-vote record of this ballot carries.
    assert_eq!(stored.ballot_id.as_deref(), Some("ab".repeat(32).as_str()));

    let message = Message::record_deserialize(&stored.message).unwrap();
    assert_eq!(message.sender.name, "voter");
    match message.statement.body {
        StatementBody::BallotAudited(election, _, _, ballot_id, ip, country, channel) => {
            assert_eq!(election.0.as_deref(), Some(ELECTION_ID));
            assert_eq!(ballot_id, "FTBE-MHRX");
            assert_eq!(ip.0, "ip: 192.0.2.10");
            assert_eq!(country.0, "country: ES");
            assert_eq!(channel.0, "ONLINE");
        }
        other => panic!("unexpected statement {other:?}"),
    }
}
