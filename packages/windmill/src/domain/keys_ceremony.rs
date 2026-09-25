// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keys ceremony rules: who may create a ceremony, when trustees may download
//! and check their private keys, and how a ceremony advances while the
//! bulletin board generates the election keys.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus, Log, Trustee, TrusteeStatus,
};
use sequent_core::types::hasura::core::{Election, KeysCeremony, Trustee as TrusteeRecord};
use serde_json::Value;
use std::collections::HashSet;

/// Harvest answers this error with 409 Conflict.
#[derive(Debug, thiserror::Error)]
#[error("Private key download is no longer available")]
pub struct PrivateKeyDownloadUnavailable;

/// The fewest trustees a ceremony can require to decrypt.
pub const MIN_THRESHOLD: usize = 2;

/// A keys ceremony to store.
#[derive(Clone, Debug, PartialEq)]
pub struct NewKeysCeremony {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub trustee_ids: Vec<String>,
    pub threshold: i32,
    pub status: Value,
    pub execution_status: String,
    pub name: Option<String>,
    pub settings: Value,
    pub is_default: bool,
    pub permission_labels: Vec<String>,
}

/// The electoral log entry for a new keys ceremony.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeygenAuditEntry {
    pub board_name: String,
    pub tenant_id: String,
    /// The election event id as stored, which selects the signing keys.
    pub stored_election_event_id: String,
    /// The election event id of the request, which the entry records.
    pub election_event_id: String,
    pub user_id: String,
    pub username: String,
    pub election_id: Option<String>,
}

/// The task that moves a ceremony forward on the bulletin board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysBoardStep {
    /// Post the ceremony configuration so the trustees generate their keys.
    CreateKeys,
    /// Read the generated keys back into the ceremony status.
    SetPublicKey,
}

/// The kind of a key generation message on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysBoardStatement {
    PublicKey,
    PublicKeySigned,
    Other,
}

/// A key generation message, identified by the key of its sender.
#[derive(Clone, Debug)]
pub struct KeysBoardMessage<S> {
    pub statement: KeysBoardStatement,
    pub sender: S,
}

/// The key generation messages of a ceremony board.
#[derive(Clone, Debug)]
pub struct KeysBoardMessages<S> {
    /// Log entries for the messages of batch 0 posted since the time asked
    /// for, sorted by date.
    pub logs: Vec<Log>,
    pub messages: Vec<KeysBoardMessage<S>>,
}

/// Every requested trustee must exist, and the threshold must be between
/// `MIN_THRESHOLD` and the number of trustees.
pub fn validate_new_ceremony_trustees(
    requested_names: usize,
    found_trustees: usize,
    threshold: usize,
) -> Result<()> {
    if requested_names != found_trustees {
        return Err(anyhow!("can't find trustees"));
    }
    if !(MIN_THRESHOLD..=found_trustees).contains(&threshold) {
        return Err(anyhow!("invalid threshold, minimum is 2"));
    }
    Ok(())
}

/// No ceremony can start while one covers all the elections. A ceremony that
/// doesn't record whether it is the default one counts as the default.
pub fn validate_no_default_ceremony(keys_ceremonies: &[KeysCeremony]) -> Result<()> {
    if keys_ceremonies
        .iter()
        .any(|keys_ceremony| keys_ceremony.is_default())
    {
        return Err(anyhow!(
            "there's already an existing running ceremony for all elections"
        ));
    }
    Ok(())
}

/// An election has at most one keys ceremony.
pub fn validate_election_without_ceremony(election_id: &str, election: &Election) -> Result<()> {
    if election.keys_ceremony_id.is_some() {
        return Err(anyhow!(
            "there's already an existing running ceremony for election id '{}'",
            election_id
        ));
    }
    Ok(())
}

/// A ceremony for the whole election event must be its only ceremony.
pub fn validate_event_without_ceremonies(keys_ceremonies: &[KeysCeremony]) -> Result<()> {
    if !keys_ceremonies.is_empty() {
        return Err(anyhow!("Can't create an election event keys ceremony when there are already existing keys ceremonies."));
    }
    Ok(())
}

/// A new ceremony has no public key yet, and every trustee waits for the
/// board to generate its key.
pub fn initial_status(trustees: &[TrusteeRecord], logs: Vec<Log>) -> Result<KeysCeremonyStatus> {
    Ok(KeysCeremonyStatus {
        stop_date: None,
        public_key: None,
        logs,
        trustees: trustees
            .iter()
            .map(|trustee| {
                Ok(Trustee {
                    name: trustee.name.clone().ok_or(anyhow!("empty trustee name"))?,
                    status: TrusteeStatus::WAITING,
                })
            })
            .collect::<Result<Vec<Trustee>>>()?,
    })
}

pub fn ceremony_policy(is_automatic_ceremony: bool) -> CeremoniesPolicy {
    if is_automatic_ceremony {
        CeremoniesPolicy::AUTOMATED_CEREMONIES
    } else {
        CeremoniesPolicy::MANUAL_CEREMONIES
    }
}

pub fn ceremony_settings(policy: &CeremoniesPolicy) -> Value {
    serde_json::json!({
        "policy": policy.to_string(),
    })
}

/// The permission labels of the elections a ceremony covers, each once.
pub fn unique_permission_labels(elections: Vec<Election>) -> Vec<String> {
    elections
        .into_iter()
        .filter_map(|election| election.permission_label)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Reads the permission labels claim of a user, a Postgres array literal such
/// as `{"label-a","label-b"}`.
pub fn parse_user_permission_labels(
    user_permission_labels: Option<String>,
) -> Result<HashSet<String>> {
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

    Ok(deserialize_str(&user_permission_labels_json)?)
}

/// A user may act on elections only when it holds all their labels.
pub fn covers_permission_labels(
    elections_permission_labels: &[String],
    user_permission_labels: &HashSet<String>,
) -> bool {
    elections_permission_labels
        .iter()
        .all(|label| user_permission_labels.contains(label))
}

pub fn validate_private_key_download(
    keys_ceremony: &KeysCeremony,
    trustee_name: &str,
) -> Result<KeysCeremonyStatus> {
    // check keys_ceremony has correct execution status
    if keys_ceremony.execution_status()? != KeysCeremonyExecutionStatus::IN_PROGRESS {
        return Err(PrivateKeyDownloadUnavailable.into());
    }

    // get ceremony status
    let current_status: KeysCeremonyStatus = keys_ceremony
        .status()
        .with_context(|| "error parsing keys ceremony current status")?;

    // check the trustee is part of this ceremony
    let trustee = current_status
        .trustees
        .iter()
        .find(|trustee| trustee.name == trustee_name)
        .ok_or_else(|| anyhow!("Trustee not part of the keys ceremony"))?;

    // downloading again would move a trustee who already checked the key
    // back to KEY_RETRIEVED
    if trustee.status == TrusteeStatus::KEY_CHECKED {
        return Err(PrivateKeyDownloadUnavailable.into());
    }

    Ok(current_status)
}

/// A trustee can check its private key once the board generated it, also
/// after the ceremony succeeded.
pub fn validate_private_key_check(
    keys_ceremony: &KeysCeremony,
    trustee_name: &str,
) -> Result<KeysCeremonyStatus> {
    let execution_status = keys_ceremony.execution_status()?;
    if execution_status != KeysCeremonyExecutionStatus::IN_PROGRESS
        && execution_status != KeysCeremonyExecutionStatus::SUCCESS
    {
        return Err(anyhow!(
            "Keys ceremony not in ExecutionStatus::IN_PROCESS or  ExecutionStatus::SUCCESS"
        ));
    }

    let current_status = keys_ceremony
        .status()
        .with_context(|| "error parsing keys ceremony current status")?;

    let can_check = current_status.trustees.iter().any(|trustee| {
        trustee.name == trustee_name
            && matches!(
                trustee.status,
                TrusteeStatus::KEY_GENERATED
                    | TrusteeStatus::KEY_RETRIEVED
                    | TrusteeStatus::KEY_CHECKED
            )
    });
    if !can_check {
        return Err(anyhow!(
            "Trustee not part of the keys ceremony or has invalid state"
        ));
    }

    Ok(current_status)
}

/// The status after the trustee downloads its private key.
pub fn with_key_retrieved(
    status: &KeysCeremonyStatus,
    trustee_name: &str,
    logs: Vec<Log>,
) -> KeysCeremonyStatus {
    with_trustee_status(status, trustee_name, &TrusteeStatus::KEY_RETRIEVED, logs)
}

/// The status after the trustee checks its private key. The ceremony
/// succeeds once every trustee has checked theirs.
pub fn with_key_checked(
    status: &KeysCeremonyStatus,
    trustee_name: &str,
    logs: Vec<Log>,
) -> (KeysCeremonyStatus, KeysCeremonyExecutionStatus) {
    let new_status = with_trustee_status(status, trustee_name, &TrusteeStatus::KEY_CHECKED, logs);
    let execution_status = if new_status
        .trustees
        .iter()
        .all(|trustee| trustee.status == TrusteeStatus::KEY_CHECKED)
    {
        KeysCeremonyExecutionStatus::SUCCESS
    } else {
        KeysCeremonyExecutionStatus::IN_PROGRESS
    };
    (new_status, execution_status)
}

fn with_trustee_status(
    status: &KeysCeremonyStatus,
    trustee_name: &str,
    trustee_status: &TrusteeStatus,
    logs: Vec<Log>,
) -> KeysCeremonyStatus {
    KeysCeremonyStatus {
        stop_date: None,
        public_key: status.public_key.clone(),
        logs,
        trustees: status
            .trustees
            .iter()
            .map(|trustee| {
                if trustee.name == trustee_name {
                    Trustee {
                        name: trustee.name.clone(),
                        status: trustee_status.clone(),
                    }
                } else {
                    trustee.clone()
                }
            })
            .collect(),
    }
}

/// A started ceremony needs its configuration on the board; one in progress
/// needs the public key once the trustees generate it.
pub fn next_board_step(
    execution_status: &KeysCeremonyExecutionStatus,
    status: &KeysCeremonyStatus,
) -> Option<KeysBoardStep> {
    if *execution_status == KeysCeremonyExecutionStatus::STARTED {
        Some(KeysBoardStep::CreateKeys)
    } else if *execution_status == KeysCeremonyExecutionStatus::IN_PROGRESS
        && status.public_key.is_none()
    {
        Some(KeysBoardStep::SetPublicKey)
    } else {
        None
    }
}

/// Keys are created only for a started ceremony without a public key.
pub fn awaits_key_generation(
    execution_status: &KeysCeremonyExecutionStatus,
    status: &KeysCeremonyStatus,
) -> bool {
    *execution_status == KeysCeremonyExecutionStatus::STARTED && status.public_key.is_none()
}

/// The trustees of a ceremony must be the trustees stored with those names.
pub fn validate_known_trustees(
    trustee_names: &HashSet<String>,
    trustees: &[TrusteeRecord],
) -> Result<()> {
    let found_names = trustees
        .iter()
        .filter_map(|trustee| trustee.name.clone())
        .collect::<HashSet<String>>();
    if *trustee_names != found_names {
        return Err(anyhow!(
            "trustee_names don't correspond to trustees_by_name"
        ));
    }
    Ok(())
}

/// A trustee has generated its key once the board has its public key share.
pub fn trustee_key_status<S: PartialEq>(
    sender: &S,
    messages: &[KeysBoardMessage<S>],
) -> TrusteeStatus {
    let has_public_key = messages.iter().any(|message| {
        matches!(
            message.statement,
            KeysBoardStatement::PublicKey | KeysBoardStatement::PublicKeySigned
        ) && message.sender == *sender
    });
    if has_public_key {
        TrusteeStatus::KEY_GENERATED
    } else {
        TrusteeStatus::WAITING
    }
}

/// An automated ceremony succeeds as soon as the public key exists; a manual
/// one waits for the trustees to check their keys.
pub fn public_key_execution_status(
    policy: &CeremoniesPolicy,
    public_key: Option<&str>,
) -> KeysCeremonyExecutionStatus {
    match (policy, public_key) {
        (CeremoniesPolicy::AUTOMATED_CEREMONIES, Some(_)) => KeysCeremonyExecutionStatus::SUCCESS,
        _ => KeysCeremonyExecutionStatus::IN_PROGRESS,
    }
}

/// Milliseconds since the Unix epoch, at whole-second precision.
pub fn stop_date(now: DateTime<Local>) -> String {
    (now.timestamp() * 1000).to_string()
}
