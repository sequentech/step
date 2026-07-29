// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use tracing::{error, instrument};
use windmill::services;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::external::api_datafix::{
    acquire_inbound_voter_lock, audit_inbound_operation,
    audit_inbound_operation_standalone, ensure_inbound_reenable_is_safe,
    ensure_voter_has_no_active_vote, release_inbound_voter_lock,
    valid_inbound_voting_channel, InboundVoterLock,
};
use windmill::services::external::datafix_types::*;
use windmill::services::external::utils::get_event_id_and_datafix_annotations;

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
        DatafixResponse::error(DatafixErrorCode::Forbidden)
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);

    let result = services::external::api_datafix::add_datafix_voter(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
        &election_event_id,
        &realm,
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
        DatafixResponse::error(DatafixErrorCode::Forbidden)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let InboundVoterLock {
        lock,
        election_event_id,
        realm,
        ..
    } = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(resolved) => resolved,
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
                return Err(DatafixResponse::error(
                    DatafixErrorCode::InternalError,
                ));
            }
        };
    let transaction_result = hasura_db_client.transaction().await;
    if let Some(err) =
        transaction_result.as_ref().err().map(ToString::to_string)
    {
        error!("Error starting hasura transaction {err}");
        drop(transaction_result);
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "UpdateVoter",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
    }
    let hasura_transaction =
        transaction_result.expect("transaction result was checked above");

    let guard_result = match input.enabled {
        Some(true) => {
            ensure_inbound_reenable_is_safe(
                &hasura_transaction,
                &keycloak_transaction,
                &claims,
                &input.voter_id,
            )
            .await
        }
        Some(false) => {
            ensure_voter_has_no_active_vote(
                &hasura_transaction,
                &keycloak_transaction,
                &claims,
                &input.voter_id,
            )
            .await
        }
        None => Ok(()),
    };
    if let Err(err) = guard_result {
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
    let result = services::external::api_datafix::update_datafix_voter(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
        &election_event_id,
        &realm,
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
        DatafixResponse::error(DatafixErrorCode::Forbidden)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let InboundVoterLock { lock, realm, .. } = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(resolved) => resolved,
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
                return Err(DatafixResponse::error(
                    DatafixErrorCode::InternalError,
                ));
            }
        };
    let transaction_result = hasura_db_client.transaction().await;
    if let Some(err) =
        transaction_result.as_ref().err().map(ToString::to_string)
    {
        error!("Error starting hasura transaction {err}");
        drop(transaction_result);
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "DeleteVoter",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
    }
    let hasura_transaction =
        transaction_result.expect("transaction result was checked above");

    if let Err(err) = ensure_voter_has_no_active_vote(
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
            "DeleteVoter",
            false,
        )
        .await;
        drop(hasura_transaction);
        drop(hasura_db_client);
        release_inbound_voter_lock(lock).await;
        return Err(err);
    }
    let result = services::external::api_datafix::disable_datafix_voter(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
        &realm,
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
        DatafixResponse::error(DatafixErrorCode::Forbidden)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let InboundVoterLock { lock, realm, .. } = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(resolved) => resolved,
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
                return Err(DatafixResponse::error(
                    DatafixErrorCode::InternalError,
                ));
            }
        };
    let transaction_result = hasura_db_client.transaction().await;
    if let Some(err) =
        transaction_result.as_ref().err().map(ToString::to_string)
    {
        error!("Error starting hasura transaction {err}");
        drop(transaction_result);
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "UnmarkVoted",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
    }
    let hasura_transaction =
        transaction_result.expect("transaction result was checked above");

    if let Err(err) = ensure_voter_has_no_active_vote(
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
            "UnmarkVoted",
            false,
        )
        .await;
        drop(hasura_transaction);
        drop(hasura_db_client);
        release_inbound_voter_lock(lock).await;
        return Err(err);
    }
    let result = services::external::api_datafix::unmark_voter_as_voted(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
        &realm,
    )
    .await;
    audit_inbound_operation(
        &hasura_transaction,
        Some(&keycloak_transaction),
        &claims,
        &input.voter_id,
        "UnmarkVoted",
        result.is_ok(),
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    release_inbound_voter_lock(lock).await;
    result
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
        DatafixResponse::error(DatafixErrorCode::Forbidden)
    })?;

    if !valid_inbound_voting_channel(&input.channel) {
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "MarkVoted",
            false,
        )
        .await;
        return Err(DatafixResponse::error(DatafixErrorCode::InvalidRequest));
    }

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let InboundVoterLock { lock, realm, .. } = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(resolved) => resolved,
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
                return Err(DatafixResponse::error(
                    DatafixErrorCode::InternalError,
                ));
            }
        };
    let transaction_result = hasura_db_client.transaction().await;
    if let Some(err) =
        transaction_result.as_ref().err().map(ToString::to_string)
    {
        error!("Error starting hasura transaction {err}");
        drop(transaction_result);
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "MarkVoted",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
    }
    let hasura_transaction =
        transaction_result.expect("transaction result was checked above");

    if let Err(err) = ensure_voter_has_no_active_vote(
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
            "MarkVoted",
            false,
        )
        .await;
        drop(hasura_transaction);
        drop(hasura_db_client);
        release_inbound_voter_lock(lock).await;
        return Err(err);
    }
    let result = services::external::api_datafix::mark_as_voted_via_channel(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
        &realm,
    )
    .await;
    audit_inbound_operation(
        &hasura_transaction,
        Some(&keycloak_transaction),
        &claims,
        &input.voter_id,
        "MarkVoted",
        result.is_ok(),
    )
    .await;
    drop(hasura_transaction);
    drop(hasura_db_client);
    release_inbound_voter_lock(lock).await;
    result
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
        DatafixResponse::error(DatafixErrorCode::Forbidden)
    })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let InboundVoterLock {
        lock,
        election_event_id,
        realm,
        datafix_annotations,
    } = match acquire_inbound_voter_lock(
        &keycloak_transaction,
        &claims,
        &input.voter_id,
    )
    .await
    {
        Ok(resolved) => resolved,
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
                return Err(DatafixResponse::error(
                    DatafixErrorCode::InternalError,
                ));
            }
        };
    let transaction_result = hasura_db_client.transaction().await;
    if let Some(err) =
        transaction_result.as_ref().err().map(ToString::to_string)
    {
        error!("Error starting hasura transaction {err}");
        drop(transaction_result);
        drop(hasura_db_client);
        audit_inbound_operation_standalone(
            &claims,
            &input.voter_id,
            "ReplacePin",
            false,
        )
        .await;
        release_inbound_voter_lock(lock).await;
        return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
    }
    let hasura_transaction =
        transaction_result.expect("transaction result was checked above");

    let result = services::external::api_datafix::replace_voter_pin(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
        &election_event_id,
        &realm,
        &datafix_annotations,
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
