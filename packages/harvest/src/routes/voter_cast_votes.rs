// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use electoral_log::adapters::ballot_box_reads::BallotIdMatch;
use rocket::{http::Status, serde::json::Json};
use sequent_core::services::{
    authorization::authorize_voter_event, jwt::JwtClaims,
};
use sequent_core::types::permissions::VoterPermissions;
use serde::Deserialize;
use windmill::services::{
    ballot_box_reads::{
        get_voter_cast_votes as read_voter_cast_votes, VoterCastVote,
        VoterCastVotes, VoterScope,
    },
    database::get_hasura_pool,
};

#[derive(Deserialize)]
pub struct VoterCastVotesInput {
    election_event_id: String,
    election_id: Option<String>,
    ballot_id: Option<String>,
    ballot_id_prefix: Option<String>,
}

/// Which cast votes the input asks for: all, or those of an election with a
/// ballot ID or ballot ID prefix.
fn requested(
    input: &VoterCastVotesInput,
) -> Result<VoterCastVotes<'_>, (Status, String)> {
    let bad_request = || {
        (
            Status::BadRequest,
            "Give either no election, or an election with a ballot ID or a \
             non-empty ballot ID prefix"
                .to_owned(),
        )
    };
    match (
        input.election_id.as_deref(),
        input.ballot_id.as_deref(),
        input.ballot_id_prefix.as_deref(),
    ) {
        (None, None, None) => Ok(VoterCastVotes::All),
        (Some(election_id), Some(ballot_id), None) => {
            Ok(VoterCastVotes::Matching {
                election_id,
                ballot_id: BallotIdMatch::Exact(ballot_id),
            })
        }
        (Some(election_id), None, Some(prefix)) if !prefix.is_empty() => {
            Ok(VoterCastVotes::Matching {
                election_id,
                ballot_id: BallotIdMatch::Prefix(prefix),
            })
        }
        _ => Err(bad_request()),
    }
}

/// The voter's own cast votes, from the event's ballot box.
#[post("/get-voter-cast-votes", format = "json", data = "<body>")]
pub async fn get_voter_cast_votes(
    body: Json<VoterCastVotesInput>,
    claims: JwtClaims,
) -> Result<Json<Vec<VoterCastVote>>, (Status, String)> {
    let (area_id, election_ids) = authorize_voter_event(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &body.election_event_id,
    )?;
    let which = requested(&body)?;
    let scope = VoterScope {
        tenant_id: &claims.hasura_claims.tenant_id,
        election_event_id: &body.election_event_id,
        voter_id: &claims.hasura_claims.user_id,
        area_id: &area_id,
        election_ids: &election_ids,
    };
    let result: anyhow::Result<Vec<VoterCastVote>> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction =
            client.build_transaction().read_only(true).start().await?;
        let votes = read_voter_cast_votes(&transaction, &scope, which).await?;
        transaction.commit().await?;
        Ok(votes)
    }
    .await;
    result.map(Json).map_err(|error| {
        tracing::error!(error = %error, "Unable to read the voter's cast votes");
        (
            Status::InternalServerError,
            "Unable to read the voter's cast votes".to_owned(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        election_id: Option<&str>,
        ballot_id: Option<&str>,
        ballot_id_prefix: Option<&str>,
    ) -> VoterCastVotesInput {
        VoterCastVotesInput {
            election_event_id: "event".into(),
            election_id: election_id.map(String::from),
            ballot_id: ballot_id.map(String::from),
            ballot_id_prefix: ballot_id_prefix.map(String::from),
        }
    }

    #[test]
    fn requests_name_all_votes_or_an_election_and_a_ballot() {
        assert!(matches!(
            requested(&input(None, None, None)),
            Ok(VoterCastVotes::All)
        ));
        assert!(matches!(
            requested(&input(Some("e"), Some("abcd"), None)),
            Ok(VoterCastVotes::Matching {
                election_id: "e",
                ballot_id: BallotIdMatch::Exact("abcd")
            })
        ));
        assert!(matches!(
            requested(&input(Some("e"), None, Some("ab"))),
            Ok(VoterCastVotes::Matching {
                election_id: "e",
                ballot_id: BallotIdMatch::Prefix("ab")
            })
        ));
        for refused in [
            input(Some("e"), None, None),
            input(None, Some("abcd"), None),
            input(Some("e"), Some("abcd"), Some("ab")),
            input(Some("e"), None, Some("")),
        ] {
            assert_eq!(requested(&refused).unwrap_err().0, Status::BadRequest);
        }
    }
}
