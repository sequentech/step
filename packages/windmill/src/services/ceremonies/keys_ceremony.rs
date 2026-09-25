// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::adapters::keys_ceremony::{
    B4KeysBoard, ElectoralLogKeysCeremonyAudit, PgKeysCeremonyStore,
};
use crate::adapters::system::{RandomIds, SystemClock};
pub use crate::domain::keys_ceremony::PrivateKeyDownloadUnavailable;
use crate::domain::keys_ceremony::{
    awaits_key_generation, ceremony_policy, ceremony_settings, covers_permission_labels,
    initial_status, next_board_step, parse_user_permission_labels, public_key_execution_status,
    stop_date, trustee_key_status, unique_permission_labels, validate_election_without_ceremony,
    validate_event_without_ceremonies, validate_known_trustees, validate_new_ceremony_trustees,
    validate_no_default_ceremony, validate_private_key_check, validate_private_key_download,
    with_key_checked, with_key_retrieved, KeygenAuditEntry, KeysBoardMessage, KeysBoardStep,
    NewKeysCeremony,
};
use crate::ports::clock::{Clock, IdGenerator};
use crate::ports::keys_ceremony::{
    KeysBoard, KeysCeremonies, KeysCeremonyAudit, KeysCeremonyElectionEvents,
    KeysCeremonyElections, KeysCeremonyTasks, KeysCeremonyTrustees,
};
use crate::services::ceremonies::serialize_logs::{
    append_keys_trustee_check_log, append_keys_trustee_download_log, generate_keys_initial_log,
    sort_logs,
};
use crate::services::election_event_board::get_election_event_board;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus, Trustee, TrusteeStatus,
};
use sequent_core::types::hasura::core::{KeysCeremony, Trustee as TrusteeRecord};
use serde_json::Value;
use std::collections::HashSet;
use tracing::instrument;
use tracing::{event, info, Level};

/// A trustee acting on its private key.
pub struct TrusteeKeyRequest<'a> {
    /// The trustee claim of the user, which names the trustee.
    pub trustee: Option<String>,
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub keys_ceremony_id: &'a str,
}

/// A request to create a keys ceremony.
pub struct KeysCeremonyRequest<'a> {
    pub tenant_id: &'a str,
    pub user_id: &'a str,
    pub username: &'a str,
    pub election_event_id: &'a str,
    pub threshold: usize,
    pub trustee_names: Vec<String>,
    /// The election of the ceremony, or `None` for all the elections of the
    /// event.
    pub election_id: Option<String>,
    pub name: Option<String>,
    pub policy: CeremoniesPolicy,
}

/// Whether a board task changed the ceremony, and so has work to commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysCeremonyUpdate {
    Skipped,
    Updated,
}

/// The board of a keys ceremony, and its election when the ceremony is for a
/// single election: the default ceremony uses the election event board, any
/// other the board of its first election.
#[instrument(name = "get_keys_ceremony_board", skip(store, board), err)]
pub async fn keys_ceremony_board<S, B>(
    store: &S,
    board: &B,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<(String, Option<String>)>
where
    S: KeysCeremonyElectionEvents + KeysCeremonyElections,
    B: KeysBoard,
{
    if keys_ceremony.is_default() {
        // fetch election_event
        let election_event = store.election_event(tenant_id, election_event_id).await?;

        // get board name
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;
        Ok((board_name, None))
    } else {
        let election = store
            .first_election_of_keys_ceremony(tenant_id, election_event_id, &keys_ceremony.id)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Can't find election with keys ceremony {}",
                    keys_ceremony.id
                )
            })?;
        let board = board.election_board(tenant_id, &election.id)?;
        Ok((board, Some(election.id)))
    }
}

// returns (board_name, election_id), where the election_id might be None for an event Board
pub async fn get_keys_ceremony_board(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<(String, Option<String>)> {
    keys_ceremony_board(
        &PgKeysCeremonyStore { transaction },
        &B4KeysBoard { transaction },
        tenant_id,
        election_event_id,
        keys_ceremony,
    )
    .await
}

/// Hands a trustee its encrypted private key and records the download.
pub async fn download_private_key<S, B, C>(
    store: &S,
    board: &B,
    clock: &C,
    request: TrusteeKeyRequest<'_>,
) -> Result<String>
where
    S: KeysCeremonies + KeysCeremonyElectionEvents + KeysCeremonyElections + KeysCeremonyTrustees,
    B: KeysBoard,
    C: Clock,
{
    let TrusteeKeyRequest {
        trustee,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    } = request;
    // The trustee name is simply the username of the user
    let trustee_name = trustee.ok_or(anyhow!("trustee name not found"))?;

    // Lock the ceremony until the status update below is committed, so a
    // check committed meanwhile cannot be overwritten
    let keys_ceremony = store
        .lock_keys_ceremony(tenant_id, election_event_id, keys_ceremony_id)
        .await?;
    let current_status = validate_private_key_download(&keys_ceremony, &trustee_name)?;

    let (board_name, _) =
        keys_ceremony_board(store, board, tenant_id, election_event_id, &keys_ceremony).await?;

    let trustee_public_key = store
        .trustee_by_name(tenant_id, &trustee_name)
        .await
        .with_context(|| "can't find trustee in the database")?
        .public_key
        .ok_or(anyhow!("can't get trustee's public key"))?;

    // get the encrypted private key
    let encrypted_private_key = board
        .trustee_encrypted_private_key(board_name.as_str(), trustee_public_key.as_str())
        .await?;

    // Update ceremony with the information that this trustee did get the
    // private key
    let logs = append_keys_trustee_download_log(&current_status.logs, &trustee_name, clock.now());
    let status: Value =
        serde_json::to_value(with_key_retrieved(&current_status, &trustee_name, logs))?;

    store
        .update_keys_ceremony_status(
            tenant_id,
            election_event_id,
            keys_ceremony_id,
            /* status */ &status,
            /* execution_status */
            &keys_ceremony
                .execution_status
                .with_context(|| "empty current execution_status")?,
        )
        .await
        .with_context(|| "couldn't update keys ceremony")?;

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        election_event_id,
        keys_ceremony_id,
        trustee_name
    );
    Ok(encrypted_private_key)
}

#[instrument]
pub async fn get_private_key(
    transaction: &Transaction<'_>,
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<String> {
    download_private_key(
        &PgKeysCeremonyStore { transaction },
        &B4KeysBoard { transaction },
        &SystemClock,
        TrusteeKeyRequest {
            trustee: claims.trustee,
            tenant_id: &tenant_id,
            election_event_id: &election_event_id,
            keys_ceremony_id: &keys_ceremony_id,
        },
    )
    .await
}

/// The encrypted private key of a trustee, as posted on the ceremony board.
#[instrument(name = "find_trustee_private_key", skip(store, board), err)]
pub async fn trustee_private_key<S, B>(
    store: &S,
    board: &B,
    tenant_id: &str,
    election_event_id: &str,
    trustee_name: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<String>
where
    S: KeysCeremonyElectionEvents + KeysCeremonyElections + KeysCeremonyTrustees,
    B: KeysBoard,
{
    let (board_name, _) =
        keys_ceremony_board(store, board, tenant_id, election_event_id, keys_ceremony).await?;

    let trustee_public_key = store
        .trustee_by_name(tenant_id, trustee_name)
        .await?
        .public_key
        .ok_or(anyhow!("can't get trustee public key"))?;

    // get the encrypted private key
    board
        .trustee_encrypted_private_key(board_name.as_str(), trustee_public_key.as_str())
        .await
}

pub async fn find_trustee_private_key(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    trustee_name: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<String> {
    trustee_private_key(
        &PgKeysCeremonyStore { transaction },
        &B4KeysBoard { transaction },
        tenant_id,
        election_event_id,
        trustee_name,
        keys_ceremony,
    )
    .await
}

/// Compares the private key a trustee kept with the one on the board, and
/// records the check when they match.
pub async fn check_trustee_private_key<S, B, C>(
    store: &S,
    board: &B,
    clock: &C,
    request: TrusteeKeyRequest<'_>,
    private_key_base64: &str,
) -> Result<bool>
where
    S: KeysCeremonies + KeysCeremonyElectionEvents + KeysCeremonyElections + KeysCeremonyTrustees,
    B: KeysBoard,
    C: Clock,
{
    let TrusteeKeyRequest {
        trustee,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    } = request;
    // The trustee name is simply the username of the user
    let trustee_name = trustee.ok_or(anyhow!("trustee name not found"))?;

    // Lock the ceremony until the status update below is committed, so a
    // download committed meanwhile cannot overwrite the check
    let keys_ceremony = store
        .lock_keys_ceremony(tenant_id, election_event_id, keys_ceremony_id)
        .await?;
    let current_status = validate_private_key_check(&keys_ceremony, &trustee_name)?;

    // get the encrypted private key
    let encrypted_private_key = trustee_private_key(
        store,
        board,
        tenant_id,
        election_event_id,
        &trustee_name,
        &keys_ceremony,
    )
    .await?;

    if encrypted_private_key != private_key_base64 {
        return Ok(false);
    }

    let logs = append_keys_trustee_check_log(&current_status.logs, &trustee_name, clock.now());
    let (new_status, new_execution_status) = with_key_checked(&current_status, &trustee_name, logs);

    store
        .update_keys_ceremony_status(
            tenant_id,
            election_event_id,
            keys_ceremony_id,
            /* status */ &serde_json::to_value(new_status)?,
            /* execution_status */ &new_execution_status.to_string(),
        )
        .await
        .with_context(|| "couldn't update keys ceremony")?;

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        election_event_id,
        keys_ceremony_id,
        trustee_name
    );
    Ok(true)
}

#[instrument(err)]
pub async fn check_private_key(
    transaction: &Transaction<'_>,
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
    private_key_base64: String,
) -> Result<bool> {
    check_trustee_private_key(
        &PgKeysCeremonyStore { transaction },
        &B4KeysBoard { transaction },
        &SystemClock,
        TrusteeKeyRequest {
            trustee: claims.trustee,
            tenant_id: &tenant_id,
            election_event_id: &election_event_id,
            keys_ceremony_id: &keys_ceremony_id,
        },
        &private_key_base64,
    )
    .await
}

/// Creates a keys ceremony, assigns it to its elections and records it in the
/// electoral log. Returns the id of the new ceremony.
pub async fn start_keys_ceremony<S, A, C, I>(
    store: &S,
    audit: &A,
    clock: &C,
    ids: &I,
    request: KeysCeremonyRequest<'_>,
) -> Result<String>
where
    S: KeysCeremonies + KeysCeremonyTrustees + KeysCeremonyElections + KeysCeremonyElectionEvents,
    A: KeysCeremonyAudit,
    C: Clock,
    I: IdGenerator,
{
    let KeysCeremonyRequest {
        tenant_id,
        user_id,
        username,
        election_event_id,
        threshold,
        trustee_names,
        election_id,
        name,
        policy,
    } = request;

    // verify trustee names and fetch their objects to get their ids
    let trustees = store
        .trustees_by_name(tenant_id, &trustee_names)
        .await
        .with_context(|| "can't find trustees")?;
    validate_new_ceremony_trustees(trustee_names.len(), trustees.len(), threshold)?;

    // get the election event
    let election_event = store.election_event(tenant_id, election_event_id).await?;

    let keys_ceremonies = store
        .keys_ceremonies(tenant_id, election_event_id)
        .await
        .with_context(|| "error listing existing keys ceremonies")?;
    validate_no_default_ceremony(&keys_ceremonies)?;

    // find if there's any previous ceremony and if so, stop. shouldn't happen,
    // we only allow one per election
    if let Some(election_id) = &election_id {
        let election = store
            .election(tenant_id, election_event_id, election_id)
            .await?
            .ok_or(anyhow!("Can't find election"))?;
        validate_election_without_ceremony(election_id, &election)?;
    } else {
        // it's an event ceremony, then there can be no other keys ceremony
        validate_event_without_ceremonies(&keys_ceremonies)?;
    }

    let keys_ceremony_id = ids.new_id().to_string();
    let status = serde_json::to_value(initial_status(
        &trustees,
        generate_keys_initial_log(&trustee_names, clock.now()),
    )?)?;
    let is_default = election_id.is_none();

    let elections = store
        .assign_keys_ceremony(
            tenant_id,
            election_event_id,
            election_id.clone(),
            &keys_ceremony_id,
        )
        .await?;

    store
        .insert_keys_ceremony(NewKeysCeremony {
            id: keys_ceremony_id.clone(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            trustee_ids: trustees.into_iter().map(|trustee| trustee.id).collect(),
            threshold: threshold.try_into()?,
            status,
            execution_status: KeysCeremonyExecutionStatus::default().to_string(),
            name,
            settings: ceremony_settings(&policy),
            is_default,
            permission_labels: unique_permission_labels(elections),
        })
        .await
        .with_context(|| "couldn't insert keys ceremony")?;

    // Save it in the electoral log
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;
    audit
        .keygen(KeygenAuditEntry {
            board_name,
            tenant_id: tenant_id.to_string(),
            stored_election_event_id: election_event.id,
            election_event_id: election_event_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            election_id,
        })
        .await?;

    Ok(keys_ceremony_id)
}

#[instrument(err)]
pub async fn create_keys_ceremony(
    transaction: &Transaction<'_>,
    tenant_id: String,
    user_id: &str,
    username: &str,
    election_event_id: String,
    threshold: usize,
    trustee_names: Vec<String>,
    election_id: Option<String>,
    name: Option<String>,
    is_automatic_ceremony: bool,
) -> Result<String> {
    start_keys_ceremony(
        &PgKeysCeremonyStore { transaction },
        &ElectoralLogKeysCeremonyAudit { transaction },
        &SystemClock,
        &RandomIds,
        KeysCeremonyRequest {
            tenant_id: &tenant_id,
            user_id,
            username,
            election_event_id: &election_event_id,
            threshold,
            trustee_names,
            election_id,
            name,
            policy: ceremony_policy(is_automatic_ceremony),
        },
    )
    .await
}

/// Whether the user holds every permission label of the election, or of all
/// the elections of the event.
pub async fn has_election_permission_labels<E: KeysCeremonyElections>(
    elections: &E,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    user_permission_labels: Option<String>,
) -> Result<bool> {
    let elections_permission_label = elections
        .election_permission_labels(tenant_id, election_event_id, election_id)
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election permissionlabel {:?}", e))?;

    let user_permission_labels_vec = parse_user_permission_labels(user_permission_labels)?;

    info!(elections_permission_label = ?elections_permission_label);
    info!(user_permission_labels_vec = ?user_permission_labels_vec);

    Ok(covers_permission_labels(
        &elections_permission_label,
        &user_permission_labels_vec,
    ))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn validate_permission_labels(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    user_permission_labels: Option<String>,
) -> Result<bool> {
    has_election_permission_labels(
        &PgKeysCeremonyStore {
            transaction: hasura_transaction,
        },
        tenant_id,
        election_event_id,
        election_id,
        user_permission_labels,
    )
    .await
}

/// Posts the configuration of a started ceremony to its board, unless the
/// board already has one, and moves the ceremony in progress.
pub async fn generate_keys<S, B>(
    store: &S,
    board: &B,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<KeysCeremonyUpdate>
where
    S: KeysCeremonies + KeysCeremonyTrustees + KeysCeremonyElectionEvents + KeysCeremonyElections,
    B: KeysBoard,
{
    let keys_ceremony = store
        .keys_ceremony(tenant_id, election_event_id, keys_ceremony_id)
        .await
        .with_context(|| "error finding keys ceremony")?;

    let trustees = store
        .trustees_by_id(tenant_id, &keys_ceremony.trustee_ids)
        .await?;
    info!("trustees: {:?}", trustees);
    let trustee_pks: Vec<String> = trustees
        .into_iter()
        .filter_map(|trustee| trustee.public_key)
        .collect();
    info!("trustee_pks: {:?}", trustee_pks);

    let (board_name, _) =
        keys_ceremony_board(store, board, tenant_id, election_event_id, &keys_ceremony).await?;

    let execution_status = keys_ceremony.execution_status()?;
    let status = keys_ceremony.status()?;

    // check config is not already created
    if !awaits_key_generation(&execution_status, &status) {
        info!("Unexpected status: {}", execution_status);
        return Ok(KeysCeremonyUpdate::Skipped);
    }

    if !board.configuration_exists(board_name.as_str()).await? {
        // create config/keys for board
        board
            .create_keys(
                tenant_id,
                election_event_id,
                board_name.as_str(),
                trustee_pks,
                keys_ceremony.threshold as usize,
            )
            .await?;
    }

    store
        .update_keys_ceremony_status(
            tenant_id,
            election_event_id,
            &keys_ceremony.id,
            &serde_json::to_value(status)?,
            &KeysCeremonyExecutionStatus::IN_PROGRESS.to_string(),
        )
        .await?;

    Ok(KeysCeremonyUpdate::Updated)
}

/// Reads the keys the trustees generated on the board into an in-progress
/// ceremony that has no public key yet.
pub async fn record_public_key<S, B, C>(
    store: &S,
    board: &B,
    clock: &C,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<KeysCeremonyUpdate>
where
    S: KeysCeremonies + KeysCeremonyTrustees + KeysCeremonyElectionEvents + KeysCeremonyElections,
    B: KeysBoard,
    C: Clock,
{
    let keys_ceremony = store
        .keys_ceremony(tenant_id, election_event_id, keys_ceremony_id)
        .await?;
    let current_status = keys_ceremony.status()?;
    if current_status.public_key.is_some() {
        info!("Public key already set");
        return Ok(KeysCeremonyUpdate::Skipped);
    }
    let execution_status = keys_ceremony.execution_status()?;
    if execution_status != KeysCeremonyExecutionStatus::IN_PROGRESS {
        info!("Unexpected status {}", execution_status);
        return Ok(KeysCeremonyUpdate::Skipped);
    }
    let (board_name, _) =
        keys_ceremony_board(store, board, tenant_id, election_event_id, &keys_ceremony).await?;
    let public_key_opt = board.public_key(&board_name).await.ok();
    // verify trustee names and fetch their objects to get their ids
    let trustee_names = current_status
        .trustees
        .iter()
        .map(|trustee| trustee.name.clone())
        .collect::<HashSet<String>>();
    let trustees_by_name = store
        .trustees_by_name(
            tenant_id,
            &trustee_names.iter().cloned().collect::<Vec<_>>(),
        )
        .await?;
    // we should have a list with the same trustees
    validate_known_trustees(&trustee_names, &trustees_by_name)?;

    // Timestamp since last update.
    let next_timestamp = keys_ceremony
        .last_updated_at
        .with_context(|| "empty last_updated_at")?
        .timestamp() as u64;

    let board_messages = board
        .public_key_messages(&board_name, next_timestamp)
        .await?;
    let mut logs = current_status.logs.clone();
    logs.extend(board_messages.logs);

    let new_execution_status =
        public_key_execution_status(&keys_ceremony.policy(), public_key_opt.as_deref());

    let new_status: Value = serde_json::to_value(KeysCeremonyStatus {
        stop_date: Some(stop_date(clock.now())),
        public_key: public_key_opt,
        logs: sort_logs(&logs),
        trustees: current_status
            .trustees
            .iter()
            .map(|trustee| {
                Ok(Trustee {
                    name: trustee.name.clone(),
                    status: get_trustee_status(
                        board,
                        &trustee.name,
                        &trustees_by_name,
                        &board_messages.messages,
                    )?,
                })
            })
            .collect::<Result<Vec<Trustee>>>()?,
    })?;

    // update public key
    store
        .update_keys_ceremony_status(
            tenant_id,
            election_event_id,
            keys_ceremony_id,
            &new_status,
            &new_execution_status.to_string(),
        )
        .await?;

    Ok(KeysCeremonyUpdate::Updated)
}

#[instrument(skip(board, trustees_hasura, messages), err)]
fn get_trustee_status<B: KeysBoard>(
    board: &B,
    trustee_name: &str,
    trustees_hasura: &[TrusteeRecord],
    messages: &[KeysBoardMessage<B::Sender>],
) -> Result<TrusteeStatus> {
    let Some(found_trustee) = trustees_hasura
        .iter()
        .find(|trustee| trustee.name.as_deref() == Some(trustee_name))
    else {
        return Ok(TrusteeStatus::WAITING);
    };
    let Some(pk_str) = &found_trustee.public_key else {
        return Ok(TrusteeStatus::WAITING);
    };
    Ok(trustee_key_status(&board.trustee_sender(pk_str)?, messages))
}

/// Queues the next board task of every keys ceremony of the election event.
pub async fn dispatch_keys_ceremony_tasks<S, T>(
    store: &S,
    tasks: &T,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()>
where
    S: KeysCeremonies,
    T: KeysCeremonyTasks,
{
    let keys_ceremonies = store.keys_ceremonies(tenant_id, election_event_id).await?;

    for keys_ceremony in keys_ceremonies {
        let status = keys_ceremony.status()?;
        let execution_status = keys_ceremony.execution_status()?;
        match next_board_step(&execution_status, &status) {
            Some(KeysBoardStep::CreateKeys) => {
                // create the public keys in async task
                let task_id = tasks
                    .send_create_keys(tenant_id, election_event_id, &keys_ceremony.id)
                    .await?;
                event!(Level::INFO, "Sent create_keys task {}", task_id);
            }
            Some(KeysBoardStep::SetPublicKey) => {
                let task_id = tasks
                    .send_set_public_key(tenant_id, election_event_id, &keys_ceremony.id)
                    .await?;
                event!(
                    Level::INFO,
                    "Sent set_public_key task {} for keys ceremony {}",
                    task_id,
                    keys_ceremony.id
                );
            }
            None => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRUSTEE_NAME: &str = "trustee1";

    fn keys_ceremony(
        execution_status: KeysCeremonyExecutionStatus,
        trustee_status: TrusteeStatus,
    ) -> KeysCeremony {
        let status = KeysCeremonyStatus {
            stop_date: None,
            public_key: Some("public-key".to_string()),
            logs: vec![],
            trustees: vec![
                Trustee {
                    name: TRUSTEE_NAME.to_string(),
                    status: trustee_status,
                },
                Trustee {
                    name: "trustee2".to_string(),
                    status: TrusteeStatus::KEY_GENERATED,
                },
            ],
        };

        KeysCeremony {
            id: "keys-ceremony".to_string(),
            created_at: None,
            last_updated_at: None,
            tenant_id: "tenant".to_string(),
            election_event_id: "election-event".to_string(),
            trustee_ids: vec![],
            status: Some(serde_json::to_value(status).expect("serializable status")),
            execution_status: Some(execution_status.to_string()),
            labels: None,
            annotations: None,
            threshold: 2,
            name: None,
            settings: None,
            is_default: None,
            permission_label: None,
        }
    }

    fn is_download_unavailable(result: &Result<KeysCeremonyStatus>) -> bool {
        result.as_ref().is_err_and(|error| {
            error
                .downcast_ref::<PrivateKeyDownloadUnavailable>()
                .is_some()
        })
    }

    #[test]
    fn allows_download_until_the_trustee_checks_the_key() {
        for trustee_status in [TrusteeStatus::KEY_GENERATED, TrusteeStatus::KEY_RETRIEVED] {
            let ceremony = keys_ceremony(KeysCeremonyExecutionStatus::IN_PROGRESS, trustee_status);

            assert!(validate_private_key_download(&ceremony, TRUSTEE_NAME).is_ok());
        }
    }

    #[test]
    fn rejects_download_when_the_ceremony_is_not_in_progress() {
        for execution_status in [
            KeysCeremonyExecutionStatus::USER_CONFIGURATION,
            KeysCeremonyExecutionStatus::STARTED,
            KeysCeremonyExecutionStatus::SUCCESS,
            KeysCeremonyExecutionStatus::CANCELLED,
        ] {
            let ceremony = keys_ceremony(execution_status, TrusteeStatus::KEY_CHECKED);

            assert!(is_download_unavailable(&validate_private_key_download(
                &ceremony,
                TRUSTEE_NAME
            )));
        }
    }

    #[test]
    fn rejects_download_after_the_trustee_checked_the_key() {
        let ceremony = keys_ceremony(
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            TrusteeStatus::KEY_CHECKED,
        );

        assert!(is_download_unavailable(&validate_private_key_download(
            &ceremony,
            TRUSTEE_NAME
        )));
    }

    #[test]
    fn reports_a_trustee_outside_the_ceremony_as_a_failure() {
        let ceremony = keys_ceremony(
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            TrusteeStatus::KEY_GENERATED,
        );

        let result = validate_private_key_download(&ceremony, "trustee3");

        assert!(result.is_err());
        assert!(!is_download_unavailable(&result));
    }

    #[test]
    fn reports_invalid_ceremony_data_as_a_failure() {
        let mut unknown_execution_status = keys_ceremony(
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            TrusteeStatus::KEY_GENERATED,
        );
        unknown_execution_status.execution_status = Some("UNKNOWN".to_string());

        let mut missing_execution_status = unknown_execution_status.clone();
        missing_execution_status.execution_status = None;

        let mut malformed_status = keys_ceremony(
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            TrusteeStatus::KEY_GENERATED,
        );
        malformed_status.status = Some(serde_json::json!({"trustees": "invalid"}));

        for ceremony in [
            unknown_execution_status,
            missing_execution_status,
            malformed_status,
        ] {
            let result = validate_private_key_download(&ceremony, TRUSTEE_NAME);

            assert!(result.is_err());
            assert!(!is_download_unavailable(&result));
        }
    }
}

#[cfg(test)]
#[path = "keys_ceremony_tests.rs"]
mod use_case_tests;
