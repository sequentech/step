// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{
    get_election_by_id, get_election_permission_label, set_election_keys_ceremony,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony;
use crate::postgres::protocol_board::{insert_protocol_board, NewProtocolBoard};
use crate::postgres::trustee;
use crate::services::ceremonies::serialize_logs::*;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_board::save_manager_key;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::{generate_manager, initial_trustees, Committee, DkgBoard, RawTrusteeRecord};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus, TrusteeStatus,
};
use sequent_core::types::hasura::core::KeysCeremony;
use serde_json::Value;
use std::collections::HashSet;
use tracing::info;
use tracing::instrument;
use uuid::Uuid;

const MANUAL_CEREMONY_ERROR: &str = "the trustee private key steps are not available yet";

#[instrument(err)]
pub async fn get_private_key(
    transaction: &Transaction<'_>,
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<String> {
    // The trustee name is simply the username of the user
    let trustee_name = claims.trustee.ok_or(anyhow!("trustee name not found"))?;

    // get the keys ceremonies for this election event
    let keys_ceremony = keys_ceremony::get_keys_ceremony_by_id(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;
    // check keys_ceremony has correct execution status
    if keys_ceremony.execution_status()? != KeysCeremonyExecutionStatus::IN_PROGRESS {
        return Err(anyhow!(
            "Keys ceremony status should be in ExecutionStatus::IN_PROGRESS which is set when config message has been added to the board and trustees are working."
        ));
    }

    // get ceremony status
    let current_status: KeysCeremonyStatus = keys_ceremony
        .status()
        .with_context(|| "error parsing keys ceremony current status")?;

    // check the trustee is part of this ceremony
    if !current_status
        .trustees
        .iter()
        .any(|trustee| trustee.name == trustee_name)
    {
        return Err(anyhow!("Trustee not part of the keys ceremony"));
    }

    Err(anyhow!(MANUAL_CEREMONY_ERROR))
}

#[instrument(skip(_transaction), err)]
pub async fn find_trustee_private_key(
    _transaction: &Transaction<'_>,
    _tenant_id: &str,
    _election_event_id: &str,
    _trustee_name: &str,
    _keys_ceremony: &KeysCeremony,
) -> Result<String> {
    Err(anyhow!(MANUAL_CEREMONY_ERROR))
}

#[instrument(err)]
pub async fn check_private_key(
    transaction: &Transaction<'_>,
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
    // Nothing can be read back to compare this against yet.
    _private_key_base64: String,
) -> Result<bool> {
    // The trustee name is simply the username of the user
    let trustee_name = claims.trustee.ok_or(anyhow!("trustee name not found"))?;

    // get the keys ceremonies for this election event
    let keys_ceremony: KeysCeremony = keys_ceremony::get_keys_ceremony_by_id(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;

    let current_execution_status = keys_ceremony.execution_status()?;
    // check keys_ceremony has correct execution status
    if current_execution_status != KeysCeremonyExecutionStatus::IN_PROGRESS
        && current_execution_status != KeysCeremonyExecutionStatus::SUCCESS
    {
        return Err(anyhow!(
            "Keys ceremony not in ExecutionStatus::IN_PROCESS or  ExecutionStatus::SUCCESS"
        ));
    }

    // get ceremony status
    let current_status = keys_ceremony
        .status()
        .with_context(|| "error parsing keys ceremony current status")?;

    // check the trustee is part of this ceremony
    if !current_status.trustees.iter().any(|trustee| {
        trustee.name == trustee_name
            && matches!(
                trustee.status,
                TrusteeStatus::KEY_GENERATED
                    | TrusteeStatus::KEY_RETRIEVED
                    | TrusteeStatus::KEY_CHECKED
            )
    }) {
        return Err(anyhow!(
            "Trustee not part of the keys ceremony or has invalid state"
        ));
    }

    Err(anyhow!(MANUAL_CEREMONY_ERROR))
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
    // verify trustee names and fetch their objects to get their ids
    let trustees = trustee::get_trustees_by_name(&transaction, &tenant_id, &trustee_names)
        .await
        .with_context(|| "can't fetch trustees by names")?;

    if trustee_names.len() != trustees.len() {
        return Err(anyhow!("can't find all the trustees by their names"));
    }

    // Collect the trustees for the committee. The order the administrator gave
    // becomes the Configuration order (i.e., protocol's trustee indices).
    let (trustee_ids, raw_trustees): (Vec<String>, Vec<RawTrusteeRecord>) = trustee_names
        .iter()
        .map(|name| {
            trustees
                .iter()
                .find(|trustee| trustee.name.as_deref() == Some(name.as_str()))
                .map(|trustee| {
                    (
                        trustee.id.clone(),
                        RawTrusteeRecord {
                            name: name.to_string(),
                            signing_public_key: trustee.public_key.clone(),
                            share_encryption_public_key: trustee
                                .share_encryption_public_key
                                .clone(),
                        },
                    )
                })
                .ok_or_else(|| anyhow!("can't find trustee {name}"))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    // get the election event
    let election_event =
        get_election_event_by_id(&transaction, &tenant_id, &election_event_id).await?;

    let keys_ceremonies =
        keys_ceremony::get_keys_ceremonies(&transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "error listing existing keys ceremonies")?;

    let default_ceremony = keys_ceremonies
        .clone()
        .into_iter()
        .find(|keys_ceremony| keys_ceremony.is_default());

    if default_ceremony.is_some() {
        return Err(anyhow!(
            "there's already an existing running ceremony for all elections"
        ));
    }

    // find if there's any previous ceremony and if so, stop. shouldn't happen,
    // we only allow one per election
    if let Some(election_id) = election_id.clone() {
        let election =
            get_election_by_id(transaction, &tenant_id, &election_event_id, &election_id)
                .await?
                .ok_or(anyhow!("Can't find election"))?;
        if election.keys_ceremony_id.is_some() {
            return Err(anyhow!(
                "there's already an existing running ceremony for election id '{}'",
                election_id
            ));
        }
    } else {
        // it's an event ceremony, then there can be no other keys ceremony
        if keys_ceremonies.len() > 0 {
            return Err(anyhow!("Can't create an election event keys ceremony when there are already existing keys ceremonies."));
        }
    };

    // Sanity check and prepare everything early as possible, including protocol rules.
    let keys_ceremony_uuid = Uuid::new_v4();
    let manager = generate_manager();
    let committee = Committee::new(&raw_trustees).context("failed to build the committee")?;
    let dkg = DkgBoard::new(&keys_ceremony_uuid, &manager, &committee, threshold)
        .context("failed to build the dkgboard")?;
    let keys_ceremony_id: String = keys_ceremony_uuid.to_string();
    let execution_status: String = KeysCeremonyExecutionStatus::default().to_string();
    let status: Value = serde_json::to_value(KeysCeremonyStatus {
        stop_date: None,
        public_key: None,
        public_key_hash: None,
        failure: None,
        logs: generate_keys_initial_log(&trustee_names),
        trustees: initial_trustees(&committee),
    })?;
    let is_default = election_id.is_none();

    let elections = set_election_keys_ceremony(
        &transaction,
        &tenant_id,
        &election_event_id,
        election_id.clone(),
        &keys_ceremony_id,
    )
    .await?;

    // Get permission labels, removing duplicates
    let permission_labels: Vec<String> = elections
        .into_iter()
        .filter_map(|election| election.permission_label)
        .collect::<HashSet<_>>() // Remove duplicates
        .into_iter()
        .collect(); // Convert back to Vec

    let ceremony_policy = if is_automatic_ceremony {
        CeremoniesPolicy::AUTOMATED_CEREMONIES
    } else {
        CeremoniesPolicy::MANUAL_CEREMONIES
    };

    let settings = serde_json::json!({
        "policy": ceremony_policy.to_string(),
    });

    // insert keys-ceremony into the database using postgres
    keys_ceremony::insert_keys_ceremony(
        &transaction,
        keys_ceremony_id.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        trustee_ids,
        /* threshold */ threshold.try_into()?,
        /* status */ Some(status),
        /* execution_status */ Some(execution_status),
        name,
        Some(settings),
        is_default,
        permission_labels,
    )
    .await
    .with_context(|| "couldn't insert keys ceremony")?;

    // The manager key and the board are recorded before anything is published;
    //  the create_keys task posts the stored `Configuration` to the board.
    save_manager_key(
        transaction,
        &tenant_id,
        &election_event_id,
        &dkg.name,
        &manager,
    )
    .await
    .with_context(|| "couldn't store the protocol manager key")?;
    insert_protocol_board(
        transaction,
        &NewProtocolBoard {
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            parent_id: None,
            keys_ceremony_id: keys_ceremony_id.clone(),
            name: dkg.name.as_str().to_string(),
            manager_message: dkg.configuration.to_bytes(),
        },
    )
    .await
    .with_context(|| "couldn't record the ceremony's DKG board")?;

    // Save it in the electoral log
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    // let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    let election_ids = election_id.clone().map(|id| vec![id]);
    let electoral_log = ElectoralLog::for_admin_user(
        &transaction,
        &board_name,
        &tenant_id,
        &election_event.id,
        user_id,
        Some(username.to_string()),
        election_ids.clone(),
        None,
    )
    .await?;
    electoral_log
        .post_keygen(
            election_event_id.clone(),
            Some(user_id.to_string()),
            Some(username.to_string()),
            election_id.clone(),
        )
        .await
        .with_context(|| "error posting to the electoral log")?;

    Ok(keys_ceremony_id)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn validate_permission_labels(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    user_permission_labels: Option<String>,
) -> Result<bool> {
    let elections_permission_label = get_election_permission_label(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Error getting election permissionlabel {:?}", e))?;

    let user_permission_labels = match user_permission_labels {
        Some(perms) => perms,
        None => return Err(anyhow!("user dont have permission labels")),
    };

    let user_permission_labels_json = user_permission_labels
        .trim()
        .strip_prefix('{')
        .unwrap_or(&user_permission_labels)
        .strip_suffix('}')
        .unwrap_or(&user_permission_labels)
        .to_string();
    let user_permission_labels_json = format!("[{}]", user_permission_labels_json);

    let user_permission_labels_vec: HashSet<String> =
        deserialize_str(&user_permission_labels_json)?;

    info!(elections_permission_label = ?elections_permission_label);
    info!(user_permission_labels_vec = ?user_permission_labels_vec);

    let is_valid_permission_labels = elections_permission_label
        .iter()
        .all(|c| user_permission_labels_vec.contains(c));

    Ok(is_valid_permission_labels)
}
