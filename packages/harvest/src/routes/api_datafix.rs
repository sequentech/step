// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::keycloak::{
    ATTR_RESET_VALUE, DISABLE_COMMENT, DISABLE_REASON_SET_NOT_VOTED_PENDING,
    VOTED_CHANNEL_INTERNET_VALUE,
};
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use tracing::{error, instrument};
use uuid::Uuid;
use windmill::postgres::cast_vote::{
    finalize_voter_release, get_voter_cast_vote_state,
    quarantine_valid_cast_votes, DatafixPendingOperation,
};
use windmill::services;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::datafix::types::*;
use windmill::services::datafix::utils::{
    datafix_voter_lock_key, get_event_id_and_datafix_annotations, get_user_id,
    post_operation_result_to_electoral_log, voted_via_internet,
    voted_via_not_internet_channel, DATAFIX_VOTER_LOCK_SECS,
};
use windmill::services::pg_lock::PgLock;

async fn acquire_inbound_voter_lock(
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<PgLock, JsonErrorResponse> {
    let lock_key = {
        let mut hasura_client: DbClient = get_hasura_pool().await.get().await.map_err(|err| {
            error!("Error getting Hasura client for the inbound Datafix lock: {err}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
        let hasura_transaction = hasura_client.transaction().await.map_err(|err| {
            error!("Error starting Hasura transaction for the inbound Datafix lock: {err}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
        let (election_event_id, _) = get_event_id_and_datafix_annotations(
            &hasura_transaction,
            &claims.tenant_id,
            &claims.datafix_event_id,
        )
        .await?;
        let realm = get_event_realm(&claims.tenant_id, &election_event_id);
        let user_id =
            get_user_id(keycloak_transaction, &realm, username).await?;
        datafix_voter_lock_key(&claims.tenant_id, &election_event_id, &user_id)
    };
    PgLock::acquire(
        lock_key,
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| {
        error!("Another operation is updating this Datafix voter: {err}");
        DatafixResponse::new(Status::Conflict)
    })
}

async fn release_inbound_voter_lock(lock: PgLock) {
    if let Err(err) = lock.release().await {
        error!("Unable to release the inbound Datafix voter lock: {err}");
    }
}

async fn renew_inbound_voter_lock(
    lock: &PgLock,
) -> Result<(), JsonErrorResponse> {
    lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| {
            error!("The inbound Datafix voter lock was lost: {err}");
            DatafixResponse::new(Status::Conflict)
        })
}

async fn discard_inbound_voter_cast_votes(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<(), JsonErrorResponse> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id).map_err(|err| {
        error!("Invalid tenant ID while discarding Datafix cast votes: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;
    let election_event_id = parse_uuid_v4(&election_event_id).map_err(|err| {
        error!("Invalid election event ID while discarding Datafix cast votes: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    finalize_voter_release(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &user_id,
    )
        .await
        .map_err(|err| {
            error!("Error discarding cast votes for an inbound Datafix operation: {err}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
    Ok(())
}

async fn quarantine_inbound_voter_cast_votes(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
    pending_operation: DatafixPendingOperation,
) -> Result<Vec<Uuid>, JsonErrorResponse> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id).map_err(|err| {
        error!(
            "Invalid tenant ID while quarantining Datafix cast votes: {err}"
        );
        DatafixResponse::new(Status::InternalServerError)
    })?;
    let election_event_id = parse_uuid_v4(&election_event_id).map_err(|err| {
        error!("Invalid election event ID while quarantining Datafix cast votes: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let cast_vote_ids = quarantine_valid_cast_votes(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &user_id,
        pending_operation,
    )
    .await
    .map_err(|err| {
        error!("Error quarantining cast votes for an inbound Datafix operation: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;
    Ok(cast_vote_ids)
}

async fn ensure_inbound_reenable_is_safe(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<(), JsonErrorResponse> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id)
        .map_err(|_| DatafixResponse::new(Status::InternalServerError))?;
    let election_event_uuid = parse_uuid_v4(&election_event_id)
        .map_err(|_| DatafixResponse::new(Status::InternalServerError))?;
    let state = get_voter_cast_vote_state(
        hasura_transaction,
        &tenant_id,
        &election_event_uuid,
        &user_id,
    )
    .await
    .map_err(|err| {
        error!("Error checking unresolved votes before enabling a Datafix voter: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;
    let client = KeycloakAdminClient::new().await.map_err(|err| {
        error!("Error creating a Keycloak client before enabling a Datafix voter: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;
    let user = client.get_user(&realm, &user_id).await.map_err(|err| {
        error!("Error loading a Datafix voter before enabling it: {err}");
        DatafixResponse::new(Status::InternalServerError)
    })?;
    let attributes = user.attributes.unwrap_or_default();
    let pending_release = matches!(
        attributes
            .get(DISABLE_COMMENT)
            .and_then(|values| values.last()),
        Some(value) if value == DISABLE_REASON_SET_NOT_VOTED_PENDING
    );
    if state.has_unresolved_vote
        || state.has_valid_vote
        || pending_release
        || voted_via_internet(&attributes)
        || voted_via_not_internet_channel(&attributes)
    {
        return Err(DatafixResponse::new(Status::Conflict));
    }
    Ok(())
}

async fn audit_inbound_operation(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: Option<&Transaction<'_>>,
    claims: &DatafixClaims,
    username: &str,
    operation_name: &str,
    succeeded: bool,
) {
    let election_event_id = match get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await
    {
        Ok((election_event_id, _)) => election_event_id,
        Err(err) => {
            error!("Unable to resolve the election event for the inbound Datafix audit entry: {err:?}");
            return;
        }
    };
    let user_id = if let Some(transaction) = keycloak_transaction {
        let realm = get_event_realm(&claims.tenant_id, &election_event_id);
        get_user_id(transaction, &realm, username).await.ok()
    } else {
        None
    };
    let outcome = if succeeded { "Succeeded" } else { "Failed" };

    if let Err(err) = post_operation_result_to_electoral_log(
        hasura_transaction,
        &claims.tenant_id,
        &election_event_id,
        user_id.as_deref(),
        username,
        ExtApiRequestDirection::Inbound,
        format!("{operation_name} {outcome}"),
    )
    .await
    {
        error!("Unable to record the inbound Datafix {operation_name} audit entry: {err}");
    }
}

async fn audit_inbound_operation_standalone(
    claims: &DatafixClaims,
    username: &str,
    operation_name: &str,
    succeeded: bool,
) {
    let mut client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            error!("Unable to get a Hasura client for the inbound Datafix audit: {err}");
            return;
        }
    };
    let transaction = match client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Unable to start a transaction for the inbound Datafix audit: {err}");
            return;
        }
    };
    audit_inbound_operation(
        &transaction,
        None,
        claims,
        username,
        operation_name,
        succeeded,
    )
    .await;
}

async fn complete_inbound_voter_vote_change(
    lock: PgLock,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
    operation_name: &str,
    result: Result<Json<DatafixResponse>, JsonErrorResponse>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    if result.is_err() {
        audit_inbound_operation_standalone(
            claims,
            username,
            operation_name,
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        return result;
    }

    let completion: Result<(), JsonErrorResponse> = async {
        renew_inbound_voter_lock(&lock).await?;
        let mut client: DbClient = get_hasura_pool().await.get().await.map_err(|err| {
            error!("Error getting Hasura client to finalize inbound Datafix votes: {err}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
        let transaction = client.transaction().await.map_err(|err| {
            error!("Error starting transaction to finalize inbound Datafix votes: {err}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
        discard_inbound_voter_cast_votes(
            &transaction,
            keycloak_transaction,
            claims,
            username,
        )
        .await?;
        transaction.commit().await.map_err(|err| {
            error!("Error committing inbound Datafix cast-vote finalization: {err}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
        Ok(())
    }
    .await;

    audit_inbound_operation_standalone(
        claims,
        username,
        operation_name,
        completion.is_ok(),
    )
    .await;
    release_inbound_voter_lock(lock).await;
    completion?;
    result
}

fn valid_inbound_voting_channel(channel: &str) -> bool {
    let channel = channel.trim();
    !channel.is_empty()
        && !channel.eq_ignore_ascii_case(ATTR_RESET_VALUE)
        && !channel.eq_ignore_ascii_case(VOTED_CHANNEL_INTERNET_VALUE)
}

#[instrument(skip_all)]
#[post("/add-voter", format = "json", data = "<body>")]
pub async fn add_voter(
    claims: DatafixClaims,
    body: Json<VoterInformationBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterInformationBody = body.into_inner();

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let result = services::datafix::api_datafix::add_datafix_voter(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
    )
    .await;
    audit_inbound_operation(
        &hasura_transaction,
        None,
        &claims,
        &input.voter_id,
        "AddVoter",
        result.is_ok(),
    )
    .await;
    result
}

#[instrument(skip_all)]
#[post("/update-voter", format = "json", data = "<body>")]
pub async fn update_voter(
    claims: DatafixClaims,
    body: Json<VoterInformationBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterInformationBody = body.into_inner();

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let lock = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(lock) => lock,
        Err(err) => {
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "UpdateVoter",
                false,
            )
            .await;
            return Err(err);
        }
    };

    let mut hasura_db_client: DbClient =
        match get_hasura_pool().await.get().await {
            Ok(client) => client,
            Err(err) => {
                error!("Error getting hasura client {err}");
                audit_inbound_operation_standalone(
                    &claims,
                    &input.voter_id,
                    "UpdateVoter",
                    false,
                )
                .await;
                release_inbound_voter_lock(lock).await;
                return Err(DatafixResponse::new(Status::InternalServerError));
            }
        };
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Error starting hasura transaction {err}");
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "UpdateVoter",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };

    if input.enabled == Some(true) {
        if let Err(err) = ensure_inbound_reenable_is_safe(
            &hasura_transaction,
            &keycloak_transaction,
            &claims,
            &input.voter_id,
        )
        .await
        {
            audit_inbound_operation(
                &hasura_transaction,
                Some(&keycloak_transaction),
                &claims,
                &input.voter_id,
                "UpdateVoter",
                false,
            )
            .await;
            drop(hasura_transaction);
            drop(hasura_db_client);
            release_inbound_voter_lock(lock).await;
            return Err(err);
        }
    }
    let result = services::datafix::api_datafix::update_datafix_voter(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
    )
    .await;
    audit_inbound_operation(
        &hasura_transaction,
        Some(&keycloak_transaction),
        &claims,
        &input.voter_id,
        "UpdateVoter",
        result.is_ok(),
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    release_inbound_voter_lock(lock).await;
    result
}

#[derive(Deserialize, Debug)]
pub struct VoterIdBody {
    voter_id: String,
}

#[instrument(skip_all)]
#[post("/delete-voter", format = "json", data = "<body>")]
pub async fn delete_voter(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let lock = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(lock) => lock,
        Err(err) => {
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "DeleteVoter",
                false,
            )
            .await;
            return Err(err);
        }
    };

    let mut hasura_db_client: DbClient =
        match get_hasura_pool().await.get().await {
            Ok(client) => client,
            Err(err) => {
                error!("Error getting hasura client {err}");
                audit_inbound_operation_standalone(
                    &claims,
                    &input.voter_id,
                    "DeleteVoter",
                    false,
                )
                .await;
                release_inbound_voter_lock(lock).await;
                return Err(DatafixResponse::new(Status::InternalServerError));
            }
        };
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Error starting hasura transaction {err}");
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "DeleteVoter",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };

    let result = services::datafix::api_datafix::disable_datafix_voter(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
    )
    .await;
    audit_inbound_operation(
        &hasura_transaction,
        Some(&keycloak_transaction),
        &claims,
        &input.voter_id,
        "DeleteVoter",
        result.is_ok(),
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    release_inbound_voter_lock(lock).await;
    result
}

#[instrument(skip_all)]
#[post("/unmark-voted", format = "json", data = "<body>")]
pub async fn unmark_voted(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let lock = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(lock) => lock,
        Err(err) => {
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "UnmarkVoted",
                false,
            )
            .await;
            return Err(err);
        }
    };

    let mut hasura_db_client: DbClient =
        match get_hasura_pool().await.get().await {
            Ok(client) => client,
            Err(err) => {
                error!("Error getting hasura client {err}");
                audit_inbound_operation_standalone(
                    &claims,
                    &input.voter_id,
                    "UnmarkVoted",
                    false,
                )
                .await;
                release_inbound_voter_lock(lock).await;
                return Err(DatafixResponse::new(Status::InternalServerError));
            }
        };
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Error starting hasura transaction {err}");
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "UnmarkVoted",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };

    let _quarantined_cast_vote_ids = match quarantine_inbound_voter_cast_votes(
        &hasura_transaction,
        &keycloak_transaction,
        &claims,
        &input.voter_id,
        DatafixPendingOperation::InboundUnmarkVoted,
    )
    .await
    {
        Ok(ids) => ids,
        Err(err) => {
            audit_inbound_operation(
                &hasura_transaction,
                Some(&keycloak_transaction),
                &claims,
                &input.voter_id,
                "UnmarkVoted",
                false,
            )
            .await;
            drop(hasura_transaction);
            drop(hasura_db_client);
            release_inbound_voter_lock(lock).await;
            return Err(err);
        }
    };
    if let Err(err) = hasura_transaction.commit().await {
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "UnmarkVoted",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        error!("Error committing inbound Datafix cast-vote quarantine: {err}");
        return Err(DatafixResponse::new(Status::InternalServerError));
    }
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "UnmarkVoted",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            error!(
                "Error starting the inbound Datafix service transaction: {err}"
            );
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };
    let result = services::datafix::api_datafix::unmark_voter_as_voted(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    complete_inbound_voter_vote_change(
        lock,
        &keycloak_transaction,
        &claims,
        &input.voter_id,
        "UnmarkVoted",
        result,
    )
    .await
}

#[instrument(skip_all)]
#[post("/mark-voted", format = "json", data = "<body>")]
pub async fn mark_voted(
    claims: DatafixClaims,
    body: Json<MarkVotedBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: MarkVotedBody = body.into_inner();

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    if !valid_inbound_voting_channel(&input.channel) {
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "MarkVoted",
            false,
        )
        .await;
        return Err(DatafixResponse::new(Status::BadRequest));
    }

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let lock = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(lock) => lock,
        Err(err) => {
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "MarkVoted",
                false,
            )
            .await;
            return Err(err);
        }
    };

    let mut hasura_db_client: DbClient =
        match get_hasura_pool().await.get().await {
            Ok(client) => client,
            Err(err) => {
                error!("Error getting hasura client {err}");
                audit_inbound_operation_standalone(
                    &claims,
                    &input.voter_id,
                    "MarkVoted",
                    false,
                )
                .await;
                release_inbound_voter_lock(lock).await;
                return Err(DatafixResponse::new(Status::InternalServerError));
            }
        };
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Error starting hasura transaction {err}");
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "MarkVoted",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };

    let _quarantined_cast_vote_ids = match quarantine_inbound_voter_cast_votes(
        &hasura_transaction,
        &keycloak_transaction,
        &claims,
        &input.voter_id,
        DatafixPendingOperation::InboundMarkVoted,
    )
    .await
    {
        Ok(ids) => ids,
        Err(err) => {
            audit_inbound_operation(
                &hasura_transaction,
                Some(&keycloak_transaction),
                &claims,
                &input.voter_id,
                "MarkVoted",
                false,
            )
            .await;
            drop(hasura_transaction);
            drop(hasura_db_client);
            release_inbound_voter_lock(lock).await;
            return Err(err);
        }
    };
    if let Err(err) = hasura_transaction.commit().await {
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "MarkVoted",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        error!("Error committing inbound Datafix cast-vote quarantine: {err}");
        return Err(DatafixResponse::new(Status::InternalServerError));
    }
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "MarkVoted",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            error!(
                "Error starting the inbound Datafix service transaction: {err}"
            );
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };
    let result = services::datafix::api_datafix::mark_as_voted_via_channel(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    complete_inbound_voter_vote_change(
        lock,
        &keycloak_transaction,
        &claims,
        &input.voter_id,
        "MarkVoted",
        result,
    )
    .await
}

#[derive(Serialize, Debug)]
pub struct ReplacePinOutput {
    pin: String,
}

#[instrument(skip_all)]
#[post("/replace-pin", format = "json", data = "<body>")]
pub async fn replace_pin(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<ReplacePinOutput>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();
    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let lock = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(lock) => lock,
        Err(err) => {
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "ReplacePin",
                false,
            )
            .await;
            return Err(err);
        }
    };

    let mut hasura_db_client: DbClient =
        match get_hasura_pool().await.get().await {
            Ok(client) => client,
            Err(err) => {
                error!("Error getting hasura client {err}");
                audit_inbound_operation_standalone(
                    &claims,
                    &input.voter_id,
                    "ReplacePin",
                    false,
                )
                .await;
                release_inbound_voter_lock(lock).await;
                return Err(DatafixResponse::new(Status::InternalServerError));
            }
        };
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Error starting hasura transaction {err}");
            drop(hasura_db_client);
            audit_inbound_operation_standalone(
                &claims,
                &input.voter_id,
                "ReplacePin",
                false,
            )
            .await;
            release_inbound_voter_lock(lock).await;
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };

    let result = services::datafix::api_datafix::replace_voter_pin(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
    )
    .await;
    audit_inbound_operation(
        &hasura_transaction,
        Some(&keycloak_transaction),
        &claims,
        &input.voter_id,
        "ReplacePin",
        result.is_ok(),
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    release_inbound_voter_lock(lock).await;
    let pin = result?;

    Ok(Json(ReplacePinOutput { pin }))
}

#[cfg(test)]
mod tests {
    use super::valid_inbound_voting_channel;

    #[test]
    fn inbound_voting_channel_rejects_reserved_values() {
        for channel in [
            "",
            " ",
            "NONE",
            "none",
            "Internet",
            "INTERNET",
            " Internet ",
        ] {
            assert!(!valid_inbound_voting_channel(channel));
        }
        assert!(valid_inbound_voting_channel("PHONE"));
        assert!(valid_inbound_voting_channel("Paper"));
    }
}
