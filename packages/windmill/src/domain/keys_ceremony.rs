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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    const TRUSTEE: &str = "trustee1";
    const OTHER_TRUSTEE: &str = "trustee2";
    const PUBLIC_KEY: &str = "public-key";

    fn status(trustees: &[(&str, TrusteeStatus)]) -> KeysCeremonyStatus {
        KeysCeremonyStatus {
            stop_date: Some("1699990000000".to_string()),
            public_key: Some(PUBLIC_KEY.to_string()),
            logs: vec![],
            trustees: trustees
                .iter()
                .map(|(name, status)| Trustee {
                    name: name.to_string(),
                    status: status.clone(),
                })
                .collect(),
        }
    }

    fn keys_ceremony(
        execution_status: KeysCeremonyExecutionStatus,
        trustee_status: TrusteeStatus,
    ) -> KeysCeremony {
        let status = status(&[
            (TRUSTEE, trustee_status),
            (OTHER_TRUSTEE, TrusteeStatus::KEY_GENERATED),
        ]);
        serde_json::from_value(json!({
            "id": "keys-ceremony",
            "tenant_id": "tenant",
            "election_event_id": "election-event",
            "trustee_ids": [],
            "status": status,
            "execution_status": execution_status.to_string(),
            "threshold": 2,
        }))
        .expect("keys ceremony")
    }

    fn with_default(is_default: Option<bool>) -> KeysCeremony {
        KeysCeremony {
            is_default,
            ..keys_ceremony(
                KeysCeremonyExecutionStatus::IN_PROGRESS,
                TrusteeStatus::KEY_GENERATED,
            )
        }
    }

    fn trustee_record(name: Option<&str>) -> TrusteeRecord {
        serde_json::from_value(json!({"id": "id", "name": name, "tenant_id": "tenant"}))
            .expect("trustee")
    }

    fn election(keys_ceremony_id: Option<&str>) -> Election {
        serde_json::from_value(json!({
            "id": "election",
            "tenant_id": "tenant",
            "election_event_id": "election-event",
            "keys_ceremony_id": keys_ceremony_id,
        }))
        .expect("election")
    }

    fn statuses(status: &KeysCeremonyStatus) -> Vec<(&str, TrusteeStatus)> {
        status
            .trustees
            .iter()
            .map(|trustee| (trustee.name.as_str(), trustee.status.clone()))
            .collect()
    }

    fn log(text: &str) -> Log {
        Log {
            created_date: "2023-11-14T22:13:20+00:00".to_string(),
            log_text: text.to_string(),
        }
    }

    fn message(statement: KeysBoardStatement, sender: &str) -> KeysBoardMessage<String> {
        KeysBoardMessage {
            statement,
            sender: sender.to_string(),
        }
    }

    fn error_message(result: Result<impl std::fmt::Debug>) -> String {
        format!("{:#}", result.expect_err("an error"))
    }

    #[test]
    fn a_new_ceremony_needs_every_requested_trustee() {
        assert_eq!(
            error_message(validate_new_ceremony_trustees(3, 2, 2)),
            "can't find trustees"
        );
        // Checked first, so it wins over an invalid threshold.
        assert_eq!(
            error_message(validate_new_ceremony_trustees(3, 2, 5)),
            "can't find trustees"
        );
    }

    #[test]
    fn thresholds_from_two_to_the_number_of_trustees_are_valid() {
        for threshold in [2, 3] {
            assert!(validate_new_ceremony_trustees(3, 3, threshold).is_ok());
        }
    }

    #[test]
    fn thresholds_below_two_or_above_the_number_of_trustees_are_invalid() {
        for (trustees, threshold) in [(3, 0), (3, 1), (3, 4), (1, 1), (1, 2)] {
            assert_eq!(
                error_message(validate_new_ceremony_trustees(
                    trustees, trustees, threshold
                )),
                "invalid threshold, minimum is 2"
            );
        }
    }

    #[test]
    fn a_ceremony_for_all_elections_blocks_new_ceremonies() {
        for is_default in [Some(true), None] {
            assert_eq!(
                error_message(validate_no_default_ceremony(&[
                    with_default(Some(false)),
                    with_default(is_default),
                ])),
                "there's already an existing running ceremony for all elections"
            );
        }
    }

    #[test]
    fn ceremonies_for_single_elections_do_not_block_new_ceremonies() {
        assert!(validate_no_default_ceremony(&[]).is_ok());
        assert!(validate_no_default_ceremony(&[with_default(Some(false))]).is_ok());
    }

    #[test]
    fn an_election_can_have_only_one_ceremony() {
        assert!(validate_election_without_ceremony("election", &election(None)).is_ok());
        assert_eq!(
            error_message(validate_election_without_ceremony(
                "requested-election",
                &election(Some("keys-ceremony"))
            )),
            "there's already an existing running ceremony for election id 'requested-election'"
        );
    }

    #[test]
    fn a_ceremony_for_the_whole_event_must_be_its_only_ceremony() {
        assert!(validate_event_without_ceremonies(&[]).is_ok());
        assert_eq!(
            error_message(validate_event_without_ceremonies(&[with_default(Some(false))])),
            "Can't create an election event keys ceremony when there are already existing keys ceremonies."
        );
    }

    #[test]
    fn a_new_ceremony_waits_for_every_trustee_without_a_public_key() {
        let status = initial_status(
            &[
                trustee_record(Some(OTHER_TRUSTEE)),
                trustee_record(Some(TRUSTEE)),
            ],
            vec![log("created")],
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(status).unwrap(),
            json!({
                "stop_date": null,
                "public_key": null,
                "logs": [{"created_date": "2023-11-14T22:13:20+00:00", "log_text": "created"}],
                "trustees": [
                    {"name": "trustee2", "status": "WAITING"},
                    {"name": "trustee1", "status": "WAITING"},
                ],
            })
        );
    }

    #[test]
    fn a_trustee_without_a_name_cannot_join_a_ceremony() {
        assert_eq!(
            error_message(initial_status(
                &[trustee_record(Some(TRUSTEE)), trustee_record(None)],
                vec![]
            )),
            "empty trustee name"
        );
    }

    #[test]
    fn automatic_ceremonies_are_stored_with_the_automated_policy() {
        assert_eq!(
            ceremony_settings(&ceremony_policy(true)),
            json!({"policy": "automated-ceremonies"})
        );
        assert_eq!(
            ceremony_settings(&ceremony_policy(false)),
            json!({"policy": "manual-ceremonies"})
        );
    }

    #[test]
    fn user_permission_labels_are_read_from_a_postgres_array() {
        let cases = [
            (r#"{"label-a","label-b"}"#, vec!["label-a", "label-b"]),
            (r#"  {"label-a"}  "#, vec!["label-a"]),
            (r#""label-a""#, vec!["label-a"]),
            ("{}", vec![]),
        ];
        for (claim, expected) in cases {
            let labels = parse_user_permission_labels(Some(claim.to_string())).unwrap();

            assert_eq!(
                labels,
                expected
                    .into_iter()
                    .map(str::to_string)
                    .collect::<HashSet<_>>(),
                "{claim}"
            );
        }
    }

    #[test]
    fn missing_or_malformed_user_permission_labels_are_rejected() {
        assert_eq!(
            error_message(parse_user_permission_labels(None)),
            "user dont have permission labels"
        );
        for claim in ["{label-a}", r#"{"label-a""#] {
            assert!(parse_user_permission_labels(Some(claim.to_string())).is_err());
        }
    }

    #[test]
    fn a_user_needs_every_permission_label_of_the_elections() {
        let user_labels: HashSet<String> = ["label-a".to_string()].into();

        assert!(covers_permission_labels(&[], &user_labels));
        assert!(covers_permission_labels(
            &["label-a".to_string(), "label-a".to_string()],
            &user_labels
        ));
        assert!(!covers_permission_labels(
            &["label-a".to_string(), "label-b".to_string()],
            &user_labels
        ));
    }

    #[test]
    fn each_permission_label_of_the_elections_is_kept_once() {
        let labelled = |label: Option<&str>| Election {
            permission_label: label.map(str::to_string),
            ..election(None)
        };

        let mut labels = unique_permission_labels(vec![
            labelled(Some("label-b")),
            labelled(None),
            labelled(Some("label-a")),
            labelled(Some("label-b")),
        ]);
        labels.sort();

        assert_eq!(labels, vec!["label-a", "label-b"]);
    }

    #[test]
    fn a_trustee_waiting_for_its_key_may_still_download_it() {
        let ceremony = keys_ceremony(
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            TrusteeStatus::WAITING,
        );

        assert!(validate_private_key_download(&ceremony, TRUSTEE).is_ok());
    }

    #[test]
    fn a_trustee_can_check_a_generated_key_while_in_progress_or_after_success() {
        for execution_status in [
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            KeysCeremonyExecutionStatus::SUCCESS,
        ] {
            for trustee_status in [
                TrusteeStatus::KEY_GENERATED,
                TrusteeStatus::KEY_RETRIEVED,
                TrusteeStatus::KEY_CHECKED,
            ] {
                let ceremony = keys_ceremony(execution_status.clone(), trustee_status);

                assert!(validate_private_key_check(&ceremony, TRUSTEE).is_ok());
            }
        }
    }

    #[test]
    fn checks_are_refused_in_any_other_ceremony_status() {
        for execution_status in [
            KeysCeremonyExecutionStatus::USER_CONFIGURATION,
            KeysCeremonyExecutionStatus::STARTED,
            KeysCeremonyExecutionStatus::CANCELLED,
        ] {
            let ceremony = keys_ceremony(execution_status, TrusteeStatus::KEY_RETRIEVED);

            assert_eq!(
                error_message(validate_private_key_check(&ceremony, TRUSTEE)),
                "Keys ceremony not in ExecutionStatus::IN_PROCESS or  ExecutionStatus::SUCCESS"
            );
        }
    }

    #[test]
    fn only_a_trustee_of_the_ceremony_with_a_generated_key_can_check_it() {
        let waiting = keys_ceremony(
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            TrusteeStatus::WAITING,
        );
        for (ceremony, trustee_name) in [(&waiting, TRUSTEE), (&waiting, "trustee3")] {
            assert_eq!(
                error_message(validate_private_key_check(ceremony, trustee_name)),
                "Trustee not part of the keys ceremony or has invalid state"
            );
        }
    }

    #[test]
    fn an_unreadable_ceremony_status_fails_the_check() {
        let ceremony = KeysCeremony {
            status: Some(json!({"trustees": "invalid"})),
            ..keys_ceremony(
                KeysCeremonyExecutionStatus::IN_PROGRESS,
                TrusteeStatus::KEY_RETRIEVED,
            )
        };

        assert!(
            error_message(validate_private_key_check(&ceremony, TRUSTEE))
                .starts_with("error parsing keys ceremony current status: ")
        );
    }

    #[test]
    fn a_download_marks_only_that_trustee_retrieved() {
        let current = status(&[
            (TRUSTEE, TrusteeStatus::KEY_GENERATED),
            (OTHER_TRUSTEE, TrusteeStatus::KEY_GENERATED),
        ]);

        let new_status = with_key_retrieved(&current, TRUSTEE, vec![log("downloaded")]);

        assert_eq!(
            statuses(&new_status),
            vec![
                (TRUSTEE, TrusteeStatus::KEY_RETRIEVED),
                (OTHER_TRUSTEE, TrusteeStatus::KEY_GENERATED),
            ]
        );
        assert_eq!(new_status.public_key.as_deref(), Some(PUBLIC_KEY));
        assert_eq!(new_status.stop_date, None);
        assert_eq!(new_status.logs.len(), 1);
        assert_eq!(new_status.logs[0].log_text, "downloaded");
    }

    #[test]
    fn a_check_keeps_the_ceremony_in_progress_while_another_trustee_has_not_checked() {
        let current = status(&[
            (TRUSTEE, TrusteeStatus::KEY_RETRIEVED),
            (OTHER_TRUSTEE, TrusteeStatus::KEY_RETRIEVED),
        ]);

        let (new_status, execution_status) =
            with_key_checked(&current, TRUSTEE, vec![log("checked")]);

        assert_eq!(
            statuses(&new_status),
            vec![
                (TRUSTEE, TrusteeStatus::KEY_CHECKED),
                (OTHER_TRUSTEE, TrusteeStatus::KEY_RETRIEVED),
            ]
        );
        assert_eq!(execution_status, KeysCeremonyExecutionStatus::IN_PROGRESS);
        assert_eq!(new_status.stop_date, None);
        assert_eq!(new_status.public_key.as_deref(), Some(PUBLIC_KEY));
    }

    #[test]
    fn the_ceremony_succeeds_when_the_last_trustee_checks_its_key() {
        let current = status(&[
            (TRUSTEE, TrusteeStatus::KEY_RETRIEVED),
            (OTHER_TRUSTEE, TrusteeStatus::KEY_CHECKED),
        ]);

        let (_, execution_status) = with_key_checked(&current, TRUSTEE, vec![]);

        assert_eq!(execution_status, KeysCeremonyExecutionStatus::SUCCESS);
    }

    #[test]
    fn each_ceremony_state_needs_its_own_board_step() {
        let with_key = status(&[]);
        let without_key = KeysCeremonyStatus {
            public_key: None,
            ..status(&[])
        };
        let cases = [
            (
                KeysCeremonyExecutionStatus::STARTED,
                &without_key,
                Some(KeysBoardStep::CreateKeys),
            ),
            (
                KeysCeremonyExecutionStatus::STARTED,
                &with_key,
                Some(KeysBoardStep::CreateKeys),
            ),
            (
                KeysCeremonyExecutionStatus::IN_PROGRESS,
                &without_key,
                Some(KeysBoardStep::SetPublicKey),
            ),
            (KeysCeremonyExecutionStatus::IN_PROGRESS, &with_key, None),
            (
                KeysCeremonyExecutionStatus::USER_CONFIGURATION,
                &without_key,
                None,
            ),
            (KeysCeremonyExecutionStatus::SUCCESS, &without_key, None),
            (KeysCeremonyExecutionStatus::CANCELLED, &without_key, None),
        ];
        for (execution_status, status, step) in cases {
            assert_eq!(
                next_board_step(&execution_status, status),
                step,
                "{execution_status} with public key {:?}",
                status.public_key
            );
        }
    }

    #[test]
    fn keys_are_generated_only_for_a_started_ceremony_without_a_public_key() {
        let with_key = status(&[]);
        let without_key = KeysCeremonyStatus {
            public_key: None,
            ..status(&[])
        };

        assert!(awaits_key_generation(
            &KeysCeremonyExecutionStatus::STARTED,
            &without_key
        ));
        assert!(!awaits_key_generation(
            &KeysCeremonyExecutionStatus::STARTED,
            &with_key
        ));
        for execution_status in [
            KeysCeremonyExecutionStatus::USER_CONFIGURATION,
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            KeysCeremonyExecutionStatus::SUCCESS,
            KeysCeremonyExecutionStatus::CANCELLED,
        ] {
            assert!(!awaits_key_generation(&execution_status, &without_key));
        }
    }

    #[test]
    fn the_ceremony_trustees_must_be_the_stored_trustees() {
        let names: HashSet<String> = [TRUSTEE.to_string(), OTHER_TRUSTEE.to_string()].into();
        let both = [
            trustee_record(Some(TRUSTEE)),
            trustee_record(Some(OTHER_TRUSTEE)),
        ];

        assert!(validate_known_trustees(&names, &both).is_ok());
        assert!(validate_known_trustees(
            &names,
            &[
                trustee_record(Some(OTHER_TRUSTEE)),
                trustee_record(None),
                trustee_record(Some(TRUSTEE))
            ]
        )
        .is_ok());
        for trustees in [
            &both[..1],
            &[
                trustee_record(Some(TRUSTEE)),
                trustee_record(Some(OTHER_TRUSTEE)),
                trustee_record(Some("trustee3")),
            ][..],
        ] {
            assert_eq!(
                error_message(validate_known_trustees(&names, trustees)),
                "trustee_names don't correspond to trustees_by_name"
            );
        }
    }

    #[test]
    fn a_trustee_generated_its_key_once_the_board_has_its_public_key_share() {
        for statement in [
            KeysBoardStatement::PublicKey,
            KeysBoardStatement::PublicKeySigned,
        ] {
            let messages = [
                message(KeysBoardStatement::Other, "other-sender"),
                message(statement, "sender"),
            ];

            assert_eq!(
                trustee_key_status(&"sender".to_string(), &messages),
                TrusteeStatus::KEY_GENERATED
            );
        }
    }

    #[test]
    fn other_messages_or_senders_do_not_count_as_a_generated_key() {
        let messages = [
            message(KeysBoardStatement::Other, "sender"),
            message(KeysBoardStatement::PublicKey, "other-sender"),
            message(KeysBoardStatement::PublicKeySigned, "other-sender"),
        ];

        assert_eq!(
            trustee_key_status(&"sender".to_string(), &messages),
            TrusteeStatus::WAITING
        );
        assert_eq!(
            trustee_key_status(&"sender".to_string(), &[]),
            TrusteeStatus::WAITING
        );
    }

    #[test]
    fn only_automated_ceremonies_succeed_with_the_public_key() {
        let automated = CeremoniesPolicy::AUTOMATED_CEREMONIES;
        let manual = CeremoniesPolicy::MANUAL_CEREMONIES;

        assert_eq!(
            public_key_execution_status(&automated, Some(PUBLIC_KEY)),
            KeysCeremonyExecutionStatus::SUCCESS
        );
        assert_eq!(
            public_key_execution_status(&automated, None),
            KeysCeremonyExecutionStatus::IN_PROGRESS
        );
        assert_eq!(
            public_key_execution_status(&manual, Some(PUBLIC_KEY)),
            KeysCeremonyExecutionStatus::IN_PROGRESS
        );
    }

    #[test]
    fn the_stop_date_counts_whole_seconds_in_milliseconds() {
        let now = Local
            .timestamp_opt(1_700_000_000, 999_000_000)
            .single()
            .expect("valid time");

        assert_eq!(stop_date(now), "1700000000000");
    }
}
