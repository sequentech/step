// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn results_publication_message_keeps_actor_and_action_details() -> Result<()> {
    let signing_data = SigningData::new(
        StrandSignatureSk::generate()?,
        "admin",
        StrandSignatureSk::generate()?,
    );
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
    let signing_data = SigningData::new(
        StrandSignatureSk::generate()?,
        "windmill",
        StrandSignatureSk::generate()?,
    );
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
