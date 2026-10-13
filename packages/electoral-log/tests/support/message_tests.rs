// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn results_publication_message_keeps_actor_and_action_details() -> Result<()> {
    let signing_data = SigningData::new(SigningKey::generate()?, "admin", SigningKey::generate()?);
    let details = ResultsPublicationDetails {
        publication_id: ResultsPublicationIdString("publication-id".to_string()),
        action: ResultsPublicationAction::Revoke,
        route_scope: ResultsPublicationRouteScopeString("event".to_string()),
        route_election_id: ElectionIdString(None),
        access: ResultsPublicationAccessString("public".to_string()),
        visibility_scope: ResultsPublicationVisibilityScopeString("full_event".to_string()),
        contest_ids: vec![ContestIdString("contest-id".to_string())],
    };

    let message = Message::results_publication_action_message(
        EventIdString("event-id".to_string()),
        details,
        &signing_data,
        Some("user-id".to_string()),
        Some("username".to_string()),
    )?;

    assert_eq!(message.user_id.as_deref(), Some("user-id"));
    assert_eq!(message.username.as_deref(), Some("username"));
    assert!(matches!(
        message.statement.body,
        StatementBody::ResultsPublicationAction(ResultsPublicationDetails {
            action: ResultsPublicationAction::Revoke,
            ..
        })
    ));
    Ok(())
}

#[test]
fn only_channel_aware_cast_votes_use_schema_version_two() -> Result<()> {
    let signing_data =
        SigningData::new(SigningKey::generate()?, "windmill", SigningKey::generate()?);
    let legacy = Message::cast_vote_message(
        EventIdString("event-id".to_string()),
        ElectionIdString(Some("election-id".to_string())),
        PseudonymHash::new([1; 64]),
        CastVoteHash::new([2; 64]),
        &signing_data,
        VoterIpString("ip".to_string()),
        VoterCountryString("country".to_string()),
        Some("voter-id".to_string()),
        None,
        "area-id".to_string(),
    )?;
    let with_channel = Message::cast_vote_with_channel_message(
        EventIdString("event-id".to_string()),
        ElectionIdString(Some("election-id".to_string())),
        PseudonymHash::new([1; 64]),
        CastVoteHash::new([2; 64]),
        &signing_data,
        VoterIpString("ip".to_string()),
        VoterCountryString("country".to_string()),
        VotingChannelString("TELEPHONE".to_string()),
        Some("voter-id".to_string()),
        None,
        "area-id".to_string(),
    )?;

    let legacy_row: ElectoralLogMessage = (&legacy).try_into()?;
    let with_channel_row: ElectoralLogMessage = (&with_channel).try_into()?;
    assert_eq!(legacy_row.version, "1");
    assert_eq!(with_channel_row.version, "2");
    assert_eq!(
        with_channel.statement.head.description,
        "Inserted cast vote. Voting channel: TELEPHONE."
    );
    Ok(())
}

#[test]
fn publication_failures_are_signed_errors_with_task_and_publication_context() -> Result<()> {
    let system_sk = SigningKey::generate()?;
    let system_pk = VerificationKey::from_sk(&system_sk)?;
    let signing_data = SigningData::new(system_sk.clone(), "", system_sk);
    for stage in [
        BallotPublicationStage::Generate,
        BallotPublicationStage::Publish,
    ] {
        let mut message = Message::ballot_publication_failure_message(
            EventIdString("event-id".to_string()),
            BallotPublicationFailure {
                publication_id: BallotPublicationIdString("publication-id".to_string()),
                task_id: "task-id".to_string(),
                stage,
                error: ErrorMessageString("Publication failure reason".to_string()),
            },
            &signing_data,
            Some("Admin".to_string()),
        )?;
        message.verify(&system_pk)?;
        assert_eq!(message.statement.head.log_type.to_string(), "ERROR");
        assert_eq!(
            message.statement.head.kind.to_string(),
            "BallotPublicationFailure"
        );
        assert_eq!(message.username.as_deref(), Some("Admin"));
        assert!(message.statement.head.description.contains("task-id"));
        assert!(message
            .statement
            .head
            .description
            .contains("publication-id"));
        assert!(message
            .statement
            .head
            .description
            .contains("Publication failure reason"));
        let encoded = borsh::to_vec(&message)?;
        let decoded: Message = borsh::from_slice(&encoded)?;
        decoded.verify(&system_pk)?;
        let row: ElectoralLogMessage = (&message).try_into()?;
        let persisted: Message = borsh::from_slice(&row.message)?;
        persisted.verify(&system_pk)?;
        assert!(persisted
            .statement
            .head
            .description
            .contains("Publication failure reason"));
        if let StatementBody::BallotPublicationFailure(details) = &mut message.statement.body {
            details.error.0 = "Changed error".to_string();
        }
        assert!(message.verify(&system_pk).is_err());
    }
    Ok(())
}
