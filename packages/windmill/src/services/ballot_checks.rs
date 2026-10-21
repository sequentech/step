// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_box_reads::{
    ballot_box_time, get_event_ballot_box, get_voter_cast_votes, VoterCastVotes, VoterScope,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_reads;
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
    /// The Ballot ID the ballot box signed for a received ballot.
    Received(String),
    Exact(String),
    Prefix(String),
}

impl BallotIdMatch {
    /// How the ballot box matches a Ballot ID of the ballot's hash.
    fn ballot_box_match(&self) -> ballot_box_reads::BallotIdMatch<'_> {
        match self {
            Self::Received(ballot_id) | Self::Exact(ballot_id) => {
                ballot_box_reads::BallotIdMatch::Exact(ballot_id)
            }
            Self::Prefix(prefix) => ballot_box_reads::BallotIdMatch::Prefix(prefix),
        }
    }
}

/// A Ballot ID is the one the ballot box signed for a received ballot, or the
/// hexadecimal hash of the ballot. Anything else matches no ballot and never
/// reaches the ballot box.
pub fn ballot_id_match(ballot_id: &str, telephone_voting: bool) -> Option<BallotIdMatch> {
    if let Some(received_ballot_id) = normalize_ballot_id(ballot_id) {
        return Some(BallotIdMatch::Received(received_ballot_id));
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

/// A cast ballot of the voter whose Ballot ID matches the lookup.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocatedBallot {
    pub ballot_id: String,
    pub content: String,
    pub cast_at: DateTime<Utc>,
}

pub fn locate_ballot_output(
    period: &ChecksPeriod,
    matches: &[LocatedBallot],
) -> LocateBallotOutput {
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
        [ballot] => LocateBallotOutput {
            status: LocateBallotStatus::Found,
            ballot_id: Some(ballot.ballot_id.clone()),
            content: Some(ballot.content.clone()),
            cast_at: Some(ballot.cast_at.to_rfc3339()),
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
#[instrument(skip_all, err)]
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
    let matches = match &ballot_id_match {
        BallotIdMatch::Received(_) => {
            let event =
                get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
            let identity = crate::services::vote_privacy::ballot_identity(&event, voter_id).await?;
            get_event_ballot_box(hasura_transaction, tenant_id, election_event_id)
                .await?
                .store
                .received_cast_ballots(
                    election_event_id,
                    &identity,
                    election_id,
                    ballot_id_match.ballot_box_match(),
                )
                .await?
                .into_iter()
                .filter(|cast| cast.ballot.ballot.area_id == area_id)
                .map(|cast| {
                    Ok(LocatedBallot {
                        ballot_id: cast.received_ballot_id,
                        cast_at: ballot_box_time(cast.ballot.ballot.accepted_at)?,
                        content: cast.ballot.content,
                    })
                })
                .collect::<Result<Vec<_>>>()?
        }
        BallotIdMatch::Exact(_) | BallotIdMatch::Prefix(_) => {
            let election_ids = [election_id.to_string()];
            let scope = VoterScope {
                tenant_id,
                election_event_id,
                voter_id,
                area_id,
                election_ids: &election_ids,
            };
            get_voter_cast_votes(
                hasura_transaction,
                &scope,
                VoterCastVotes::Matching {
                    election_id,
                    ballot_id: ballot_id_match.ballot_box_match(),
                },
            )
            .await?
            .into_iter()
            .map(|vote| {
                Ok(LocatedBallot {
                    ballot_id: vote.ballot_id.context("A cast ballot has no Ballot ID")?,
                    content: vote.content.context("A cast ballot has no content")?,
                    cast_at: vote
                        .created_at
                        .as_deref()
                        .map(DateTime::parse_from_rfc3339)
                        .transpose()?
                        .context("A cast ballot has no cast time")?
                        .with_timezone(&Utc),
                })
            })
            .collect::<Result<Vec<_>>>()?
        }
    };

    Ok(locate_ballot_output(&period, &matches))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn cast_vote(ballot_id: &str) -> LocatedBallot {
        LocatedBallot {
            ballot_id: ballot_id.to_string(),
            content: "encrypted".to_string(),
            cast_at: at("2028-05-08T10:00:00Z"),
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
        assert_eq!(
            prefix.ballot_box_match(),
            ballot_box_reads::BallotIdMatch::Prefix("0abc")
        );
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
                    Some(BallotIdMatch::Received("FTBE-MHRX".to_string())),
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
