// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Fixtures of the protected actions' tests: a dispatcher that records the
//! tasks the last signature sends, effects that stand in for today's
//! services, a seal hook that records what the close fed it and answers two
//! country seals, and the gates as a route calls them, committed. Include it after `signing`:
//! `#[path = "support/signing_actions.rs"] mod signing_actions;`.

#![allow(dead_code)]

use super::signing::*;
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::signing::SigningAction;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use windmill::postgres::signing::{SigningApprovalRow, SigningRequestRow};
use windmill::services::signing::actions::voting::{gate_election_status, SealRecord, SealSummary};
use windmill::services::signing::actions::{
    executors, refuse, EffectProgress, SealRecordSink, SignedActionDispatcher, SignedActionEffects,
    SignedActionTask,
};
use windmill::services::signing::approve::SigningServices;
use windmill::services::signing::executors::{PostCommit, PostCommitTask};
use windmill::services::signing::guard::GuardOutcome;
use windmill::services::signing::{SigningCaller, SigningResult};

/// Records the tasks the last signature dispatches.
#[derive(Default, Clone)]
pub struct Dispatched(pub Arc<Mutex<Vec<SignedActionTask>>>);

struct Recorded(SignedActionTask, Arc<Mutex<Vec<SignedActionTask>>>);

#[async_trait]
impl PostCommitTask for Recorded {
    async fn send(&self) -> anyhow::Result<()> {
        self.1.lock().unwrap().push(self.0);
        Ok(())
    }
}

impl SignedActionDispatcher for Dispatched {
    fn task(&self, task: SignedActionTask) -> PostCommit {
        PostCommit::SendTask(Box::new(Recorded(task, self.0.clone())))
    }
}

impl Dispatched {
    pub fn tasks(&self) -> Vec<SignedActionTask> {
        self.0.lock().unwrap().clone()
    }
}

/// What a [`FakeEffects`] does after its write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Succeed,
    /// Fails before any call outside the database.
    FailBefore,
    /// Fails after a call outside the database.
    FailAfter,
    /// Refuses: the state is not what was signed.
    Refuse,
}

/// Stands in for today's services: records each run, writes into the
/// effect's transaction, and answers what a status change answers. An open
/// or close that succeeds sets its channels' status on the Post, as the
/// status change does.
pub struct FakeEffects {
    runs: Mutex<Vec<Uuid>>,
    effect: Effect,
}

impl FakeEffects {
    pub fn new(effect: Effect) -> Self {
        FakeEffects {
            runs: Mutex::new(vec![]),
            effect,
        }
    }

    pub fn runs(&self) -> usize {
        self.runs.lock().unwrap().len()
    }
}

#[async_trait]
impl SignedActionEffects for FakeEffects {
    async fn run(
        &self,
        tx: &Transaction<'_>,
        request: &SigningRequestRow,
        _approvals: &[SigningApprovalRow],
        progress: &EffectProgress,
    ) -> anyhow::Result<Value> {
        self.runs.lock().unwrap().push(request.id);
        if self.effect == Effect::Refuse {
            return Err(refuse("state-changed", "The Post changed."));
        }
        // A write of the effect, which commits with its result or not at all.
        tx.execute(
            "UPDATE sequent_backend.election SET description = 'ran' WHERE id = $1",
            &[&request.election_id],
        )
        .await?;
        match self.effect {
            Effect::FailBefore => anyhow::bail!("the database is busy"),
            Effect::FailAfter => {
                progress.reach_outside();
                anyhow::bail!("the board is unreachable")
            }
            _ => {
                if let Some(target) = target_status(request.action) {
                    set_channels(tx, request, target).await?;
                }
                Ok(json!({
                    "channels": request.subject.get("channels").cloned().unwrap_or(json!([])),
                    "from": request.subject.get("from").cloned().unwrap_or(json!([])),
                }))
            }
        }
    }
}

/// The status an open or close sets.
fn target_status(action: SigningAction) -> Option<&'static str> {
    match action {
        SigningAction::OpenVoting => Some("OPEN"),
        SigningAction::CloseVoting => Some("CLOSED"),
        _ => None,
    }
}

/// Sets the request's channels to `target` in its Post's status.
async fn set_channels(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    target: &str,
) -> anyhow::Result<()> {
    let mut status = serde_json::Map::new();
    for channel in request
        .subject
        .get("channels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let key = match channel {
            "ONLINE" => "voting_status",
            "KIOSK" => "kiosk_voting_status",
            "EARLY_VOTING" => "early_voting_status",
            "TELEPHONE" => "telephone_voting_status",
            other => anyhow::bail!("unknown channel {other}"),
        };
        status.insert(key.to_owned(), json!(target));
    }
    // As the real effect does.
    windmill::postgres::trusted_write(tx).await?;
    tx.execute(
        "UPDATE sequent_backend.election
         SET status = COALESCE(status, '{}'::jsonb) || $2::jsonb WHERE id = $1",
        &[&request.election_id, &Value::Object(status)],
    )
    .await?;
    Ok(())
}

/// The two countries [`RecordingSeal`] answers seals for: id, name, the
/// seal's SHA-512 hash and its ballots.
pub fn sealed_countries() -> [(Uuid, &'static str, String, u64); 2] {
    [
        (
            Uuid::from_u128(0x5a1e_0001),
            "Spain",
            "3f9a".repeat(32),
            1356,
        ),
        (
            Uuid::from_u128(0x5a1e_0002),
            "Portugal",
            "c0d7".repeat(32),
            804,
        ),
    ]
}

/// What a [`RecordingSeal`] states signed each seal, for a record with
/// `signatures` closing signatures under `code`.
pub fn sealed_by(signatures: usize, code: &str) -> String {
    format!("{signatures} closing signatures, signing code {code}")
}

/// Stands in for VOTE-FREEZE's hook: records each record it is fed and the
/// Post's status it reads in the close's transaction, and answers one seal
/// per [`sealed_countries`], signed by the record's closing signatures.
#[derive(Default)]
pub struct RecordingSeal(pub Mutex<Vec<(SealRecord, Option<Value>)>>);

#[async_trait]
impl SealRecordSink for RecordingSeal {
    async fn feed(
        &self,
        tx: &Transaction<'_>,
        record: &SealRecord,
    ) -> anyhow::Result<Vec<SealSummary>> {
        let status: Option<Value> = tx
            .query_one(
                "SELECT status FROM sequent_backend.election WHERE id = $1",
                &[&record.election_id],
            )
            .await?
            .get(0);
        self.0.lock().unwrap().push((record.clone(), status));
        let signed_by = sealed_by(record.signatures.len(), &record.code);
        Ok(sealed_countries()
            .into_iter()
            .map(|(area_id, area_name, hash, ballots)| SealSummary {
                area_id: Some(area_id),
                area_name: Some(area_name.to_owned()),
                hash_algorithm: "SHA-512".to_owned(),
                hash,
                ballots: Some(ballots),
                signed_by: Some(signed_by.clone()),
            })
            .collect())
    }
}

/// A starter of the world's Post actions: the start permissions and the
/// Post's label.
pub fn starter(w: &World) -> SigningCaller {
    caller(
        "starter",
        &[
            Permissions::ELECTION_STATE_WRITE,
            Permissions::PUBLISH_WRITE,
            Permissions::ADMIN_CEREMONY,
        ],
        &[&w.label],
    )
}

/// Sets the Post's voting status (its JSON `status`).
pub async fn set_post_status(w: &World, status: Value) {
    w.execute(
        "UPDATE sequent_backend.election SET status = $2 WHERE id = $1 AND set_config('sequent.trusted_write', 'on', true) = 'on'",
        &[&w.post, &status],
    )
    .await;
}

/// The Post's status, as stored.
pub async fn post_status(w: &World) -> Option<Value> {
    w.pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT status FROM sequent_backend.election WHERE id = $1",
            &[&w.post],
        )
        .await
        .unwrap()
        .get(0)
}

/// The gate of a Post's status change, as the route calls it, committed.
pub async fn status_gate(
    w: &World,
    who: &SigningCaller,
    status: VotingStatus,
    channels: Option<Vec<VotingStatusChannel>>,
) -> SigningResult<GuardOutcome> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate_election_status(
        &tx,
        who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &w.post.to_string(),
        &status,
        &channels,
    )
    .await;
    tx.commit().await.unwrap();
    outcome
}

pub fn waiting_id(outcome: SigningResult<GuardOutcome>) -> Uuid {
    match outcome {
        Ok(GuardOutcome::SigningRequired(summary)) => summary.id,
        other => panic!("expected a signing request, got {other:?}"),
    }
}

/// `n` signers of `action` with registered certificates, named `prefix-i`.
pub async fn signers(
    w: &World,
    action: SigningAction,
    n: usize,
    prefix: &str,
) -> Vec<(SigningCaller, TestCert)> {
    let mut signers = vec![];
    for i in 0..n {
        let user = format!("{prefix}-{i}");
        let certificate = cert_with_pem(&user, &user, &user, &real_pem(&format!("SBEI {i}")));
        w.register(&user, &certificate, None).await;
        signers.push((w.signer(&user, action), certificate));
    }
    signers
}

/// Signs waiting request `id` of `action` with `n` new signers, recording
/// the tasks in `dispatched`; the approve step's services.
pub async fn sign_all(
    w: &World,
    action: SigningAction,
    id: Uuid,
    n: usize,
    prefix: &str,
    dispatched: &Dispatched,
) -> SigningServices {
    let signers = signers(w, action, n, prefix).await;
    let services = services(
        FakeVerifier::knowing(&signers.iter().map(|(_, c)| c).collect::<Vec<_>>()),
        executors(Arc::new(dispatched.clone())),
    );
    for (i, (signer, certificate)) in signers.iter().enumerate() {
        w.sign(&services, signer, id, certificate, at(1 + i as u32))
            .await
            .unwrap();
    }
    services
}

/// The task row's status.
pub async fn task_status(w: &World, task: &SignedActionTask) -> String {
    w.pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT execution_status FROM sequent_backend.tasks_execution WHERE id = $1",
            &[&task.task_execution_id],
        )
        .await
        .unwrap()
        .get(0)
}

/// Builds protected publication fixtures through an explicitly trusted transaction.
pub async fn publication_fixture_write(
    world: &World,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) {
    let mut client = world.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    windmill::postgres::trusted_write(&tx).await.unwrap();
    tx.execute(sql, params).await.unwrap();
    tx.commit().await.unwrap();
}
