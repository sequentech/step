// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::cast_vote::find_voter_cast_votes_by_ballot_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::cast_votes::CastVote;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{checks_period_from_presentation, ChecksPeriod};
use sequent_core::ballot_receipt::normalize_ballot_id;
use sequent_core::types::hasura::extra::VotingChannels;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::instrument;

/// Telephone voters read out the first characters of the Ballot ID, so in
/// elections with that channel a lookup of this length matches by prefix.
pub const TELEPHONE_BALLOT_ID_PREFIX_LENGTH: usize = 4;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Display, EnumString)]
pub enum LocateBallotStatus {
    #[strum(serialize = "found")]
    #[serde(rename = "found")]
    Found,
    #[strum(serialize = "not-found")]
    #[serde(rename = "not-found")]
    NotFound,
    #[strum(serialize = "ambiguous")]
    #[serde(rename = "ambiguous")]
    Ambiguous,
    #[strum(serialize = "checks-ended")]
    #[serde(rename = "checks-ended")]
    ChecksEnded,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct LocateBallotOutput {
    pub status: LocateBallotStatus,
    pub ballot_id: Option<String>,
    pub content: Option<String>,
    pub cast_at: Option<String>,
    pub checks_available_until: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BallotIdMatch {
    Exact(String),
    Prefix(String),
}

impl BallotIdMatch {
    pub fn like_pattern(&self) -> String {
        match self {
            Self::Exact(ballot_id) => ballot_id.clone(),
            Self::Prefix(prefix) => format!("{prefix}%"),
        }
    }
}

/// A Ballot ID is the one the ballot box signed for a received ballot, or the
/// hexadecimal hash of the ballot. Anything else matches no ballot and never
/// reaches the query as a pattern.
pub fn ballot_id_match(ballot_id: &str, telephone_voting: bool) -> Option<BallotIdMatch> {
    if let Some(received_ballot_id) = normalize_ballot_id(ballot_id) {
        return Some(BallotIdMatch::Exact(received_ballot_id));
    }
    let normalized = ballot_id.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || !normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }
    if telephone_voting && normalized.len() == TELEPHONE_BALLOT_ID_PREFIX_LENGTH {
        Some(BallotIdMatch::Prefix(normalized))
    } else {
        Some(BallotIdMatch::Exact(normalized))
    }
}

fn checks_available_until(period: &ChecksPeriod) -> Option<String> {
    match period {
        ChecksPeriod::Unlimited => None,
        ChecksPeriod::OpenUntil(until) | ChecksPeriod::Ended(until) => Some(until.to_rfc3339()),
    }
}

pub fn locate_ballot_output(period: &ChecksPeriod, matches: &[CastVote]) -> LocateBallotOutput {
    let checks_available_until = checks_available_until(period);
    let without_ballot = |status| LocateBallotOutput {
        status,
        ballot_id: None,
        content: None,
        cast_at: None,
        checks_available_until: checks_available_until.clone(),
    };
    if matches!(period, ChecksPeriod::Ended(_)) {
        return without_ballot(LocateBallotStatus::ChecksEnded);
    }
    match matches {
        [] => without_ballot(LocateBallotStatus::NotFound),
        [cast_vote] => LocateBallotOutput {
            status: LocateBallotStatus::Found,
            ballot_id: cast_vote.ballot_id.clone(),
            content: cast_vote.content.clone(),
            cast_at: cast_vote
                .created_at
                .map(|created_at| created_at.to_rfc3339()),
            checks_available_until,
        },
        _ => without_ballot(LocateBallotStatus::Ambiguous),
    }
}

/// The period in which voters can view their cast ballots, read from the
/// stored event at this instant. A period that cannot be read is an error.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_checks_period(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    now: DateTime<Utc>,
) -> Result<ChecksPeriod> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    checks_period_from_presentation(election_event.presentation.as_ref(), now)
}

/// Looks up one of the voter's own cast ballots, while checks are open.
#[instrument(skip(hasura_transaction), err)]
pub async fn locate_ballot(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    voter_id: &str,
    ballot_id: &str,
    now: DateTime<Utc>,
) -> Result<LocateBallotOutput> {
    let period = get_checks_period(hasura_transaction, tenant_id, election_event_id, now).await?;
    if matches!(period, ChecksPeriod::Ended(_)) {
        return Ok(locate_ballot_output(&period, &[]));
    }

    let election = get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Election not found"))?;
    let telephone_voting = election
        .voting_channels
        .map(serde_json::from_value::<VotingChannels>)
        .transpose()?
        .and_then(|channels| channels.telephone)
        .unwrap_or(false);

    let Some(ballot_id_match) = ballot_id_match(ballot_id, telephone_voting) else {
        return Ok(locate_ballot_output(&period, &[]));
    };
    let matches = find_voter_cast_votes_by_ballot_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        voter_id,
        &ballot_id_match.like_pattern(),
    )
    .await?;

    Ok(locate_ballot_output(&period, &matches))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::cast_votes::CastVoteStatus;

    fn at(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn cast_vote(ballot_id: &str) -> CastVote {
        CastVote {
            id: "cast-vote".to_string(),
            tenant_id: "tenant".to_string(),
            election_id: Some("election".to_string()),
            area_id: Some("area".to_string()),
            created_at: Some(at("2028-05-08T10:00:00Z")),
            last_updated_at: None,
            content: Some("encrypted".to_string()),
            voter_id_string: Some("voter".to_string()),
            election_event_id: "event".to_string(),
            ballot_id: Some(ballot_id.to_string()),
            cast_ballot_signature: None,
            status: CastVoteStatus::Valid,
        }
    }

    #[test]
    fn ballot_ids_match_exactly_in_any_case() {
        assert_eq!(
            ballot_id_match(" 0AbC12 ", false),
            Some(BallotIdMatch::Exact("0abc12".to_string()))
        );
        assert_eq!(
            ballot_id_match("0abc", false),
            Some(BallotIdMatch::Exact("0abc".to_string()))
        );
    }

    #[test]
    fn telephone_elections_match_a_four_character_prefix() {
        let prefix = ballot_id_match("0ABC", true).unwrap();
        assert_eq!(prefix, BallotIdMatch::Prefix("0abc".to_string()));
        assert_eq!(prefix.like_pattern(), "0abc%");
        assert_eq!(
            ballot_id_match("0abc12", true),
            Some(BallotIdMatch::Exact("0abc12".to_string()))
        );
    }

    #[test]
    fn a_ballot_box_ballot_id_matches_exactly_however_it_was_typed() {
        for typed in ["FTBE-MHRX", "ftbe-mhrx", " ftbemhrx ", "FTBE MHRX"] {
            for telephone_voting in [true, false] {
                assert_eq!(
                    ballot_id_match(typed, telephone_voting),
                    Some(BallotIdMatch::Exact("FTBE-MHRX".to_string())),
                    "{typed}"
                );
            }
        }
    }

    #[test]
    fn text_that_is_not_a_ballot_id_matches_nothing() {
        for ballot_id in ["", "  ", "%", "0ab%", "0ab_", "xyz1", "0abc' OR ''='"] {
            assert_eq!(ballot_id_match(ballot_id, true), None, "{ballot_id}");
            assert_eq!(ballot_id_match(ballot_id, false), None, "{ballot_id}");
        }
    }

    #[test]
    fn an_open_period_answers_with_the_ballot_and_nothing_about_the_voter() {
        let until = at("2028-06-07T15:59:00Z");
        let output = locate_ballot_output(&ChecksPeriod::OpenUntil(until), &[cast_vote("0abc12")]);
        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            serde_json::json!({
                "status": "found",
                "ballot_id": "0abc12",
                "content": "encrypted",
                "cast_at": output.cast_at,
                "checks_available_until": "2028-06-07T15:59:00+00:00",
            })
        );
        assert_eq!(
            at(output.cast_at.as_deref().unwrap()),
            at("2028-05-08T10:00:00Z")
        );
    }

    #[test]
    fn unlimited_checks_have_no_date() {
        let output = locate_ballot_output(&ChecksPeriod::Unlimited, &[cast_vote("0abc12")]);
        assert_eq!(output.status, LocateBallotStatus::Found);
        assert_eq!(output.checks_available_until, None);
    }

    #[test]
    fn no_match_and_several_matches_return_no_ballot() {
        let none = locate_ballot_output(&ChecksPeriod::Unlimited, &[]);
        assert_eq!(none.status, LocateBallotStatus::NotFound);
        assert_eq!(none.content, None);

        let several = locate_ballot_output(
            &ChecksPeriod::Unlimited,
            &[cast_vote("0abc12"), cast_vote("0abc34")],
        );
        assert_eq!(several.status, LocateBallotStatus::Ambiguous);
        assert_eq!(several.ballot_id, None);
        assert_eq!(several.content, None);
    }

    #[test]
    fn an_ended_period_returns_no_ballot_even_when_one_matches() {
        let until = at("2028-06-07T15:59:00Z");
        let output = locate_ballot_output(&ChecksPeriod::Ended(until), &[cast_vote("0abc12")]);
        assert_eq!(
            output,
            LocateBallotOutput {
                status: LocateBallotStatus::ChecksEnded,
                ballot_id: None,
                content: None,
                cast_at: None,
                checks_available_until: Some("2028-06-07T15:59:00+00:00".to_string()),
            }
        );
    }
}
