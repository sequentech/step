// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustees' key steps behind their own signature (`Gate` mode):
//! confirming a key share in the keys ceremony and contributing it to a
//! tally.
//!
//! With the action's rule `Required`, a trustee first starts a request whose
//! subject names the SHA-256 of their key share (never the key share
//! itself), signs it, and then takes the step with the request's id: the
//! step consumes the completed request, keeps a reference to it with the
//! ceremony and runs. While the rule is `Required`, only signed steps count
//! towards a ceremony's success and a tally's trustees
//! ([`TrusteeSignatures`]). With the rule `NotRequired` the step runs as it
//! always did.

use super::guard::{
    consume_gate, effective_rule, guard, GuardOutcome, GuardRequest, RequestScope,
    SigningRequestSummary,
};
use super::{InvalidReason, SigningCaller, SigningError};
use crate::domain::trustee_signatures::TrusteeSignatures;
use crate::postgres::signing::{
    find_signing_trustee_id, get_signing_trustee_name, list_signing_approvals, SigningRequestRow,
};
use crate::services::ceremonies::{keys_ceremony, tally_ceremony};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::{
    sha256_hex, KeyCeremonySubject, SigningAction, SigningRequestStatus, TallyKeySubject,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fmt;
use tracing::instrument;
use uuid::Uuid;

/// The prefix of the ceremony annotation that indexes a trustee's signed
/// step, followed by the trustee's id. The annotation is only an index: a
/// verifier reads and checks the signature from the request and its
/// approval, never from the annotation.
pub const KEY_SHARE_SIGNATURE_ANNOTATION: &str = "signing:key-share:";

/// Which of the trustees' key steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyShareKind {
    /// Confirm a key share: the trustee checks it in the keys ceremony.
    Confirm,
    /// Contribute a key share: the trustee restores it for a tally session.
    Contribute,
}

impl KeyShareKind {
    pub fn action(&self) -> SigningAction {
        match self {
            KeyShareKind::Confirm => SigningAction::ConfirmKeyShare,
            KeyShareKind::Contribute => SigningAction::ContributeKeyShare,
        }
    }

    /// The subject field naming the keys ceremony or the tally session.
    fn id_field(&self) -> &'static str {
        match self {
            KeyShareKind::Confirm => "keys_ceremony_id",
            KeyShareKind::Contribute => "tally_session_id",
        }
    }

    /// The table the step's ceremony is kept in.
    fn table(&self) -> &'static str {
        match self {
            KeyShareKind::Confirm => "keys_ceremony",
            KeyShareKind::Contribute => "tally_session",
        }
    }
}

/// A trustee's key step and the keys ceremony or tally session it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyShareStep {
    pub kind: KeyShareKind,
    /// The keys ceremony or the tally session.
    pub id: Uuid,
}

impl KeyShareStep {
    pub fn action(&self) -> SigningAction {
        self.kind.action()
    }

    /// What the trustee signs: exactly the action's subject.
    fn subject(&self, trustee_id: Uuid, key_share_sha256: &str) -> anyhow::Result<Value> {
        let trustee_id = trustee_id.to_string();
        let key_share_sha256 = key_share_sha256.to_owned();
        Ok(match self.kind {
            KeyShareKind::Confirm => serde_json::to_value(KeyCeremonySubject {
                keys_ceremony_id: self.id.to_string(),
                trustee_id,
                key_share_sha256,
            })?,
            KeyShareKind::Contribute => serde_json::to_value(TallyKeySubject {
                tally_session_id: self.id.to_string(),
                trustee_id,
                key_share_sha256,
            })?,
        })
    }

    /// Keeps the reference to the trustee's signed step with the ceremony,
    /// as a JSON string (tally session annotations are read as strings).
    async fn record(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        trustee_id: Uuid,
        reference: &Value,
    ) -> anyhow::Result<()> {
        let sql = format!(
            "UPDATE sequent_backend.{}
             SET annotations = COALESCE(annotations, '{{}}'::jsonb) || jsonb_build_object($4::text, $5::text)
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            self.kind.table()
        );
        let key = format!("{KEY_SHARE_SIGNATURE_ANNOTATION}{trustee_id}");
        let updated = hasura_transaction
            .execute(
                &sql,
                &[
                    &tenant_id,
                    &election_event_id,
                    &self.id,
                    &key,
                    &reference.to_string(),
                ],
            )
            .await
            .context("Error recording the trustee's signed step")?;
        if updated != 1 {
            return Err(anyhow!("There is no such ceremony."));
        }
        Ok(())
    }
}

/// The ceremony side of a key step: the existing check or restore.
#[async_trait]
pub trait KeyShareCeremony: Send + Sync {
    /// Whether `key_share` is the trustee's, recording nothing. Fails as the
    /// step fails when the trustee can't take it now.
    async fn matches(
        &self,
        hasura_transaction: &Transaction<'_>,
        key_share: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool>;

    /// Takes the step as it always did: `false`, recording nothing, when
    /// `key_share` is not the trustee's. `signatures` says which trustees'
    /// steps count.
    async fn run(
        &self,
        hasura_transaction: &Transaction<'_>,
        key_share: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool>;
}

/// The keys ceremony's private key check.
pub struct KeysCeremonyKeyShare<'a> {
    pub claims: &'a JwtClaims,
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub keys_ceremony_id: &'a str,
}

#[async_trait]
impl KeyShareCeremony for KeysCeremonyKeyShare<'_> {
    async fn matches(
        &self,
        hasura_transaction: &Transaction<'_>,
        key_share: &str,
        _signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool> {
        // A trustee may check their key share again, signed or not.
        keys_ceremony::key_share_matches(
            hasura_transaction,
            self.claims.trustee.clone(),
            self.tenant_id,
            self.election_event_id,
            self.keys_ceremony_id,
            key_share,
        )
        .await
    }

    async fn run(
        &self,
        hasura_transaction: &Transaction<'_>,
        key_share: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool> {
        keys_ceremony::check_private_key(
            hasura_transaction,
            self.claims.clone(),
            self.tenant_id.to_owned(),
            self.election_event_id.to_owned(),
            self.keys_ceremony_id.to_owned(),
            key_share.to_owned(),
            signatures,
        )
        .await
    }
}

/// The tally's private key restore.
pub struct TallyKeyShare<'a> {
    pub claims: &'a JwtClaims,
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub tally_session_id: &'a str,
}

#[async_trait]
impl KeyShareCeremony for TallyKeyShare<'_> {
    async fn matches(
        &self,
        hasura_transaction: &Transaction<'_>,
        key_share: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool> {
        tally_ceremony::key_share_matches(
            hasura_transaction,
            self.claims,
            self.tenant_id,
            self.election_event_id,
            self.tally_session_id,
            key_share,
            signatures,
        )
        .await
    }

    async fn run(
        &self,
        hasura_transaction: &Transaction<'_>,
        key_share: &str,
        signatures: &TrusteeSignatures,
    ) -> anyhow::Result<bool> {
        tally_ceremony::set_private_key(
            hasura_transaction,
            self.claims,
            self.tenant_id,
            self.election_event_id,
            self.tally_session_id,
            key_share,
            signatures,
        )
        .await
    }
}

/// What a trustee sends with a key step. The ids are the route's, parsed
/// only once the step needs signatures, so a step without them answers as
/// it always did.
pub struct KeyShareInput<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub kind: KeyShareKind,
    /// The keys ceremony or the tally session.
    pub target_id: &'a str,
    /// The key share file's text, as the step always took it.
    pub key_share: &'a str,
    /// The SHA-256 the browser computed of `key_share`, lowercase hex.
    pub key_share_sha256: Option<&'a str>,
    /// The trustee's completed request, to take the step with.
    pub signing_request_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyShareOutcome {
    /// The key share is not the trustee's: the caller rolls back, so
    /// nothing is recorded and no request is used.
    Invalid,
    /// The step ran. With a request, the reference kept with the ceremony.
    Done(Option<Value>),
    /// The trustee signs the request first, then takes the step with it.
    SigningRequired(SigningRequestSummary),
}

/// Why a key step was not taken.
#[derive(Debug)]
pub enum KeyShareError {
    /// The ceremony refused the step (or couldn't read the rule), as the
    /// route always answered.
    Step(anyhow::Error),
    /// The signing side refused it.
    Signing(SigningError),
}

impl fmt::Display for KeyShareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyShareError::Step(error) => write!(f, "{error:?}"),
            KeyShareError::Signing(error) => write!(f, "{error}"),
        }
    }
}

impl From<SigningError> for KeyShareError {
    fn from(error: SigningError) -> Self {
        KeyShareError::Signing(error)
    }
}

fn internal(error: anyhow::Error) -> KeyShareError {
    KeyShareError::Signing(SigningError::Internal(error))
}

/// The ids of a step that needs signatures.
struct Ids {
    tenant_id: Uuid,
    election_event_id: Uuid,
    step: KeyShareStep,
}

fn parse(input: &KeyShareInput<'_>) -> Result<Ids, KeyShareError> {
    let uuid = |text: &str, name: &str| {
        Uuid::parse_str(text).map_err(|_| {
            KeyShareError::from(SigningError::bad_input(format!("{name} is not a UUID")))
        })
    };
    Ok(Ids {
        tenant_id: uuid(input.tenant_id, "tenant_id")?,
        election_event_id: uuid(input.election_event_id, "election_event_id")?,
        step: KeyShareStep {
            kind: input.kind,
            id: uuid(input.target_id, input.kind.id_field())?,
        },
    })
}

/// The trustee record the caller's token names.
struct Trustee {
    id: Uuid,
    name: String,
}

async fn caller_trustee(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
) -> Result<Trustee, KeyShareError> {
    let not_a_trustee = || SigningError::Forbidden("You are not a trustee.".into());
    let name = caller.trustee.clone().ok_or_else(not_a_trustee)?;
    let id = find_signing_trustee_id(hasura_transaction, tenant_id, &name)
        .await
        .map_err(internal)?
        .ok_or_else(not_a_trustee)?;
    Ok(Trustee { id, name })
}

/// The names of the trustees with a used signed step for `step`, besides
/// the request `excluding`.
async fn signed_trustee_names(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    step: KeyShareStep,
    excluding: Option<Uuid>,
) -> anyhow::Result<HashSet<String>> {
    Ok(hasura_transaction
        .query(
            "SELECT trustee.name
             FROM sequent_backend.signing_request request
             JOIN sequent_backend.trustee trustee
               ON trustee.tenant_id = request.tenant_id AND trustee.id = request.trustee_id
             WHERE request.tenant_id = $1 AND request.election_event_id = $2
               AND request.action = $3 AND request.status = $4
               AND request.subject->>$5 = $6
               AND ($7::uuid IS NULL OR request.id <> $7)",
            &[
                &tenant_id,
                &election_event_id,
                &step.action().to_string(),
                &SigningRequestStatus::Executed.to_string(),
                &step.kind.id_field(),
                &step.id.to_string(),
                &excluding,
            ],
        )
        .await
        .context("Error reading the trustees' signed steps")?
        .into_iter()
        .filter_map(|row| row.get::<_, Option<String>>(0))
        .collect())
}

/// Which trustees' steps count for `step`: all of them when the rule needs
/// no signatures, else those with a used signed step. `signing` is the
/// request being used now and its trustee, who counts once it runs.
pub async fn trustee_signatures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    step: KeyShareStep,
    signing: Option<(Uuid, String)>,
) -> anyhow::Result<TrusteeSignatures> {
    let rule = effective_rule(
        hasura_transaction,
        tenant_id,
        election_event_id,
        step.action(),
    )
    .await?;
    if !rule.is_required() {
        return Ok(TrusteeSignatures::NotNeeded);
    }
    let signed = signed_trustee_names(
        hasura_transaction,
        tenant_id,
        election_event_id,
        step,
        signing.as_ref().map(|(request_id, _)| *request_id),
    )
    .await?;
    Ok(TrusteeSignatures::Needed {
        signed,
        signing: signing.map(|(_, trustee)| trustee),
    })
}

/// [`trustee_signatures`] of a tally session, for the tally's execution;
/// ids that aren't UUIDs have no rule.
pub async fn tally_trustee_signatures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
) -> anyhow::Result<TrusteeSignatures> {
    let (Ok(tenant_id), Ok(election_event_id), Ok(tally_session_id)) = (
        Uuid::parse_str(tenant_id),
        Uuid::parse_str(election_event_id),
        Uuid::parse_str(tally_session_id),
    ) else {
        return Ok(TrusteeSignatures::NotNeeded);
    };
    trustee_signatures(
        hasura_transaction,
        tenant_id,
        election_event_id,
        KeyShareStep {
            kind: KeyShareKind::Contribute,
            id: tally_session_id,
        },
        None,
    )
    .await
}

/// The step as it always ran.
async fn run_unsigned(
    hasura_transaction: &Transaction<'_>,
    ceremony: &dyn KeyShareCeremony,
    input: &KeyShareInput<'_>,
) -> Result<KeyShareOutcome, KeyShareError> {
    if ceremony
        .run(
            hasura_transaction,
            input.key_share,
            &TrusteeSignatures::NotNeeded,
        )
        .await
        .map_err(KeyShareError::Step)?
    {
        Ok(KeyShareOutcome::Done(None))
    } else {
        Ok(KeyShareOutcome::Invalid)
    }
}

/// Takes a trustee's key step (see the module documentation), in the
/// caller's transaction. The caller commits unless the outcome is
/// [`KeyShareOutcome::Invalid`] or an error.
#[instrument(skip_all, fields(action = %input.kind.action()), err)]
pub async fn take_key_share_step(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    ceremony: &dyn KeyShareCeremony,
    input: &KeyShareInput<'_>,
) -> Result<KeyShareOutcome, KeyShareError> {
    let action = input.kind.action();
    let key_share_sha256 = sha256_hex(input.key_share.as_bytes());

    let Some(request_id) = input.signing_request_id else {
        // Without signatures the step answers as it always did: the ids
        // only matter to read the rule.
        let required = match (
            Uuid::parse_str(input.tenant_id),
            Uuid::parse_str(input.election_event_id),
        ) {
            (Ok(tenant_id), Ok(election_event_id)) => {
                effective_rule(hasura_transaction, tenant_id, election_event_id, action)
                    .await
                    .map_err(KeyShareError::Step)?
                    .is_required()
            }
            _ => false,
        };
        if !required {
            return run_unsigned(hasura_transaction, ceremony, input).await;
        }
        return start(
            hasura_transaction,
            caller,
            ceremony,
            input,
            &key_share_sha256,
        )
        .await;
    };

    let ids = parse(input)?;
    let trustee = caller_trustee(hasura_transaction, caller, ids.tenant_id).await?;
    let subject = ids
        .step
        .subject(trustee.id, &key_share_sha256)
        .map_err(internal)?;
    let request = consume_gate(
        hasura_transaction,
        caller,
        ids.tenant_id,
        ids.election_event_id,
        action,
        request_id,
        &subject,
    )
    .await?;
    // The reference is kept before the step runs: the step posts to the
    // board, so after it only the commit can fail.
    let reference = signed_step_reference(hasura_transaction, caller, &request).await?;
    ids.step
        .record(
            hasura_transaction,
            ids.tenant_id,
            ids.election_event_id,
            trustee.id,
            &reference,
        )
        .await
        .map_err(internal)?;
    let signatures = trustee_signatures(
        hasura_transaction,
        ids.tenant_id,
        ids.election_event_id,
        ids.step,
        Some((request.id, trustee.name.clone())),
    )
    .await
    .map_err(internal)?;
    if !ceremony
        .run(hasura_transaction, input.key_share, &signatures)
        .await
        .map_err(KeyShareError::Step)?
    {
        return Ok(KeyShareOutcome::Invalid);
    }
    Ok(KeyShareOutcome::Done(Some(reference)))
}

/// The rule needs the trustee's signature: checks the key share without
/// recording anything and starts the request naming its SHA-256.
async fn start(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    ceremony: &dyn KeyShareCeremony,
    input: &KeyShareInput<'_>,
    key_share_sha256: &str,
) -> Result<KeyShareOutcome, KeyShareError> {
    let ids = parse(input)?;
    if let Some(sent) = input.key_share_sha256 {
        if !sent.eq_ignore_ascii_case(key_share_sha256) {
            return Err(SigningError::invalid(
                InvalidReason::KeyShareHash,
                "The key share's SHA-256 is not the uploaded key share's.",
            )
            .into());
        }
    }
    let signatures = trustee_signatures(
        hasura_transaction,
        ids.tenant_id,
        ids.election_event_id,
        ids.step,
        None,
    )
    .await
    .map_err(internal)?;
    // A file that is not the trustee's key share starts nothing.
    if !ceremony
        .matches(hasura_transaction, input.key_share, &signatures)
        .await
        .map_err(KeyShareError::Step)?
    {
        return Ok(KeyShareOutcome::Invalid);
    }
    let trustee = caller_trustee(hasura_transaction, caller, ids.tenant_id).await?;
    let outcome = guard(
        hasura_transaction,
        caller,
        &GuardRequest {
            action: input.kind.action(),
            scope: RequestScope {
                tenant_id: ids.tenant_id,
                election_event_id: ids.election_event_id,
                election_id: None,
                area_id: None,
                trustee_id: Some(trustee.id),
                subject_key: Some(ids.step.id.to_string()),
            },
            subject: ids
                .step
                .subject(trustee.id, key_share_sha256)
                .map_err(internal)?,
            document: None,
            config_revision: None,
        },
    )
    .await?;
    match outcome {
        GuardOutcome::SigningRequired(summary) => Ok(KeyShareOutcome::SigningRequired(summary)),
        // The rule was switched off meanwhile.
        GuardOutcome::Proceed => run_unsigned(hasura_transaction, ceremony, input).await,
    }
}

/// What the ceremony keeps of a trustee's signed step: a reference to the
/// request, never the signature or the signer (a verifier reads and checks
/// those from the request and its approval).
async fn signed_step_reference(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    request: &SigningRequestRow,
) -> Result<Value, KeyShareError> {
    let approval = list_signing_approvals(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
    )
    .await
    .map_err(internal)?
    .into_iter()
    .find(|approval| approval.user_id == caller.user_id)
    .ok_or_else(|| internal(anyhow!("The used request has no signature of the trustee")))?;
    Ok(json!({
        "signing_request_id": request.id,
        "payload_sha256": request.payload_sha256,
        "code": request.code,
        "signed_at": approval.signed_at,
    }))
}

/// The names the signing panel shows beside a trustee request's ids. They
/// are labels, not signed: the dialog shows each only next to the id it
/// names, when that id is the signed one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct KeyShareLabels {
    /// The keys ceremony or tally session the names belong to.
    pub ceremony_id: Option<Uuid>,
    /// The keys ceremony's name (for a tally, the ceremony whose key share
    /// is contributed).
    pub ceremony_name: Option<String>,
    /// The name of the request's trustee.
    pub trustee_name: Option<String>,
}

/// [`KeyShareLabels`] of a trustee's request; empty for other actions.
pub async fn key_share_labels(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> anyhow::Result<KeyShareLabels> {
    let kind = match request.action {
        SigningAction::ConfirmKeyShare => KeyShareKind::Confirm,
        SigningAction::ContributeKeyShare => KeyShareKind::Contribute,
        _ => return Ok(KeyShareLabels::default()),
    };
    let ceremony_id = request
        .subject
        .get(kind.id_field())
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok());
    let ceremony_name = match ceremony_id {
        Some(id) => {
            let sql = match kind {
                KeyShareKind::Confirm => {
                    "SELECT name FROM sequent_backend.keys_ceremony
                     WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3"
                }
                KeyShareKind::Contribute => {
                    "SELECT keys_ceremony.name
                     FROM sequent_backend.tally_session
                     JOIN sequent_backend.keys_ceremony
                       ON keys_ceremony.tenant_id = tally_session.tenant_id
                      AND keys_ceremony.election_event_id = tally_session.election_event_id
                      AND keys_ceremony.id = tally_session.keys_ceremony_id
                     WHERE tally_session.tenant_id = $1
                       AND tally_session.election_event_id = $2
                       AND tally_session.id = $3"
                }
            };
            hasura_transaction
                .query_opt(sql, &[&request.tenant_id, &request.election_event_id, &id])
                .await
                .context("Error reading the ceremony's name")?
                .and_then(|row| row.get::<_, Option<String>>(0))
        }
        None => None,
    };
    let trustee_name = match request.trustee_id {
        Some(trustee_id) => {
            get_signing_trustee_name(hasura_transaction, request.tenant_id, trustee_id).await?
        }
        None => None,
    };
    Ok(KeyShareLabels {
        ceremony_id: ceremony_name.as_ref().and(ceremony_id),
        ceremony_name,
        trustee_name,
    })
}

/// Whether the caller's key step needs their signature, and whether they
/// have a used signed step for it: a trustee who took the step unsigned
/// before the rule needed signatures takes it again, signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct KeyShareSignatureStatus {
    pub signature_needed: bool,
    pub signed: bool,
}

/// [`KeyShareSignatureStatus`] of the caller for a keys ceremony or tally
/// session.
#[instrument(skip(hasura_transaction, caller), err)]
pub async fn key_share_signature_status(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    kind: KeyShareKind,
    target_id: &str,
) -> Result<KeyShareSignatureStatus, KeyShareError> {
    let ids = parse(&KeyShareInput {
        tenant_id,
        election_event_id,
        kind,
        target_id,
        key_share: "",
        key_share_sha256: None,
        signing_request_id: None,
    })?;
    let signatures = trustee_signatures(
        hasura_transaction,
        ids.tenant_id,
        ids.election_event_id,
        ids.step,
        None,
    )
    .await
    .map_err(internal)?;
    Ok(match signatures {
        TrusteeSignatures::NotNeeded => KeyShareSignatureStatus {
            signature_needed: false,
            signed: false,
        },
        TrusteeSignatures::Needed { signed, .. } => KeyShareSignatureStatus {
            signature_needed: true,
            signed: caller
                .trustee
                .as_deref()
                .is_some_and(|trustee| signed.contains(trustee)),
        },
    })
}
