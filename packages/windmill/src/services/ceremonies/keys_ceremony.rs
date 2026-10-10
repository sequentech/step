// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::keys_ceremony::{ElectoralLogKeysCeremonyAudit, PgKeysCeremonyStore};
use crate::adapters::system::{RandomIds, SystemClock};
pub use crate::domain::keys_ceremony::PrivateKeyDownloadUnavailable;
use crate::domain::keys_ceremony::{
    ceremony_policy, ceremony_settings, covers_permission_labels, parse_user_permission_labels,
    unique_permission_labels, validate_election_without_ceremony,
    validate_event_without_ceremonies, validate_new_ceremony_trustees,
    validate_no_default_ceremony, KeygenAuditEntry, NewKeysCeremony,
};
use crate::domain::keys_ceremony::{next_board_step, KeysBoardStep};
use crate::ports::clock::{Clock, IdGenerator};
use crate::ports::keys_ceremony::{KeysCeremonies, KeysCeremonyTasks};
use crate::ports::keys_ceremony::{
    KeysCeremonyAudit, KeysCeremonyElectionEvents, KeysCeremonyElections, KeysCeremonyTrustees,
};
use crate::postgres::keys_ceremony;
use crate::postgres::trustee;
use crate::services::ceremonies::serialize_logs::*;
use crate::services::election_event_board::get_election_event_board;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::{generate_manager, initial_trustees, Committee, DkgBoard, RawTrusteeRecord};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus,
};
use sequent_core::types::hasura::core::KeysCeremony;
use tracing::{event, info, instrument, Level};

/// Immutable custody challenge of a listed trustee. No private keys leave the runner.
pub async fn custody_request(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    trustee_name: &str,
    ceremony: &KeysCeremony,
    tally_session_id: Option<&str>,
) -> Result<protocol_board::CustodyRequest> {
    let trustee = trustee::get_trustee_by_name(transaction, tenant_id, trustee_name).await?;
    let position = ceremony
        .trustee_ids
        .iter()
        .position(|id| id == &trustee.id)
        .ok_or_else(|| anyhow!("trustee is not in the ceremony committee"))?;
    let dkg = crate::services::protocol_board::load_dkg_board(
        transaction,
        tenant_id,
        election_event_id,
        &ceremony.id,
    )
    .await?;
    let keys = dkg.configuration.trustee_public_keys(position)?;
    Ok(protocol_board::CustodyRequest {
        tenant_id: tenant_id.to_owned(),
        election_event_id: election_event_id.to_owned(),
        keys_ceremony_id: ceremony.id.clone(),
        tally_session_id: tally_session_id.map(str::to_owned),
        trustee_name: trustee_name.to_owned(),
        signing_public_key: keys.signing_public_key,
        share_encryption_public_key: keys.share_encryption_public_key,
    })
}

/// Download the public challenge for the native trustee tool.
#[instrument(skip(transaction, claims), err)]
pub async fn get_private_key(
    transaction: &Transaction<'_>,
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<String> {
    use crate::domain::keys_ceremony::{validate_private_key_download, with_key_retrieved};
    let name = claims
        .trustee
        .ok_or_else(|| anyhow!("trustee name not found"))?;
    let ceremony = keys_ceremony::lock_keys_ceremony_by_id(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;
    let status = validate_private_key_download(&ceremony, &name)?;
    let request = custody_request(
        transaction,
        &tenant_id,
        &election_event_id,
        &name,
        &ceremony,
        None,
    )
    .await?;
    let next = with_key_retrieved(
        &status,
        &name,
        append_keys_trustee_download_log(&status.logs, &name),
    );
    keys_ceremony::update_keys_ceremony_status(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
        &serde_json::to_value(next)?,
        &ceremony.execution_status()?.to_string(),
    )
    .await?;
    Ok(serde_json::to_string_pretty(&request)?)
}

/// Check a scoped native custody proof without recording a status change.
pub async fn key_share_matches(
    transaction: &Transaction<'_>,
    trustee_name: Option<String>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
    proof: &str,
) -> Result<bool> {
    let name = trustee_name.ok_or_else(|| anyhow!("trustee name not found"))?;
    let ceremony = keys_ceremony::get_keys_ceremony_by_id(
        transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    )
    .await?;
    crate::domain::keys_ceremony::validate_private_key_check(&ceremony, &name)?;
    custody_request(
        transaction,
        tenant_id,
        election_event_id,
        &name,
        &ceremony,
        None,
    )
    .await?
    .verifies(proof)
}

/// Record confirmed custody, preserving the event's trustee-signature rule.
#[instrument(skip(transaction, claims, proof, signatures), err)]
pub async fn check_private_key(
    transaction: &Transaction<'_>,
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
    proof: String,
    signatures: &crate::domain::trustee_signatures::TrusteeSignatures,
) -> Result<bool> {
    use crate::domain::keys_ceremony::{validate_private_key_check, with_key_checked};
    let name = claims
        .trustee
        .ok_or_else(|| anyhow!("trustee name not found"))?;
    let ceremony = keys_ceremony::lock_keys_ceremony_by_id(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;
    let status = validate_private_key_check(&ceremony, &name)?;
    if !custody_request(
        transaction,
        &tenant_id,
        &election_event_id,
        &name,
        &ceremony,
        None,
    )
    .await?
    .verifies(&proof)?
    {
        return Ok(false);
    }
    let (next, execution) = with_key_checked(
        &status,
        &name,
        append_keys_trustee_check_log(&status.logs, &name),
        signatures,
    );
    keys_ceremony::update_keys_ceremony_status(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
        &serde_json::to_value(next)?,
        &execution.to_string(),
    )
    .await?;
    Ok(true)
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

    // Use the stored trustee order consistently for both ceremony metadata
    // and the signed configuration's immutable trustee positions.
    let keys_ceremony_uuid = ids.new_id();
    let keys_ceremony_id = keys_ceremony_uuid.to_string();
    let raw_trustees = trustees
        .iter()
        .map(|trustee| {
            Ok(RawTrusteeRecord {
                name: trustee
                    .name
                    .clone()
                    .ok_or_else(|| anyhow!("trustee has no name"))?,
                signing_public_key: trustee.public_key.clone(),
                share_encryption_public_key: trustee.share_encryption_public_key.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let committee = Committee::new(&raw_trustees)?;
    let manager = generate_manager();
    let dkg = DkgBoard::new(&keys_ceremony_uuid, &manager, &committee, threshold)?;
    let status = serde_json::to_value(KeysCeremonyStatus {
        stop_date: None,
        public_key: None,
        public_key_hash: None,
        failure: None,
        logs: generate_keys_initial_log_at(&trustee_names, clock.now()),
        trustees: initial_trustees(&committee),
    })?;
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

    store
        .persist_dkg(
            tenant_id,
            election_event_id,
            &keys_ceremony_id,
            &dkg,
            &manager,
        )
        .await?;

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
#[path = "keys_ceremony_tests.rs"]
mod tests;
