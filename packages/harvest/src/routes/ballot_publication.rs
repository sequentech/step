// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    ballot::{ElectionEventPresentation, LockedDown},
    services::jwt::{has_gold_permission, JwtClaims},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
use windmill::{
    postgres::election_event::get_election_event_by_id,
    services::{
        ballot_styles::ballot_publication::{
            add_ballot_publication, get_ballot_publication_diff,
            update_publish_ballot, PublicationDiff,
        },
        database::get_hasura_pool,
    },
};

/// Request body for generating a ballot publication.
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBallotPublicationInput {
    /// The election event ID
    election_event_id: String,
    /// Optional election ID to filter specific election
    election_id: Option<String>,
}

/// Response containing the generated ballot publication ID.
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBallotPublicationOutput {
    /// The ballot publication ID
    ballot_publication_id: String,
}

/// Generates a new ballot publication for an election event/election.
#[instrument(skip(claims))]
#[post("/generate-ballot-publication", format = "json", data = "<body>")]
pub async fn generate_ballot_publication(
    body: Json<GenerateBallotPublicationInput>,
    claims: JwtClaims,
) -> Result<Json<GenerateBallotPublicationOutput>, (Status, String)> {
    if !has_gold_permission(&claims) {
        return Err((Status::Forbidden, "Insufficient privileges".into()));
    }

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_WRITE],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    if let Some(election_event_presentation) = election_event.presentation {
        info!(
            "election_event_presentation {:?}",
            election_event_presentation
        );
        let maybe_err = deserialize_value::<ElectionEventPresentation>(
            election_event_presentation.clone(),
        );
        info!("presentation err {:?}", maybe_err);

        if deserialize_value::<ElectionEventPresentation>(
            election_event_presentation,
        )
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
        .locked_down
            == Some(LockedDown::LOCKED_DOWN)
        {
            return Err((
                Status::Forbidden,
                "Election event is locked down".to_string(),
            ));
        }
    }

    let ballot_publication_id = add_ballot_publication(
        &hasura_transaction,
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.election_id.clone(),
        user_id.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    Ok(Json(GenerateBallotPublicationOutput {
        ballot_publication_id,
    }))
}

/// Request body for publishing a ballot.
#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotInput {
    /// The election event ID
    election_event_id: String,
    /// The ballot publication ID to publish
    ballot_publication_id: String,
}

/// Response containing the published ballot publication ID.
#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotOutput {
    /// The published ballot publication ID
    ballot_publication_id: String,
}

/// Publishes a ballot publication for voting.
#[instrument(skip(claims))]
#[post("/publish-ballot", format = "json", data = "<body>")]
pub async fn publish_ballot(
    body: Json<PublishBallotInput>,
    claims: JwtClaims,
) -> Result<Json<PublishBallotOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_WRITE],
    )?;
    let input = body.into_inner();

    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = claims.hasura_claims.tenant_id.clone();
        let user_id = claims.hasura_claims.user_id.clone();
        let username = claims.preferred_username.unwrap_or("-".to_string());
        let election_event_id = input.election_event_id.clone();
        let ballot_publication_id = input.ballot_publication_id.clone();
        Box::pin(async move {
            update_publish_ballot(
                hasura_transaction,
                user_id,
                username,
                tenant_id,
                election_event_id,
                ballot_publication_id,
            )
            .await
        })
    })
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error publishing ballot: {error:?}"),
        )
    })?;

    Ok(Json(PublishBallotOutput {
        ballot_publication_id: input.ballot_publication_id.clone(),
    }))
}

/// Request body for retrieving ballot publication changes.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotPublicationChangesInput {
    /// The election event ID
    election_event_id: String,
    /// The ballot publication ID
    ballot_publication_id: String,
    /// Optional limit on number of changes to retrieve
    limit: Option<usize>,
}

/// Ballot styles information of ballot publication.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BallotPublicationStyles {
    /// The ballot publication ID
    ballot_publication_id: String,
    /// The ballot styles as JSON
    ballot_styles: Value,
}

/// Ballot publication changes output.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetBallotPublicationChangesOutput {
    /// Current ballot publication styles
    current: BallotPublicationStyles,
    /// Previous ballot publication styles if any
    previous: Option<BallotPublicationStyles>,
}

/// Retrieves changes between ballot publication versions.
#[instrument(skip(claims))]
#[post("/get-ballot-publication-changes", format = "json", data = "<body>")]
pub async fn get_ballot_publication_changes(
    body: Json<GetBallotPublicationChangesInput>,
    claims: JwtClaims,
) -> Result<Json<PublicationDiff>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_READ],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let diff = get_ballot_publication_diff(
        &hasura_transaction,
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.ballot_publication_id.clone(),
        input.limit,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(diff))
}
