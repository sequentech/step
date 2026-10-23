// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Transparency tab of an election event (step-integration.md, sections
//! 2.1 and 2.2): its mode, its logs on the sidecar, the draft policy that the
//! Board seals, and the upload of the signed policy.

use super::client::{
    post_enrolment, read_token_file, CreateLog, SidecarClient, SidecarError, Submission,
};
use super::config::DeploymentConfig;
use super::policy::{
    check_mode, draft, policy_name, public_refusal, DraftEvent, DraftLog, PolicyFacts,
};
use super::window::voting_window;
use super::{canonical_id, sidecar_name, LogName};
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::postgres::tenant::get_tenant_by_id;
use crate::postgres::vitrine::{
    ensure_vitrine_event, get_vitrine_event, get_vitrine_logs, insert_vitrine_log,
    lock_election_event, set_vitrine_event_policy, set_vitrine_log_policy,
    update_presentation_transparency, vitrine_voting_opened, VitrineEvent, VitrineLog,
};
use crate::services::protocol_manager::get_event_databases;
use crate::types::vitrine::TransparencyOutput;
use anyhow::{anyhow, Context};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::adapters::postgres::PostgresConnection;
use electoral_log::vitrine::{ELECTION_TAG, EVENTS_TAG};
use sequent_core::ballot::{
    TransparencyMode, VitrineLogPresentation, VitrineLogsPresentation, VitrinePresentation,
};
use sequent_core::types::hasura::core::ElectionEvent;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use strum::IntoEnumIterator;
use tracing::warn;

/// Development only: `<witness admin URL>|<admin token file>` pairs, separated
/// by commas or spaces, of witnesses that Harvest enrols the event's logs in
/// itself, as their operators do in production.
pub const DEV_WITNESS_ADMINS_ENV: &str = "HARVEST_VITRINE_DEV_WITNESS_ADMINS";
const DEV_WITNESS_SEPARATOR: char = '|';
const POLICY_ID_PREFIX: &str = "sha256:";
const PRESENTATION_TRANSPARENCY: &str = "transparency";
const PRESENTATION_VITRINE: &str = "vitrine";
const SALT_BYTES: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum TransparencyError {
    /// The event's state doesn't allow it.
    #[error("{0}")]
    Refused(String),
    #[error(transparent)]
    Sidecar(#[from] SidecarError),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<tokio_postgres::Error> for TransparencyError {
    fn from(e: tokio_postgres::Error) -> Self {
        TransparencyError::Internal(e.into())
    }
}

pub type TransparencyResult<T> = Result<T, TransparencyError>;

fn refused<T>(why: impl Into<String>) -> TransparencyResult<T> {
    Err(TransparencyError::Refused(why.into()))
}

/// The event's mode: its presentation's `transparency`, read on its own so
/// that the rest of the presentation can't make it unreadable.
pub fn transparency_mode(event: &ElectionEvent) -> anyhow::Result<TransparencyMode> {
    match event
        .presentation
        .as_ref()
        .and_then(|p| p.get(PRESENTATION_TRANSPARENCY))
    {
        None | Some(Value::Null) => Ok(TransparencyMode::DISABLED),
        Some(mode) => serde_json::from_value(mode.clone())
            .with_context(|| format!("Unknown transparency mode {mode}")),
    }
}

/// A presentation without the event's transparency: an event that is
/// created or imported starts without logs, so with transparency disabled.
pub fn without_transparency(presentation: Option<Value>) -> Option<Value> {
    presentation.map(|mut presentation| {
        if let Some(object) = presentation.as_object_mut() {
            object.remove(PRESENTATION_TRANSPARENCY);
            object.remove(PRESENTATION_VITRINE);
        }
        presentation
    })
}

/// The policy ID without its `sha256:` prefix, as voters and explorers
/// name it.
pub fn policy_id_hex(policy_id: &str) -> &str {
    policy_id
        .strip_prefix(POLICY_ID_PREFIX)
        .unwrap_or(policy_id)
}

/// What voters read in the event's presentation: the logs and, once
/// uploaded, the policy ID, and the policy itself for public events.
pub fn presentation_for(
    mode: TransparencyMode,
    logs: &[VitrineLog],
    event: Option<&VitrineEvent>,
) -> VitrinePresentation {
    let log = |name: LogName| {
        logs.iter()
            .find(|l| l.name == name)
            .map(|l| VitrineLogPresentation {
                url: l.url.clone(),
                origin: l.origin.clone(),
            })
    };
    let policy_id = event.and_then(|e| e.policy_id.as_deref());
    VitrinePresentation {
        policy_id: policy_id.map(|id| policy_id_hex(id).to_string()),
        policy: event
            .and_then(|e| e.policy.clone())
            .filter(|_| mode == TransparencyMode::PUBLIC),
        logs: Some(VitrineLogsPresentation {
            ballots: log(LogName::Ballots),
            events: log(LogName::Events),
        }),
    }
}

/// The pairs of `HARVEST_VITRINE_DEV_WITNESS_ADMINS`.
pub fn dev_witness_admins(value: &str) -> anyhow::Result<Vec<(String, String)>> {
    value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (url, token_file) = pair
                .split_once(DEV_WITNESS_SEPARATOR)
                .filter(|(url, file)| !url.is_empty() && !file.is_empty())
                .ok_or_else(|| {
                    anyhow!("{DEV_WITNESS_ADMINS_ENV}: {pair:?} isn't <witness admin URL>|<admin token file>")
                })?;
            Ok((url.to_string(), token_file.to_string()))
        })
        .collect()
}

/// The draft policy, once the event has both logs, elections and a voting
/// window.
async fn draft_policy(
    tx: &Transaction<'_>,
    config: &DeploymentConfig,
    event: &ElectionEvent,
    mode: TransparencyMode,
    logs: &[VitrineLog],
    vitrine: Option<&VitrineEvent>,
) -> anyhow::Result<Option<String>> {
    let draft_log = |name: LogName| {
        logs.iter().find(|l| l.name == name).map(|l| DraftLog {
            origin: l.origin.clone(),
            vkey: l.vkey.clone(),
            log_time_vkey: l.log_time_vkey.clone(),
            url: l.url.clone(),
            path: l.sidecar_name.clone(),
        })
    };
    let (Some(ballots), Some(events)) = (draft_log(LogName::Ballots), draft_log(LogName::Events))
    else {
        return Ok(None);
    };
    if !mode.is_enabled() {
        return Ok(None);
    }
    let elections = get_elections(tx, &event.tenant_id, &event.id).await?;
    if elections.is_empty() {
        return Ok(None);
    }
    let scheduled =
        find_scheduled_event_by_election_event_id(tx, &event.tenant_id, &event.id).await?;
    let Some(window) = voting_window(&scheduled, &event.tenant_id, &event.id, &elections)? else {
        return Ok(None);
    };
    let tenant = get_tenant_by_id(tx, &event.tenant_id).await?;
    let language = event
        .get_presentation()?
        .and_then(|p| p.language_conf)
        .and_then(|l| l.default_language_code);
    let draft_event = DraftEvent {
        name: policy_name(Some(&tenant.slug), &event.tenant_id, &event.id)?,
        mode,
        election_event_id: event.id.clone(),
        election_ids: elections.iter().map(|e| e.id.clone()).collect(),
        window,
        language,
        policy_salt: vitrine.and_then(|v| v.policy_salt.clone()),
        ballots,
        events,
    };
    Ok(Some(draft(config, &draft_event)?))
}

/// The sidecar's listing entries and status of the event's logs. The tab
/// shows what it can: a sidecar that doesn't answer leaves both empty.
async fn sidecar_view(
    sidecar: Option<&SidecarClient>,
    logs: &[VitrineLog],
) -> (Option<Value>, Option<Value>) {
    let Some(sidecar) = sidecar.filter(|_| !logs.is_empty()) else {
        return (None, None);
    };
    let names: BTreeSet<&str> = logs.iter().map(|l| l.sidecar_name.as_str()).collect();
    let listing = match sidecar.logs().await {
        Ok(all) => Some(Value::Array(
            all.into_iter()
                .filter(|entry| {
                    entry
                        .get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| names.contains(name))
                })
                .collect(),
        )),
        Err(e) => {
            warn!("Error listing the Vitrine logs: {e}");
            None
        }
    };
    let mut status = serde_json::Map::new();
    for log in logs {
        match sidecar.status(&log.sidecar_name).await {
            Ok(value) => {
                status.insert(log.name.to_string(), value);
            }
            Err(e) => warn!("Error reading the status of {}: {e}", log.sidecar_name),
        }
    }
    (
        listing,
        (!status.is_empty()).then_some(Value::Object(status)),
    )
}

/// The tab's view of the event.
pub async fn transparency_output(
    tx: &Transaction<'_>,
    sidecar: Option<&SidecarClient>,
    config: Option<&DeploymentConfig>,
    tenant_id: &str,
    election_event_id: &str,
) -> TransparencyResult<TransparencyOutput> {
    let event = get_election_event_by_id(tx, tenant_id, election_event_id).await?;
    let mode = transparency_mode(&event)?;
    let elections = get_elections(tx, tenant_id, election_event_id).await?;
    let locked = vitrine_voting_opened(tx, tenant_id, election_event_id).await?;
    let logs = get_vitrine_logs(tx, tenant_id, election_event_id).await?;
    let vitrine = get_vitrine_event(tx, tenant_id, election_event_id).await?;
    let draft_policy = match config {
        Some(config) => draft_policy(tx, config, &event, mode, &logs, vitrine.as_ref()).await?,
        None => None,
    };
    let (logs_view, status) = sidecar_view(sidecar, &logs).await;
    let enrolments: Vec<Value> = logs
        .iter()
        .filter_map(|l| {
            l.enrolment
                .as_ref()
                .map(|note| json!({ "log": l.name.to_string(), "note": note }))
        })
        .collect();
    let policy_id = vitrine.as_ref().and_then(|v| v.policy_id.clone());
    Ok(TransparencyOutput {
        mode,
        refusal: public_refusal(&elections).map(|r| r.to_string()),
        locked,
        logs: logs_view,
        status,
        draft_policy,
        policy: vitrine.as_ref().and_then(|v| v.policy.clone()),
        explorer_url: config
            .zip(policy_id.as_deref())
            .and_then(|(config, id)| config.explorer_url(policy_id_hex(id))),
        policy_id,
        enrolments: (!enrolments.is_empty()).then_some(Value::Array(enrolments)),
    })
}

/// Locks the event and refuses once its voting window started.
async fn lock_unopened(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> TransparencyResult<ElectionEvent> {
    lock_election_event(tx, tenant_id, election_event_id).await?;
    if vitrine_voting_opened(tx, tenant_id, election_event_id).await? {
        return refused(
            "Voting opened in this election event: its transparency mode and policy can't change",
        );
    }
    Ok(get_election_event_by_id(tx, tenant_id, election_event_id).await?)
}

async fn create_log(
    db: &mut DbClient,
    sidecar: &SidecarClient,
    config: &DeploymentConfig,
    tenant_id: &str,
    election_event_id: &str,
    name: LogName,
    mode: TransparencyMode,
) -> TransparencyResult<()> {
    let database = get_event_databases()
        .await?
        .event(election_event_id)
        .await?
        .ok_or_else(|| anyhow!("Election event {election_event_id} has no electoral-log database"))?
        .database_name;
    let sidecar_name = sidecar_name(tenant_id, election_event_id, name)?;
    let origin = config.origin(tenant_id, election_event_id, name)?;
    let visibility = mode.to_string();
    let queue_role = PostgresConnection::vitrine_queue_role();
    let request = match name {
        LogName::Ballots => CreateLog {
            name: &sidecar_name,
            origin: &origin,
            database: &database,
            profile: ELECTION_TAG,
            visibility: &visibility,
            submission: Submission::Queue,
            app_role: Some(
                queue_role
                    .as_deref()
                    .ok_or_else(|| anyhow!("ELECTORAL_LOG_PG_VITRINE_QUEUE_ROLE isn't set"))?,
            ),
        },
        LogName::Events => CreateLog {
            name: &sidecar_name,
            origin: &origin,
            database: &database,
            profile: EVENTS_TAG,
            visibility: &visibility,
            submission: Submission::Http,
            app_role: None,
        },
    };
    let created = sidecar.create_log(&request).await?;
    if name == LogName::Events && created.token.is_none() {
        return Err(anyhow!("The sidecar gave the events log no submission token").into());
    }
    let tx = db.transaction().await?;
    insert_vitrine_log(
        &tx,
        tenant_id,
        election_event_id,
        &VitrineLog {
            name,
            sidecar_name,
            log_id: created.id,
            instance_id: created.instance_id,
            origin,
            url: created.url,
            vkey: created.vkey,
            log_time_vkey: created.log_time_vkey,
            visibility: mode,
            submission_token: created.token.filter(|_| name == LogName::Events),
            policy_ack: None,
            enrolment: None,
            frozen: false,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// `enable_transparency`: sets the event's mode, creating its logs on the
/// sidecar the first time. A `public` event whose elections limit revotes
/// is refused: the output says why and nothing changes.
pub async fn enable_transparency(
    db: &mut DbClient,
    sidecar: Option<&SidecarClient>,
    config: Option<&DeploymentConfig>,
    tenant_id: &str,
    election_event_id: &str,
    mode: TransparencyMode,
) -> TransparencyResult<TransparencyOutput> {
    let tx = db.transaction().await?;
    lock_unopened(&tx, tenant_id, election_event_id).await?;
    let elections = get_elections(&tx, tenant_id, election_event_id).await?;
    let logs = get_vitrine_logs(&tx, tenant_id, election_event_id).await?;
    let blocked = mode == TransparencyMode::PUBLIC && public_refusal(&elections).is_some();
    if blocked || !mode.is_enabled() {
        if !blocked {
            update_presentation_transparency(&tx, tenant_id, election_event_id, mode, None).await?;
        }
        let output = transparency_output(&tx, sidecar, config, tenant_id, election_event_id).await;
        tx.commit().await?;
        return output;
    }
    if let Some(log) = logs.iter().find(|l| l.visibility != mode) {
        return refused(format!(
            "The event's logs are {}: their visibility can't change",
            log.visibility
        ));
    }
    let Some(config) = config else {
        return refused(format!(
            "The deployment has no Vitrine configuration: set {}",
            super::config::CONFIG_FILE_ENV
        ));
    };
    let Some(sidecar) = sidecar else {
        return refused("The deployment has no Vitrine sidecar: set HARVEST_VITRINE_INTERNAL_URL");
    };
    check_mode(config, mode).map_err(|e| TransparencyError::Refused(e.to_string()))?;
    tx.commit().await?;

    for name in LogName::iter() {
        if !logs.iter().any(|l| l.name == name) {
            create_log(
                db,
                sidecar,
                config,
                tenant_id,
                election_event_id,
                name,
                mode,
            )
            .await?;
        }
    }

    let tx = db.transaction().await?;
    lock_unopened(&tx, tenant_id, election_event_id).await?;
    let salt = (mode == TransparencyMode::PRIVATE)
        .then(|| hex::encode(rand::random::<[u8; SALT_BYTES]>()));
    let vitrine = ensure_vitrine_event(&tx, tenant_id, election_event_id, salt.as_deref()).await?;
    let logs = get_vitrine_logs(&tx, tenant_id, election_event_id).await?;
    update_presentation_transparency(
        &tx,
        tenant_id,
        election_event_id,
        mode,
        Some(&presentation_for(mode, &logs, Some(&vitrine))),
    )
    .await?;
    let output = transparency_output(
        &tx,
        Some(sidecar),
        Some(config),
        tenant_id,
        election_event_id,
    )
    .await?;
    tx.commit().await?;
    Ok(output)
}

/// Checks that a signed policy is for this event before the sidecar sees it.
pub(super) fn check_policy_facts(
    facts: &PolicyFacts,
    mode: TransparencyMode,
    election_event_id: &str,
    election_ids: &[String],
) -> TransparencyResult<()> {
    if facts.visibility != mode {
        return refused(format!(
            "The policy is {}, and the event is {mode}",
            facts.visibility
        ));
    }
    if facts.event_id.as_deref() != Some(canonical_id(election_event_id)?.as_str()) {
        return refused("The policy is for another election event");
    }
    let elections: BTreeSet<String> = election_ids
        .iter()
        .map(|id| canonical_id(id))
        .collect::<anyhow::Result<_>>()?;
    if facts.election_ids != elections {
        return refused(
            "The policy doesn't list the event's elections: draft it again and have it sealed",
        );
    }
    Ok(())
}

/// Posts the logs' enrolment notes to the development witnesses of
/// `HARVEST_VITRINE_DEV_WITNESS_ADMINS`. A failure only warns: production
/// witnesses' operators post the notes themselves.
async fn enrol_in_dev_witnesses(notes: &[String]) {
    let Ok(value) = std::env::var(DEV_WITNESS_ADMINS_ENV) else {
        return;
    };
    let witnesses = match dev_witness_admins(&value) {
        Ok(witnesses) => witnesses,
        Err(e) => {
            warn!("{e}");
            return;
        }
    };
    for (url, token_file) in witnesses {
        let token = match read_token_file(&token_file) {
            Ok(token) => token,
            Err(e) => {
                warn!("Error reading the admin token of witness {url}: {e}");
                continue;
            }
        };
        for note in notes {
            if let Err(e) = post_enrolment(&url, &token, note).await {
                warn!("Error enrolling a log in witness {url}: {e}");
            }
        }
    }
}

/// `upload_transparency_policy`: installs the Board's signed policy in both
/// logs, which refuse one that isn't sealed, and keeps it with the logs'
/// acknowledgements and witness enrolment notes.
pub async fn upload_transparency_policy(
    db: &mut DbClient,
    sidecar: Option<&SidecarClient>,
    config: Option<&DeploymentConfig>,
    tenant_id: &str,
    election_event_id: &str,
    signed_policy: &str,
) -> TransparencyResult<TransparencyOutput> {
    let Some(sidecar) = sidecar else {
        return refused("The deployment has no Vitrine sidecar: set HARVEST_VITRINE_INTERNAL_URL");
    };
    let tx = db.transaction().await?;
    let event = lock_unopened(&tx, tenant_id, election_event_id).await?;
    let mode = transparency_mode(&event)?;
    if !mode.is_enabled() {
        return refused("Enable the event's transparency first");
    }
    let logs = get_vitrine_logs(&tx, tenant_id, election_event_id).await?;
    if logs.len() != LogName::iter().count() {
        return refused("The event's logs aren't created yet: enable its transparency again");
    }
    let elections = get_elections(&tx, tenant_id, election_event_id).await?;
    tx.commit().await?;

    let facts = PolicyFacts::read(signed_policy)
        .map_err(|e| TransparencyError::Refused(format!("This isn't a Vitrine policy: {e}")))?;
    let election_ids: Vec<String> = elections.iter().map(|e| e.id.clone()).collect();
    check_policy_facts(&facts, mode, election_event_id, &election_ids)?;

    let mut installed = Vec::new();
    for log in &logs {
        installed.push((
            log,
            sidecar.put_policy(&log.sidecar_name, signed_policy).await?,
        ));
    }
    let policy_id = installed
        .first()
        .map(|(_, p)| p.policy_id.clone())
        .ok_or_else(|| anyhow!("The event has no logs"))?;
    if installed.iter().any(|(_, p)| p.policy_id != policy_id) {
        return Err(anyhow!("The event's logs gave the policy different IDs").into());
    }
    let mut enrolments = Vec::new();
    for (log, _) in &installed {
        enrolments.push(sidecar.enrolment(&log.sidecar_name).await?);
    }

    let tx = db.transaction().await?;
    lock_unopened(&tx, tenant_id, election_event_id).await?;
    set_vitrine_event_policy(&tx, tenant_id, election_event_id, signed_policy, &policy_id).await?;
    for ((log, policy), enrolment) in installed.iter().zip(&enrolments) {
        set_vitrine_log_policy(
            &tx,
            tenant_id,
            election_event_id,
            log.name,
            &policy.ack,
            enrolment.as_deref(),
        )
        .await?;
    }
    let vitrine = get_vitrine_event(&tx, tenant_id, election_event_id).await?;
    update_presentation_transparency(
        &tx,
        tenant_id,
        election_event_id,
        mode,
        Some(&presentation_for(mode, &logs, vitrine.as_ref())),
    )
    .await?;
    let output =
        transparency_output(&tx, Some(sidecar), config, tenant_id, election_event_id).await?;
    tx.commit().await?;

    enrol_in_dev_witnesses(&enrolments.into_iter().flatten().collect::<Vec<_>>()).await;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT: &str = "e7a2b3c4-0000-4000-8000-000000000001";
    const MAYOR: &str = "e7a2b3c4-0000-4000-8000-000000000002";
    const COUNCIL: &str = "e7a2b3c4-0000-4000-8000-000000000003";

    fn log(name: LogName) -> VitrineLog {
        VitrineLog {
            name,
            sidecar_name: format!("t/{EVENT}/{name}"),
            log_id: 7,
            instance_id: "i".into(),
            origin: format!("vote.localhost/t/{EVENT}/{name}"),
            url: format!("http://localhost:8094/t/{EVENT}/{name}/"),
            vkey: "k".into(),
            log_time_vkey: "kt".into(),
            visibility: TransparencyMode::PUBLIC,
            submission_token: (name == LogName::Events).then(|| "secret".into()),
            policy_ack: None,
            enrolment: None,
            frozen: false,
        }
    }

    fn event(presentation: Option<Value>) -> ElectionEvent {
        serde_json::from_value(json!({
            "id": EVENT,
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "presentation": presentation,
            "is_archived": false,
            "encryption_protocol": "RSA256",
        }))
        .unwrap()
    }

    #[test]
    fn the_mode_is_read_on_its_own() {
        assert_eq!(
            transparency_mode(&event(None)).unwrap(),
            TransparencyMode::DISABLED
        );
        assert_eq!(
            transparency_mode(&event(Some(json!({ "transparency": "public" })))).unwrap(),
            TransparencyMode::PUBLIC
        );
        assert_eq!(
            transparency_mode(&event(Some(
                json!({ "transparency": "private", "language_conf": 3 })
            )))
            .unwrap(),
            TransparencyMode::PRIVATE
        );
        assert!(transparency_mode(&event(Some(json!({ "transparency": "open" })))).is_err());
    }

    #[test]
    fn voters_read_no_token_and_the_policy_of_public_events_only() {
        let logs = [log(LogName::Ballots), log(LogName::Events)];
        let vitrine = VitrineEvent {
            policy_salt: None,
            policy: Some("vitrine/policy/v1\n".into()),
            policy_id: Some(format!("sha256:{}", "ab".repeat(32))),
        };
        let public = presentation_for(TransparencyMode::PUBLIC, &logs, Some(&vitrine));
        assert_eq!(public.policy_id, Some("ab".repeat(32)));
        assert_eq!(public.policy.as_deref(), Some("vitrine/policy/v1\n"));
        let text = serde_json::to_string(&public).unwrap();
        assert!(!text.contains("secret"), "{text}");
        assert_eq!(
            public.logs.unwrap().events.unwrap().origin,
            format!("vote.localhost/t/{EVENT}/events")
        );
        let private = presentation_for(TransparencyMode::PRIVATE, &logs, Some(&vitrine));
        assert_eq!(private.policy, None);
        assert_eq!(private.policy_id, Some("ab".repeat(32)));
        let before_upload = presentation_for(TransparencyMode::PUBLIC, &logs, None);
        assert_eq!(before_upload.policy_id, None);
        assert_eq!(before_upload.policy, None);
    }

    #[test]
    fn created_and_imported_events_start_without_transparency() {
        let presentation = json!({
            "language_conf": { "default_language_code": "en" },
            "transparency": "public",
            "vitrine": { "policy_id": "ab" },
        });
        assert_eq!(
            without_transparency(Some(presentation)),
            Some(json!({ "language_conf": { "default_language_code": "en" } }))
        );
        assert_eq!(without_transparency(None), None);
        assert_eq!(without_transparency(Some(json!([1]))), Some(json!([1])));
    }

    #[test]
    fn dev_witnesses_are_url_and_token_file_pairs() {
        assert_eq!(
            dev_witness_admins("http://vitrine-log:8097|/a, http://vitrine-log:8099|/b").unwrap(),
            [
                ("http://vitrine-log:8097".to_string(), "/a".to_string()),
                ("http://vitrine-log:8099".to_string(), "/b".to_string())
            ]
        );
        assert!(dev_witness_admins("").unwrap().is_empty());
        assert!(dev_witness_admins("http://vitrine-log:8097").is_err());
        assert!(dev_witness_admins("|/a").is_err());
    }

    fn facts(visibility: TransparencyMode, event_id: &str, elections: &[&str]) -> PolicyFacts {
        PolicyFacts {
            visibility,
            opens: 1,
            closes: 2,
            event_id: Some(event_id.into()),
            election_ids: elections.iter().map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn a_policy_is_for_the_events_mode_and_elections() {
        let elections = [MAYOR.to_string(), COUNCIL.to_string()];
        let ok = facts(TransparencyMode::PUBLIC, EVENT, &[COUNCIL, MAYOR]);
        assert!(check_policy_facts(&ok, TransparencyMode::PUBLIC, EVENT, &elections).is_ok());
        let refusals = [
            check_policy_facts(&ok, TransparencyMode::PRIVATE, EVENT, &elections),
            check_policy_facts(
                &facts(TransparencyMode::PUBLIC, MAYOR, &[COUNCIL, MAYOR]),
                TransparencyMode::PUBLIC,
                EVENT,
                &elections,
            ),
            check_policy_facts(
                &facts(TransparencyMode::PUBLIC, EVENT, &[MAYOR]),
                TransparencyMode::PUBLIC,
                EVENT,
                &elections,
            ),
        ];
        for refusal in refusals {
            assert!(matches!(refusal, Err(TransparencyError::Refused(_))));
        }
    }
}
