// SPDX-FileCopyrightText: 2023-2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{
    get_election_by_id, get_election_permission_label, get_elections_by_keys_ceremony_id,
    set_election_keys_ceremony,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony;
use crate::postgres::trustee;
use crate::services::celery_app::get_celery_app;
use crate::services::ceremonies::serialize_logs::*;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::ElectoralLog;
use crate::services::private_keys::get_trustee_encrypted_private_key;
use crate::services::protocol_manager::get_election_board;
use crate::tasks::create_keys::{create_keys, CreateKeysBody};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus, Trustee, TrusteeStatus,
};
use sequent_core::types::hasura::core::KeysCeremony;
use serde_json::Value;
use std::collections::HashSet;
use tracing::instrument;
use tracing::{event, info, Level};
use uuid::Uuid;

// returns (board_name, election_id), where the election_id might be None for an event Board
#[instrument(skip(transaction), err)]
pub async fn get_keys_ceremony_board(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<(String, Option<String>)> {
    if keys_ceremony.is_default() {
        // fetch election_event
        let election_event =
            get_election_event_by_id(transaction, tenant_id, election_event_id).await?;

        // get board name
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;
        Ok((board_name, None))
    } else {
        let election = get_elections_by_keys_ceremony_id(
            transaction,
            tenant_id,
            election_event_id,
            &keys_ceremony.id,
        )
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Can't find election with keys ceremony {}",
                keys_ceremony.id
            )
        })?;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board = get_election_board(tenant_id, &election.id, &slug);
        Ok((board, Some(election.id)))
    }
}

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
    if let None = current_status
        .trustees
        .clone()
        .into_iter()
        .find(|trustee| trustee.name == trustee_name)
    {
        return Err(anyhow!("Trustee not part of the keys ceremony"));
    }

    let (board_name, _) =
        get_keys_ceremony_board(transaction, &tenant_id, &election_event_id, &keys_ceremony)
            .await?;

    let trustee_public_key = trustee::get_trustee_by_name(transaction, &tenant_id, &trustee_name)
        .await
        .with_context(|| "can't find trustee in the database")?
        .public_key
        .clone()
        .ok_or(anyhow!("can't get trustee's public key"))?;

    // get the encrypted private key
    let encrypted_private_key =
        get_trustee_encrypted_private_key(board_name.as_str(), trustee_public_key.as_str()).await?;

    // Update ceremony with the information that this trustee did get the
    // private key
    let status: Value = serde_json::to_value(KeysCeremonyStatus {
        stop_date: None,
        public_key: current_status.public_key.clone(),
        logs: append_keys_trustee_download_log(&current_status.logs, &trustee_name),
        trustees: current_status
            .trustees
            .clone()
            .into_iter()
            .map(|trustee| {
                if (trustee.name == trustee_name) {
                    Ok(Trustee {
                        name: trustee.name,
                        status: TrusteeStatus::KEY_RETRIEVED,
                    })
                } else {
                    Ok(trustee.clone())
                }
            })
            .collect::<Result<Vec<Trustee>>>()?,
    })?;

    // update keys-ceremony into the database using graphql
    keys_ceremony::update_keys_ceremony_status(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
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
        election_event_id.clone(),
        keys_ceremony_id.clone(),
        trustee_name.clone()
    );
    Ok(encrypted_private_key)
}

#[instrument(skip(transaction), err)]
pub async fn find_trustee_private_key(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    trustee_name: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<String> {
    let (board_name, _) =
        get_keys_ceremony_board(transaction, &tenant_id, &election_event_id, &keys_ceremony)
            .await?;

    let trustee_public_key = trustee::get_trustee_by_name(transaction, tenant_id, trustee_name)
        .await?
        .public_key
        .clone()
        .ok_or(anyhow!("can't get trustee public key"))?;

    // get the encrypted private key
    get_trustee_encrypted_private_key(board_name.as_str(), trustee_public_key.as_str()).await
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
    if let None = current_status.trustees.clone().into_iter().find(|trustee| {
        (trustee.name == trustee_name
            && (trustee.status == TrusteeStatus::KEY_GENERATED
                || trustee.status == TrusteeStatus::KEY_RETRIEVED
                || trustee.status == TrusteeStatus::KEY_CHECKED))
    }) {
        return Err(anyhow!(
            "Trustee not part of the keys ceremony or has invalid state"
        ));
    }

    // get the encrypted private key
    let encrypted_private_key = find_trustee_private_key(
        transaction,
        &tenant_id,
        &election_event_id,
        &trustee_name,
        &keys_ceremony,
    )
    .await?;

    if encrypted_private_key != private_key_base64 {
        return Ok(false);
    }

    // Update ceremony with the information that this trustee did get the
    // private key
    let new_status = KeysCeremonyStatus {
        stop_date: None,
        public_key: current_status.public_key.clone(),
        logs: append_keys_trustee_check_log(&current_status.logs, &trustee_name),
        trustees: current_status
            .trustees
            .iter()
            .map(|trustee| {
                if (trustee.name == trustee_name) {
                    Ok(Trustee {
                        name: trustee.name.clone(),
                        status: TrusteeStatus::KEY_CHECKED,
                    })
                } else {
                    Ok(trustee.clone())
                }
            })
            .collect::<Result<Vec<Trustee>>>()?,
    };

    let all_trustees_checked = new_status
        .trustees
        .iter()
        .all(|trustee| trustee.status == TrusteeStatus::KEY_CHECKED);
    let new_execution_status = if all_trustees_checked {
        KeysCeremonyExecutionStatus::SUCCESS
    } else {
        KeysCeremonyExecutionStatus::IN_PROGRESS
    };

    // update keys-ceremony into the database using graphql
    keys_ceremony::update_keys_ceremony_status(
        transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
        /* status */ &serde_json::to_value(new_status)?,
        /* execution_status */ &new_execution_status.to_string(),
    )
    .await
    .with_context(|| "couldn't update keys ceremony")?;

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        election_event_id.clone(),
        keys_ceremony_id.clone(),
        trustee_name.clone()
    );
    Ok(true)
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
        .with_context(|| "can't find trustees")?;

    if trustee_names.len() != trustees.len() {
        return Err(anyhow!("can't find trustees"));
    }
    if threshold < 2 || threshold > trustees.len() {
        return Err(anyhow!("invalid threshold, minimum is 2"));
    }

    // obtain trustee ids list
    let trustee_ids = trustees
        .clone()
        .into_iter()
        .map(|trustee| trustee.id)
        .collect();

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

    // generate default values
    let keys_ceremony_id: String = Uuid::new_v4().to_string();
    let execution_status: String = KeysCeremonyExecutionStatus::default().to_string();
    let status: Value = serde_json::to_value(KeysCeremonyStatus {
        stop_date: None,
        public_key: None,
        logs: generate_keys_initial_log(&trustee_names),
        trustees: trustees
            .clone()
            .into_iter()
            .map(|trustee| {
                Ok(Trustee {
                    name: trustee.name.ok_or(anyhow!("empty trustee name"))?,
                    status: TrusteeStatus::WAITING,
                })
            })
            .collect::<Result<Vec<Trustee>>>()?,
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
