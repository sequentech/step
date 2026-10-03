// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ChecksPeriod;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use serde::Deserialize;
use tracing::instrument;
use windmill::services::ballot_checks::{
    get_checks_period, locate_ballot as locate_voter_ballot, LocateBallotOutput,
};
use windmill::services::database::get_hasura_pool;

pub const CHECKS_ENDED_MESSAGE: &str = "checks-ended";
const CHECKS_UNAVAILABLE_MESSAGE: &str = "Unable to check the ballot";

#[derive(Deserialize, Debug)]
pub struct LocateBallotInput {
    pub election_event_id: String,
    pub election_id: String,
    pub ballot_id: String,
}

/// A voter's token is issued for one election event, so a check is only ever
/// answered for that event and for an election the voter can vote in.
pub fn voter_check_scope(
    claims: &JwtClaims,
    election_event_id: &str,
    election_id: &String,
) -> Result<String, (Status, String)> {
    if claims.hasura_claims.election_event_id.as_deref()
        != Some(election_event_id)
    {
        return Err((Status::Forbidden, "Voter not authorized".to_owned()));
    }
    let (area_id, _voting_channel) = authorize_voter_election(
        claims,
        vec![VoterPermissions::CAST_VOTE],
        election_id,
    )?;
    Ok(area_id)
}

pub fn refuse_ended_checks(
    period: &ChecksPeriod,
) -> Result<(), (Status, String)> {
    match period {
        ChecksPeriod::Unlimited | ChecksPeriod::OpenUntil(_) => Ok(()),
        ChecksPeriod::Ended(_) => {
            Err((Status::Forbidden, CHECKS_ENDED_MESSAGE.to_owned()))
        }
    }
}

/// Refuses once the event's period for viewing cast ballots has ended, or
/// when that period cannot be read.
#[instrument(err(Debug))]
pub async fn ensure_checks_open(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), (Status, String)> {
    let period: anyhow::Result<ChecksPeriod> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.transaction().await?;
        let period = get_checks_period(
            &transaction,
            tenant_id,
            election_event_id,
            Utc::now(),
        )
        .await?;
        transaction.commit().await?;
        Ok(period)
    }
    .await;
    let period = period.map_err(|error| {
        tracing::error!(error = %error, "Unable to read the checks period");
        (
            Status::InternalServerError,
            CHECKS_UNAVAILABLE_MESSAGE.to_owned(),
        )
    })?;
    refuse_ended_checks(&period)
}

#[instrument(skip(claims))]
#[post("/locate-ballot", format = "json", data = "<body>")]
pub async fn locate_ballot(
    body: Json<LocateBallotInput>,
    claims: JwtClaims,
) -> Result<Json<LocateBallotOutput>, (Status, String)> {
    let input = body.into_inner();
    let area_id = voter_check_scope(
        &claims,
        &input.election_event_id,
        &input.election_id,
    )?;

    let output: anyhow::Result<LocateBallotOutput> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.transaction().await?;
        let output = locate_voter_ballot(
            &transaction,
            &claims.hasura_claims.tenant_id,
            &input.election_event_id,
            &input.election_id,
            &area_id,
            &claims.hasura_claims.user_id,
            &input.ballot_id,
            Utc::now(),
        )
        .await?;
        transaction.commit().await?;
        Ok(output)
    }
    .await;

    output.map(Json).map_err(|error| {
        tracing::error!(error = %error, "Unable to locate the ballot");
        (
            Status::InternalServerError,
            CHECKS_UNAVAILABLE_MESSAGE.to_owned(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_claims::Claims;
    use chrono::DateTime;

    fn claims() -> JwtClaims {
        Claims::new("tenant", "voter")
            .azp("voting-portal")
            .area("area")
            .election_event("event")
            .authorized_elections(&["election"])
            .roles(["user"])
            .build()
    }

    #[test]
    fn a_check_is_scoped_to_the_voters_event_and_elections() {
        let voter = claims();
        assert_eq!(
            voter_check_scope(&voter, "event", &"election".to_string())
                .unwrap(),
            "area"
        );
        assert_eq!(
            voter_check_scope(&voter, "other", &"election".to_string())
                .unwrap_err()
                .0,
            Status::Forbidden
        );
        assert!(
            voter_check_scope(&voter, "event", &"other".to_string()).is_err()
        );

        let mut without_event = voter.clone();
        without_event.hasura_claims.election_event_id = None;
        assert!(voter_check_scope(
            &without_event,
            "event",
            &"election".to_string()
        )
        .is_err());

        let mut without_role = voter;
        without_role.hasura_claims.allowed_roles.clear();
        assert!(voter_check_scope(
            &without_role,
            "event",
            &"election".to_string()
        )
        .is_err());
    }

    #[test]
    fn only_an_ended_period_is_refused() {
        let until = DateTime::parse_from_rfc3339("2028-06-07T23:59:00+08:00")
            .unwrap()
            .with_timezone(&Utc);
        assert!(refuse_ended_checks(&ChecksPeriod::Unlimited).is_ok());
        assert!(refuse_ended_checks(&ChecksPeriod::OpenUntil(until)).is_ok());
        assert_eq!(
            refuse_ended_checks(&ChecksPeriod::Ended(until)).unwrap_err(),
            (Status::Forbidden, CHECKS_ENDED_MESSAGE.to_owned())
        );
    }
}
