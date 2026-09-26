// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The cast vote route with a scripted insertion: who it inserts for, the
//! retries, the Datafix follow-up task and the answer each error maps to.

use crate::adapters::memory::cast_votes::{Attempt, ScriptedCastVotes};
use crate::adapters::memory::task_queue::MemoryTaskQueue;
use crate::route_services::{bearer, json, Services};
use crate::test_claims::Claims;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::types::permissions::VoterPermissions;
use serde_json::{json, Value};
use windmill::services::cast_votes::{CastVote, CastVoteStatus};
use windmill::services::insert_cast_vote::{
    CastVoteError, InsertCastVoteResult,
};

const TENANT_ID: &str = "00000000-0000-4000-8000-00000000000a";
const EVENT_ID: &str = "00000000-0000-4000-8000-00000000000e";
const ELECTION_ID: &str = "00000000-0000-4000-8000-000000000001";
const AREA_ID: &str = "00000000-0000-4000-8000-0000000000a1";
const VOTER_ID: &str = "voter";

fn voter() -> Claims {
    Claims::new(TENANT_ID, VOTER_ID)
        .azp("voting-portal")
        .username("voter.one")
        .auth_time(1727270400)
        .area(AREA_ID)
        .election_event(EVENT_ID)
        .authorized_elections(&[ELECTION_ID])
        .roles([VoterPermissions::CAST_VOTE])
}

fn cast_vote(status: CastVoteStatus) -> CastVote {
    CastVote {
        id: "00000000-0000-4000-8000-0000000000c1".into(),
        tenant_id: TENANT_ID.into(),
        election_id: Some(ELECTION_ID.into()),
        area_id: Some(AREA_ID.into()),
        created_at: None,
        last_updated_at: None,
        content: Some("ballot".into()),
        voter_id_string: Some(VOTER_ID.into()),
        election_event_id: EVENT_ID.into(),
        ballot_id: Some("ballot-id".into()),
        cast_ballot_signature: None,
        status,
    }
}

async fn cast<'c>(client: &'c Client, claims: &Claims) -> LocalResponse<'c> {
    client
        .post("/insert-cast-vote")
        .header(ContentType::JSON)
        .header(bearer(claims))
        .header(Header::new("CF-Connecting-IP", "192.0.2.10"))
        .header(Header::new("CF-IPCountry", "ES"))
        .body(
            json!({"ballot_id": "ballot-id", "election_id": ELECTION_ID, "content": "ballot"})
                .to_string(),
        )
        .dispatch()
        .await
}

fn attempt() -> Attempt {
    Attempt {
        ballot_id: "ballot-id".into(),
        election_id: ELECTION_ID.into(),
        content: "ballot".into(),
        tenant_id: TENANT_ID.into(),
        auth_time: Some(1727270400),
        username: Some("voter.one".into()),
        voter_id: VOTER_ID.into(),
        area_id: AREA_ID.into(),
        voting_channel: VotingStatusChannel::ONLINE,
        voter_ip: Some("192.0.2.10".into()),
        voter_country: Some("ES".into()),
    }
}

#[rocket::async_test]
async fn a_valid_vote_is_inserted_for_the_voter_and_needs_no_follow_up() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::answering([Ok(InsertCastVoteResult::Success(
            cast_vote(CastVoteStatus::Valid),
        ))]),
    );
    let client = services.client().await;

    let (status, body) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        body,
        json!({
            "id": "00000000-0000-4000-8000-0000000000c1",
            "tenant_id": TENANT_ID,
            "election_id": ELECTION_ID,
            "area_id": AREA_ID,
            "created_at": null,
            "last_updated_at": null,
            "content": "ballot",
            "voter_id_string": VOTER_ID,
            "election_event_id": EVENT_ID,
            "ballot_id": "ballot-id",
            "cast_ballot_signature": null,
            "status": "valid",
        })
    );
    assert_eq!(services.cast_votes.attempts(), vec![attempt()]);
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_vote_pending_datafix_is_sent_for_processing() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::answering([Ok(
            InsertCastVoteResult::PendingDatafix(cast_vote(
                CastVoteStatus::InProgress,
            )),
        )]),
    );
    let client = services.client().await;

    let (status, _) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "process_cast_vote");
    assert_eq!(
        sent[0].kwargs,
        json!({
            "tenant_id": TENANT_ID,
            "election_event_id": EVENT_ID,
            "cast_vote_id": "00000000-0000-4000-8000-0000000000c1",
        })
    );
}

#[rocket::async_test]
async fn a_pending_vote_is_accepted_when_its_processing_cannot_be_enqueued() {
    // The vote is committed; the review beat enqueues it later.
    let services = Services::without_database()
        .with_tasks(MemoryTaskQueue::refusing())
        .with_cast_votes(ScriptedCastVotes::answering([Ok(
            InsertCastVoteResult::PendingDatafix(cast_vote(
                CastVoteStatus::InProgress,
            )),
        )]));
    let client = services.client().await;

    let (status, _) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
}

#[rocket::async_test]
async fn a_failed_attempt_is_retried() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::answering([
            Err(CastVoteError::GetDbClientFailed("pool exhausted".into())),
            Ok(InsertCastVoteResult::Success(cast_vote(
                CastVoteStatus::Valid,
            ))),
        ]),
    );
    let client = services.client().await;

    let (status, _) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(services.cast_votes.attempts().len(), 2);
}

#[rocket::async_test]
async fn a_voter_not_authorized_for_the_election_inserts_nothing() {
    let services = Services::without_database();
    let client = services.client().await;
    let outsider = Claims::new(TENANT_ID, VOTER_ID)
        .azp("voting-portal")
        .username("voter.one")
        .auth_time(1727270400)
        .area(AREA_ID)
        .election_event(EVENT_ID)
        .authorized_elections(&["00000000-0000-4000-8000-000000000002"])
        .roles([VoterPermissions::CAST_VOTE]);

    let (status, body) = json(cast(&client, &outsider).await).await;
    assert_eq!(status, Status::Unauthorized);
    assert_eq!(body["extensions"]["code"], "Unauthorized");
    assert!(services.cast_votes.attempts().is_empty());
}

/// Every error an insertion reports without retrying, and its answer.
fn error_answers() -> Vec<(CastVoteError, Status, &'static str, &'static str)> {
    use CastVoteError::*;
    let detail = || "detail".to_string();
    vec![
        (
            AreaNotFound,
            Status::NotFound,
            "Area not found",
            "AreaNotFound",
        ),
        (
            ElectionEventNotFound(detail()),
            Status::NotFound,
            "Election Event Not Found",
            "ElectionEventNotFound",
        ),
        (
            InvalidDatafixConfiguration(detail()),
            Status::InternalServerError,
            "Invalid Datafix election event configuration",
            "InternalServerError",
        ),
        (
            ElectoralLogNotFound(detail()),
            Status::NotFound,
            "Electoral Log Not Found",
            "ElectoralLogNotFound",
        ),
        (
            CheckStatusFailed("voting is closed".into()),
            Status::Unauthorized,
            "voting is closed",
            "CheckStatusFailed",
        ),
        (
            VotingChannelNotEnabled(detail()),
            Status::Unauthorized,
            "CheckStatusFailed",
            "CheckStatusFailed",
        ),
        (
            CheckStatusInternalFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            VoterStateLocked(detail()),
            Status::Conflict,
            "The voter state is being updated; retry the vote",
            "CheckStatusFailed",
        ),
        (
            CheckPreviousVotesFailed("already voted".into()),
            Status::BadRequest,
            "already voted",
            "CheckPreviousVotesFailed",
        ),
        (
            CheckRevotesFailed("too many".into()),
            Status::BadRequest,
            "too many",
            "CheckRevotesFailed",
        ),
        (
            CheckVotesInOtherAreasFailed("other area".into()),
            Status::BadRequest,
            "other area",
            "CheckVotesInOtherAreasFailed",
        ),
        (
            InsertFailedExceedsAllowedRevotes,
            Status::BadRequest,
            "InsertFailedExceedsAllowedRevotes",
            "InsertFailedExceedsAllowedRevotes",
        ),
        (
            InsertFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            CommitFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            GetDbClientFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            GetClientCredentialsFailed(detail()),
            Status::Unauthorized,
            "GetClientCredentialsFailed",
            "GetClientCredentialsFailed",
        ),
        (
            GetAreaIdFailed(detail()),
            Status::BadRequest,
            "GetAreaIdFailed",
            "GetAreaIdFailed",
        ),
        (
            GetTransactionFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "GetTransactionFailed",
        ),
        (
            DeserializeBallotFailed(detail()),
            Status::BadRequest,
            "DeserializeBallotFailed",
            "DeserializeBallotFailed",
        ),
        (
            DeserializeContestsFailed(detail()),
            Status::BadRequest,
            "DeserializeContestsFailed",
            "DeserializeContestsFailed",
        ),
        (
            DeserializeAreaPresentationFailed(detail()),
            Status::BadRequest,
            "DeserializeAreaPresentationFailed",
            "DeserializeAreaPresentationFailed",
        ),
        (
            SerializeVoterIdFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            SerializeBallotFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            PokValidationFailed(detail()),
            Status::BadRequest,
            "PokValidationFailed",
            "PokValidationFailed",
        ),
        (
            BallotSignFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            BallotVoterSignatureFailed(detail()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
        (
            UuidParseFailed(detail(), detail()),
            Status::BadRequest,
            "UuidParseFailed",
            "UuidParseFailed",
        ),
        (
            UnknownError(detail()),
            Status::InternalServerError,
            "UnknownError",
            "UnknownError",
        ),
        (
            BallotIdMismatch("hash mismatch".into()),
            Status::BadRequest,
            "hash mismatch",
            "BallotIdMismatch",
        ),
    ]
}

#[rocket::async_test]
async fn each_insertion_error_has_its_own_answer() {
    for (error, status, message, code) in error_answers() {
        let services = Services::without_database().with_cast_votes(
            ScriptedCastVotes::answering([Ok(
                InsertCastVoteResult::SkipRetryFailure(error),
            )]),
        );
        let client = services.client().await;

        let answer: (Status, Value) = json(cast(&client, &voter()).await).await;
        assert_eq!(
            answer,
            (
                status,
                json!({"message": message, "extensions": {"code": code}})
            ),
            "{code}"
        );
        // Terminal errors are not retried.
        assert_eq!(services.cast_votes.attempts().len(), 1, "{code}");
    }
}

#[rocket::async_test]
async fn a_failed_cast_logs_its_error_and_elapsed_time() {
    use crate::request_boundaries::{is_isolated_child, run_isolated};
    if !is_isolated_child() {
        let log = run_isolated(
            "routes::insert_cast_vote::route_tests::a_failed_cast_logs_its_error_and_elapsed_time",
            "http://127.0.0.1:1",
        );
        let elapsed: Vec<_> = log
            .lines()
            .filter_map(|line| {
                line.split_once("insert-cast-vote took ")?.1.strip_suffix(
                    " ms to complete but failed with error=AreaNotFound",
                )
            })
            .collect();
        assert_eq!(elapsed.len(), 1, "{log}");
        // Zero milliseconds is valid; no wall-clock advance is required.
        elapsed[0].parse::<u128>().expect("elapsed milliseconds");
        return;
    }
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::answering([Ok(
            InsertCastVoteResult::SkipRetryFailure(CastVoteError::AreaNotFound),
        )]),
    );
    let client = services
        .client_with_log_level(rocket::config::LogLevel::Normal)
        .await;
    assert_eq!(
        json(cast(&client, &voter()).await).await,
        (
            Status::NotFound,
            json!({"message": "Area not found", "extensions": {"code": "AreaNotFound"}}),
        ),
    );
    assert_eq!(services.cast_votes.attempts().len(), 1);
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn exhausted_retries_return_the_last_error_without_enqueuing() {
    let mut outcomes: Vec<_> = (0..5)
        .map(|_| Err(CastVoteError::GetDbClientFailed("pool exhausted".into())))
        .collect();
    outcomes.push(Err(CastVoteError::UnknownError("last attempt".into())));
    let services = Services::without_database()
        .with_cast_votes(ScriptedCastVotes::answering(outcomes));
    let client = services.client().await;

    assert_eq!(
        json(cast(&client, &voter()).await).await,
        (
            Status::InternalServerError,
            json!({"message": "UnknownError", "extensions": {"code": "UnknownError"}}),
        ),
    );
    assert_eq!(services.cast_votes.attempts().len(), 6);
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn only_telephone_votes_use_token_issue_time_when_auth_time_is_absent() {
    for (client_id, channel, auth_time) in [
        ("voting-portal", VotingStatusChannel::ONLINE, None),
        (
            "ivr-voting",
            VotingStatusChannel::TELEPHONE,
            Some(1_900_000_000),
        ),
        ("voting-portal-kiosk", VotingStatusChannel::KIOSK, None),
    ] {
        let services = Services::without_database().with_cast_votes(
            ScriptedCastVotes::answering([Ok(InsertCastVoteResult::Success(
                cast_vote(CastVoteStatus::Valid),
            ))]),
        );
        let client = services.client().await;
        let claims = Claims::new(TENANT_ID, VOTER_ID)
            .azp(client_id)
            .area(AREA_ID)
            .election_event(EVENT_ID)
            .authorized_elections(&[ELECTION_ID])
            .roles([VoterPermissions::CAST_VOTE]);
        let (status, _) = json(cast(&client, &claims).await).await;
        assert_eq!(status, Status::Ok, "{client_id}");
        let mut expected = attempt();
        expected.voting_channel = channel;
        expected.auth_time = auth_time;
        expected.username = None;
        assert_eq!(
            services.cast_votes.attempts(),
            vec![expected],
            "{client_id}"
        );
    }
}
