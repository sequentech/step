// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Electoral-log audits.
//!
//! An audit compares an election event's stored records with their Merkle leaves,
//! recomputes the stored tree, and checks every published checkpoint: its signature,
//! its signer, and that it is part of the log's history. The result is recorded on a
//! task execution; findings are reported, never repaired.
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::electoral_log_checkpoint::{
    get_electoral_log_checkpoints, PublishedCheckpoint,
};
use crate::postgres::tally_session_execution::append_tally_session_log;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::{get_electoral_log_store, get_protocol_manager};
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::tasks_execution::{post, update_fail, update_with_annotations};
use crate::tasks::audit_electoral_log::audit_electoral_log;
use crate::types::tasks::ETasksExecution;
use anyhow::{Context, Result};
use b4::messages::message::Signer;
use electoral_log::adapters::postgres::{AuditAnnotations, AuditOutcome};
use electoral_log::messages::newtypes::ElectoralLogCheckpointReason;
use electoral_log::proofs::{verify_checkpoint_signature, Checkpoint};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::Value;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignaturePk;
use tracing::instrument;

/// Recorded as the executor of audits the platform starts by itself.
pub const SYSTEM_EXECUTOR: &str = "system";
/// Findings written to the task logs; the total count is always recorded.
const LOGGED_FINDINGS: usize = 100;

/// Result of one audit run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditResult {
    pub board: String,
    /// Size of the log in the audited snapshot.
    pub tree_size: i64,
    /// Hex-encoded stored root in the audited snapshot.
    pub root: String,
    pub published_checkpoints: usize,
    /// Integrity problems found; empty when the log is consistent.
    pub findings: Vec<String>,
}

/// After a tally completes: publish a checkpoint of the event's electoral log and
/// start an audit, reporting both in the tally session's logs. Failures are reported
/// and never propagated, so they cannot fail the tally.
pub async fn checkpoint_and_audit_electoral_log(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) {
    let published = publish_event_checkpoint(
        tenant_id,
        election_event_id,
        ElectoralLogCheckpointReason::TallyCompleted,
    )
    .await;
    let line = match published {
        Ok(checkpoint) => format!(
            "Published electoral-log checkpoint at {} entries, root {}",
            checkpoint.tree_size,
            hex::encode(&checkpoint.root)
        ),
        Err(error) => {
            tracing::error!(
                "Could not publish the electoral-log checkpoint for tally session {tally_session_id}: {error:?}"
            );
            format!("Could not publish the electoral-log checkpoint: {error:#}")
        }
    };
    log_to_tally(tenant_id, election_event_id, tally_session_id, &line).await;
    if let Err(error) = start_electoral_log_audit(
        tenant_id,
        election_event_id,
        Some(tally_session_id),
        SYSTEM_EXECUTOR,
    )
    .await
    {
        tracing::error!(
            "Could not start the electoral-log audit for tally session {tally_session_id}: {error:?}"
        );
        log_to_tally(
            tenant_id,
            election_event_id,
            tally_session_id,
            &format!("Could not start the electoral-log audit: {error:#}"),
        )
        .await;
    }
}

/// Publish a signed checkpoint of an election event's electoral log, signed with the
/// event's protocol-manager key.
pub async fn publish_event_checkpoint(
    tenant_id: &str,
    election_event_id: &str,
    reason: ElectoralLogCheckpointReason,
) -> Result<Checkpoint> {
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
    let board = get_election_event_board(event.bulletin_board_reference)
        .context("Election event has no electoral-log board")?;
    let electoral_log =
        ElectoralLog::new(&transaction, tenant_id, Some(election_event_id), &board).await?;
    transaction.commit().await?;
    drop(client);
    electoral_log
        .publish_checkpoint(tenant_id, election_event_id, reason)
        .await
}

/// Create the audit task and enqueue it. Audits started by a tally also report to
/// the tally session's logs.
#[instrument(err)]
pub async fn start_electoral_log_audit(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: Option<&str>,
    executed_by_user: &str,
) -> Result<TasksExecution> {
    let task = post(
        tenant_id,
        Some(election_event_id),
        ETasksExecution::AUDIT_ELECTORAL_LOG,
        executed_by_user,
    )
    .await?;
    if let Some(tally_session_id) = tally_session_id {
        log_to_tally(
            tenant_id,
            election_event_id,
            tally_session_id,
            &format!("Electoral-log audit queued (task {})", task.id),
        )
        .await;
    }
    let sent = get_celery_app()
        .await
        .send_task(audit_electoral_log::new(
            tenant_id.to_string(),
            election_event_id.to_string(),
            tally_session_id.map(str::to_string),
            task.clone(),
        ))
        .await;
    if let Err(error) = sent {
        let message = format!("Error enqueuing the electoral-log audit: {error:?}");
        if let Err(update_error) = update_fail(&task, &message).await {
            tracing::error!("Failed to mark the audit task as failed: {update_error:?}");
        }
        anyhow::bail!(message);
    }
    Ok(task)
}

/// Append a line to a tally session's logs. Failures are logged, not propagated: the
/// audit result is already recorded on its task.
pub async fn log_to_tally(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    text: &str,
) {
    let result: Result<()> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.transaction().await?;
        append_tally_session_log(
            &transaction,
            tenant_id,
            election_event_id,
            tally_session_id,
            text,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::error!(
            "Could not add '{text}' to the logs of tally session {tally_session_id}: {error:?}"
        );
    }
}

/// Audit an election event's electoral log against its published checkpoints.
#[instrument(err)]
pub async fn run_electoral_log_audit(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<AuditResult> {
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
    let board = get_election_event_board(event.bulletin_board_reference.clone())
        .context("Election event has no electoral-log board")?;
    let protocol_manager = get_protocol_manager::<RistrettoCtx>(
        &transaction,
        tenant_id,
        Some(election_event_id),
        &board,
    )
    .await?;
    let expected_signer =
        StrandSignaturePk::from_sk(protocol_manager.get_signing_key())?.to_der_b64_string()?;
    let published =
        get_electoral_log_checkpoints(&transaction, tenant_id, election_event_id).await?;
    transaction.commit().await?;
    drop(client);

    let (checkpoints, mut findings) = check_publications(&board, &expected_signer, &published);
    let report = get_electoral_log_store()
        .await?
        .audit(&board, &checkpoints)
        .await?;
    findings.extend(report.findings());
    Ok(AuditResult {
        board,
        tree_size: report.committed_size,
        root: report.root,
        published_checkpoints: published.len(),
        findings,
    })
}

/// Check the signatures, signers and reasons of published checkpoints, returning the
/// checkpoints to verify against the log history and the findings so far.
fn check_publications(
    board: &str,
    expected_signer: &str,
    published: &[PublishedCheckpoint],
) -> (Vec<Checkpoint>, Vec<String>) {
    let mut checkpoints = Vec::new();
    let mut findings = Vec::new();
    for publication in published {
        let size = publication.tree_size;
        let (Ok(tree_size), Ok(root), Ok(reason)) = (
            u64::try_from(publication.tree_size),
            hex::decode(&publication.root),
            publication.reason.parse::<ElectoralLogCheckpointReason>(),
        ) else {
            findings.push(format!("Published checkpoint at size {size} is malformed"));
            continue;
        };
        let checkpoint = Checkpoint {
            log_name: publication.board_name.clone(),
            log_id: publication.log_id,
            tree_size,
            root,
        };
        if publication.board_name != board {
            findings.push(format!(
                "Published checkpoint at size {size} names board {} instead of {board}",
                publication.board_name
            ));
            continue;
        }
        if publication.signer_pk != expected_signer {
            findings.push(format!(
                "Published checkpoint at size {size} is signed by an unexpected key"
            ));
        } else if let Err(error) = verify_checkpoint_signature(
            &checkpoint,
            reason,
            &publication.signer_pk,
            &publication.signature,
        ) {
            findings.push(format!(
                "Published checkpoint at size {size} has an invalid signature: {error:#}"
            ));
        }
        checkpoints.push(checkpoint);
    }
    (checkpoints, findings)
}

/// Record an audit result on its task and return the one-line summary.
pub async fn record_audit(task: &TasksExecution, result: &Result<AuditResult>) -> Result<String> {
    let mut logs: Option<Value> = task.logs.clone();
    let mut log = |line: &str| -> Result<()> {
        logs = Some(serde_json::to_value(append_general_log(&logs, line))?);
        Ok(())
    };
    let (outcome, summary, annotations) = match result {
        Ok(audit) => {
            log(&format!(
                "Audited board {}: {} entries, root {}, {} published checkpoints",
                audit.board, audit.tree_size, audit.root, audit.published_checkpoints
            ))?;
            for finding in audit.findings.iter().take(LOGGED_FINDINGS) {
                log(&format!("Finding: {finding}"))?;
            }
            if audit.findings.len() > LOGGED_FINDINGS {
                log(&format!(
                    "... and {} more findings",
                    audit.findings.len() - LOGGED_FINDINGS
                ))?;
            }
            let (outcome, summary) = if audit.findings.is_empty() {
                (AuditOutcome::Clean, AuditOutcome::Clean.to_string())
            } else {
                (
                    AuditOutcome::Findings,
                    format!("{} findings", audit.findings.len()),
                )
            };
            (
                outcome,
                summary,
                AuditAnnotations {
                    outcome,
                    board: Some(audit.board.clone()),
                    tree_size: Some(audit.tree_size),
                    root: Some(audit.root.clone()),
                    findings: Some(audit.findings.len()),
                    published_checkpoints: Some(audit.published_checkpoints),
                },
            )
        }
        Err(error) => {
            log(&format!("Error: {error:#}"))?;
            (
                AuditOutcome::Error,
                AuditOutcome::Error.to_string(),
                AuditAnnotations {
                    outcome: AuditOutcome::Error,
                    board: None,
                    tree_size: None,
                    root: None,
                    findings: None,
                    published_checkpoints: None,
                },
            )
        }
    };
    log(&format!("Electoral-log audit {summary}"))?;
    let status = match outcome {
        AuditOutcome::Clean => TasksExecutionStatus::SUCCESS,
        AuditOutcome::Findings | AuditOutcome::Error => TasksExecutionStatus::FAILED,
    };
    update_with_annotations(
        &task.tenant_id,
        &task.id,
        status,
        logs.unwrap_or(Value::Array(vec![])),
        serde_json::to_value(&annotations)?,
    )
    .await?;
    Ok(format!("Electoral-log audit {}: {summary}", task.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::electoral_log::sign_checkpoint;
    use electoral_log::messages::message::SigningData;
    use strand::signature::StrandSignatureSk;

    fn publication(sk: &StrandSignatureSk, board: &str, size: i64) -> PublishedCheckpoint {
        let checkpoint = Checkpoint {
            log_name: board.to_string(),
            log_id: 1,
            tree_size: u64::try_from(size).unwrap(),
            root: vec![9; 32],
        };
        let sd = SigningData::new(sk.clone(), "", sk.clone());
        sign_checkpoint(&sd, &checkpoint, ElectoralLogCheckpointReason::VotingClosed).unwrap()
    }

    fn public_key(sk: &StrandSignatureSk) -> String {
        StrandSignaturePk::from_sk(sk)
            .unwrap()
            .to_der_b64_string()
            .unwrap()
    }

    #[test]
    fn published_checkpoints_pass_the_audit_signature_checks() {
        let key = StrandSignatureSk::generate().unwrap();
        let sd = SigningData::new(key.clone(), "", key.clone());
        let checkpoint = Checkpoint {
            log_name: "board".to_string(),
            log_id: 2,
            tree_size: 17,
            root: vec![3; 32],
        };
        let mut published = Vec::new();
        for reason in [
            ElectoralLogCheckpointReason::VotingClosed,
            ElectoralLogCheckpointReason::TallyCompleted,
        ] {
            published.push(sign_checkpoint(&sd, &checkpoint, reason).unwrap());
        }
        let (checkpoints, findings) = check_publications("board", &public_key(&key), &published);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(checkpoints, vec![checkpoint.clone(), checkpoint]);

        published[0].reason = ElectoralLogCheckpointReason::TallyCompleted.to_string();
        let (_, findings) = check_publications("board", &public_key(&key), &published);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("invalid signature"));
    }

    #[test]
    fn publications_need_the_event_key_a_valid_signature_and_the_board() {
        let key = StrandSignatureSk::generate().unwrap();
        let other = StrandSignatureSk::generate().unwrap();
        let good = publication(&key, "board", 4);
        let foreign_signer = publication(&other, "board", 5);
        let mut tampered = publication(&key, "board", 6);
        tampered.root = hex::encode([8; 32]);
        let other_board = publication(&key, "elsewhere", 7);
        let mut malformed = publication(&key, "board", 8);
        malformed.root = "zz".into();
        let mut unknown_reason = publication(&key, "board", 9);
        unknown_reason.reason = "voting_closed".into();
        let (checkpoints, findings) = check_publications(
            "board",
            &public_key(&key),
            &[
                good,
                foreign_signer,
                tampered,
                other_board,
                malformed,
                unknown_reason,
            ],
        );
        assert_eq!(
            checkpoints.iter().map(|c| c.tree_size).collect::<Vec<_>>(),
            vec![4, 5, 6]
        );
        assert_eq!(findings.len(), 5, "{findings:?}");
        assert!(findings[0].contains("unexpected key"));
        assert!(findings[1].contains("invalid signature"));
        assert!(findings[2].contains("names board"));
        assert!(findings[3].contains("malformed"));
        assert!(findings[4].contains("malformed"));
    }
}
