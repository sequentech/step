// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Unsigned electoral-log statements, queued by trusted internal producers
//! whose entry is best-effort while the board's signer is unavailable.
//!
//! A producer whose entry is a record made after its action (not a
//! condition of it) and that already treated a failure to record as
//! non-fatal may, when the board's signer is unavailable
//! ([`electoral_log::signer::SignerUnavailable`]) or its log has entries
//! waiting ([`crate::services::electoral_log_deferral::ScopeWaiting`]),
//! queue a [`PendingStatement`] instead of losing the entry. It is plainly
//! unsigned data: no signature, no placeholder.
//!
//! **Where it lives.** In the waiting table (`step_queue.electoral_log_waiting`,
//! `pgmq-broker`'s `setup.sql`), which only the workers' database role may
//! write: Keycloak and Harvest (the producer role) can't add or change a
//! row. The event queue only carries a reference to it, `{"pending_statement_ref":
//! "<delivery ID>"}` in an `Internal` event (no new message type: a worker of
//! an earlier release dead-letters it as a malformed record, replayable,
//! never stores it). So writing to the queues, as Keycloak can, gives no way
//! to have a board sign a statement: a forged reference names no row, or a
//! row a worker wrote. No new secret is involved: the boundary is the
//! existing database roles. A producer running as the producer role
//! (Harvest) can't queue one; it fails as before.
//!
//! **What the batch task checks** before signing ([`PendingStatement::read`]):
//! the row exists under the event's delivery ID and scope; it is this type
//! (versioned, unknown fields refused); its kind is allowed and comes from
//! its own origin; it is bound to
//! the event's tenant and election event, under a delivery ID of its origin;
//! its time is not in the future and not long before the row was written
//! (the row's own `created_at`, which the producer does not choose); its
//! identifiers are short and printable. It then builds the statement from the
//! typed fields with the producer's own builder, at that time, signs it with
//! the board's system signer only (never an administrator's or a voter's
//! key), and stores it under the queued delivery ID; while the signer is
//! still unavailable it waits durably like any event
//! (`services::electoral_log_deferral`). Anything else is poison: dead-lettered,
//! never signed.
//!
//! Producers whose rule is that a failure to record stops the action never
//! queue: they keep failing visibly.
//!
//! **Administrator-signed entries** (certificate-authority imports and
//! deletions). The administrator's proof is made at the time of the action,
//! with their own vault key (the key a direct post signs with), over the exact
//! statement, and carried unchanged; only the board's attestation waits. The
//! batch task never looks an administrator's key up: it checks the signature
//! with the key the row names, and that this key is the one the tenant's log
//! announces for that administrator (an `AdminPublicKey` entry signed by the
//! tenant log's system key, written when the vault made the key); otherwise
//! the entry is refused and never attested. The action queues its entry before
//! its transaction commits (fail closed: no entry, no commit), due only after
//! a delay so it is not attested before that commit, and withdraws it if the
//! commit fails.
//!
//! **The producer role** (Harvest; Keycloak shares it) can't write the table.
//! It may only call `step_queue.queue_pending_statement`, which accepts just the
//! kinds that role is allowed (a ballot publication's failure), checks their
//! form, origin, delivery ID and time, and writes the row as its owner. Queue
//! write access thus gains no signing authority beyond that allow-list.

use crate::tasks::electoral_log::{LogEventBody, LogEventInput, LogMessageType};
use anyhow::{anyhow, ensure, Context, Result};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::CertificateAuthEventAction;
use voting_crypto::signing::{Signature, VerificationKey};
use electoral_log::messages::newtypes::{
    BallotPublicationFailure, CastVoteErrorString, ElectionIdString, ErrorMessageString,
    EventIdString, KeycloakEventTypeString, PseudonymHash,
    VoterCountryString, VoterIpString,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The only version of [`PendingStatement`].
pub const PENDING_STATEMENT_VERSION: u32 = 1;
/// How far in the future a statement's time may be (clock skew between
/// workers).
pub const MAX_FUTURE_SECS: u64 = 300;
/// The longest text field a pending statement may carry.
pub const MAX_FIELD_CHARS: usize = 16 * 1024;
/// The prefix of a pending statement's delivery ID: `pending:<origin>:<uuid>`.
pub const DELIVERY_PREFIX: &str = "pending:";
/// The longest user ID, username, area or election ID a pending statement may carry.
pub const MAX_ID_CHARS: usize = 256;
/// How long before it was first queued a statement may say it happened.
pub const MAX_LAG_SECS: u64 = 600;
/// The only key of an `Internal` body that refers to a pending statement.
pub const PENDING_REF_KEY: &str = "pending_statement_ref";

/// The body of the `Internal` event that refers to a pending statement.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingRef {
    pending_statement_ref: String,
}

/// Whether an `Internal` event's body refers to a pending statement (rather
/// than carrying a signed record), and which.
pub fn pending_ref(body: &str) -> Option<String> {
    let is_ref = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.as_object().map(|object| object.contains_key(PENDING_REF_KEY)))
        .unwrap_or(false);
    // A body with the key that isn't exactly a reference refers to nothing.
    is_ref.then(|| {
        serde_json::from_str::<PendingRef>(body)
            .map(|reference| reference.pending_statement_ref)
            .unwrap_or_default()
    })
}

/// Withdraws a pending statement queued for an action that was then rolled
/// back, before it is signed: its reference then names no row and is
/// dead-lettered. Returns whether it was still waiting.
pub async fn cancel(delivery_id: &str) -> Result<bool> {
    use crate::services::electoral_log_deferral::WAITING_TABLE;
    let client = crate::services::database::get_queue_pool()
        .await
        .get()
        .await
        .context("Error connecting to the task-queue database")?;
    Ok(client
        .execute(
            &format!("DELETE FROM {WAITING_TABLE} WHERE delivery_id = $1 AND statement IS NOT NULL"),
            &[&delivery_id],
        )
        .await
        .context("Error withdrawing a pending statement")?
        > 0)
}

/// A pending statement as a worker wrote it in the waiting table.
#[derive(Clone, Debug)]
pub struct StoredPending {
    pub statement: serde_json::Value,
    pub scope: String,
    /// When the row was written, in milliseconds since the epoch.
    pub created_ms: i64,
}

/// The pending statement of `delivery_id`, if a worker wrote one.
pub async fn load<C: tokio_postgres::GenericClient + Sync>(
    client: &C,
    delivery_id: &str,
) -> Result<Option<StoredPending>> {
    use crate::services::electoral_log_deferral::WAITING_TABLE;
    Ok(client
        .query_opt(
            &format!(
                "SELECT statement, scope, (extract(epoch FROM created_at) * 1000)::bigint \
                 FROM {WAITING_TABLE} WHERE delivery_id = $1 AND statement IS NOT NULL"
            ),
            &[&delivery_id],
        )
        .await
        .context("Error reading a pending statement")?
        .map(|row| StoredPending {
            statement: row.get(0),
            scope: row.get(1),
            created_ms: row.get(2),
        }))
}

/// Which trusted producer queued a statement. Each may queue one kind.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PendingOrigin {
    /// A refused ballot's audit (`insert_cast_vote`, `post_cast_vote_error`).
    CastVoteRefusal,
    /// The enrollment-windows problems (`enrollment_windows`).
    EnrollmentWindows,
    /// A ballot publication's failure (`log_ballot_publication_failure`).
    BallotPublication,
    /// A certificate-authority import or deletion (`certificate_authority`),
    /// signed by its administrator at the time of the action.
    CertificateAuthority,
}

impl PendingOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            PendingOrigin::CastVoteRefusal => "cast-vote-refusal",
            PendingOrigin::EnrollmentWindows => "enrollment-windows",
            PendingOrigin::BallotPublication => "ballot-publication",
            PendingOrigin::CertificateAuthority => "certificate-authority",
        }
    }

    /// A new delivery ID of this origin.
    pub fn delivery_id(&self) -> String {
        format!("{DELIVERY_PREFIX}{}:{}", self.as_str(), Uuid::new_v4())
    }
}

/// The statement to make, as typed fields; never statement bytes.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PendingStatementBody {
    /// `Message::cast_vote_error_message`.
    /// No IP or country is kept (refused ballots record neither).
    CastVoteError {
        election_id: Option<String>,
        pseudonym: PseudonymHash,
        error: String,
        voter_id: String,
        area_id: String,
    },
    /// `Message::keycloak_user_event` of type [`ENROLLMENT_PROBLEM_EVENT_TYPE`].
    EnrollmentProblems { problems: String },
    /// `Message::ballot_publication_failure_message`.
    BallotPublicationFailure {
        failure: BallotPublicationFailure,
        username: Option<String>,
    },
    /// `Message::certificate_auth_event_message`, already signed by its
    /// administrator: the statement is rebuilt from these fields at the
    /// statement's time, the administrator's signature over it is checked
    /// with `admin_public_key` (which the batch task also checks is announced
    /// for `user_id` in the tenant's log), and only the board's signature is
    /// added. Never re-signed with another key.
    AdminCertificateEvent {
        action: CertificateAuthEventAction,
        subject_dns: Vec<String>,
        user_id: String,
        username: Option<String>,
        /// The administrator's public key, as their tenant log announces it.
        admin_public_key: String,
        /// The administrator's signature over the statement.
        admin_signature: String,
    },
}

impl PendingStatementBody {
    /// The origin allowed to queue this kind.
    pub fn origin(&self) -> PendingOrigin {
        match self {
            PendingStatementBody::CastVoteError { .. } => PendingOrigin::CastVoteRefusal,
            PendingStatementBody::EnrollmentProblems { .. } => PendingOrigin::EnrollmentWindows,
            PendingStatementBody::BallotPublicationFailure { .. } => {
                PendingOrigin::BallotPublication
            }
            PendingStatementBody::AdminCertificateEvent { .. } => {
                PendingOrigin::CertificateAuthority
            }
        }
    }

    /// For a statement its administrator signed: who, and with which key.
    pub fn admin(&self) -> Option<(&str, &str)> {
        match self {
            PendingStatementBody::AdminCertificateEvent {
                user_id,
                admin_public_key,
                ..
            } => Some((user_id.as_str(), admin_public_key.as_str())),
            _ => None,
        }
    }

    /// The message, made with `sd` for election event `event_id`: the same
    /// builder, with the same fields, as the producer's own post.
    pub fn message(&self, event_id: &str, sd: &SigningData) -> Result<Message> {
        let event = EventIdString(event_id.to_string());
        match self {
            PendingStatementBody::CastVoteError {
                election_id,
                pseudonym,
                error,
                voter_id,
                area_id,
            } => Message::cast_vote_error_message(
                event,
                ElectionIdString(election_id.clone()),
                pseudonym.clone(),
                CastVoteErrorString(error.clone()),
                sd,
                VoterIpString(String::new()),
                VoterCountryString(String::new()),
                Some(voter_id.clone()),
                area_id.clone(),
            ),
            PendingStatementBody::EnrollmentProblems { problems } => Message::keycloak_user_event(
                event,
                KeycloakEventTypeString(ENROLLMENT_PROBLEM_EVENT_TYPE.to_string()),
                ErrorMessageString(crate::services::privacy_signin::safe_error_text(problems)),
                None,
                None,
                sd,
                None,
            ),
            PendingStatementBody::BallotPublicationFailure { failure, username } => {
                Message::ballot_publication_failure_message(
                    event,
                    failure.clone(),
                    sd,
                    username.clone(),
                )
            }
            PendingStatementBody::AdminCertificateEvent {
                action,
                subject_dns,
                user_id,
                username,
                admin_public_key,
                admin_signature,
            } => {
                let statement =
                    Message::certificate_auth_event_statement(event, action.clone(), subject_dns.clone());
                let admin_pk = VerificationKey::from_b64_string(admin_public_key)
                    .context("The administrator's public key is unreadable")?;
                let signature = Signature::from_b64_string(admin_signature)
                    .context("The administrator's signature is unreadable")?;
                // The administrator's own signature, checked, then the board's.
                sd.attest(
                    statement,
                    "",
                    admin_pk,
                    signature,
                    Some(user_id.clone()),
                    username.clone(),
                )
            }
        }
    }

    /// The identifier fields (user IDs, usernames, areas, elections,
    /// publications, tasks), to bound their size and form.
    fn identifiers(&self) -> Vec<&str> {
        let optional = |value: &Option<String>| value.as_deref().unwrap_or_default();
        match self {
            PendingStatementBody::CastVoteError {
                election_id,
                voter_id,
                area_id,
                ..
            } => vec![optional(election_id), voter_id, area_id],
            PendingStatementBody::EnrollmentProblems { .. } => vec![],
            PendingStatementBody::BallotPublicationFailure { failure, username } => vec![
                failure.publication_id.0.as_str(),
                failure.task_id.as_str(),
                optional(username),
            ],
            PendingStatementBody::AdminCertificateEvent {
                user_id,
                username,
                admin_public_key,
                admin_signature,
                ..
            } => vec![user_id, optional(username), admin_public_key, admin_signature],
        }
    }

    /// The free-text fields, to bound their size.
    fn texts(&self) -> Vec<&str> {
        match self {
            PendingStatementBody::CastVoteError { error, .. } => vec![error],
            PendingStatementBody::EnrollmentProblems { problems } => vec![problems],
            PendingStatementBody::BallotPublicationFailure { failure, .. } => {
                vec![failure.error.0.as_str()]
            }
            PendingStatementBody::AdminCertificateEvent { subject_dns, .. } => {
                subject_dns.iter().map(String::as_str).collect()
            }
        }
    }
}

/// An unsigned statement queued by a trusted producer, bound to where and
/// when it belongs. See the module documentation.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PendingStatement {
    /// [`PENDING_STATEMENT_VERSION`].
    pub version: u32,
    pub origin: PendingOrigin,
    pub tenant_id: String,
    pub election_event_id: String,
    /// When it happened, in seconds since the Unix epoch: the statement's time.
    pub occurred_at: u64,
    pub statement: PendingStatementBody,
}

impl PendingStatement {
    /// A statement of `statement`'s own origin, happening now.
    pub fn new(tenant_id: &str, election_event_id: &str, statement: PendingStatementBody) -> Self {
        PendingStatement {
            version: PENDING_STATEMENT_VERSION,
            origin: statement.origin(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            occurred_at: electoral_log::timestamp(),
            statement,
        }
    }

    /// The scope its entry belongs to.
    pub fn scope(&self) -> String {
        crate::services::electoral_log_deferral::scope_key(&self.tenant_id, &self.election_event_id)
    }

    /// The event-queue event that refers to it under `delivery_id`.
    pub fn reference_event(&self, delivery_id: &str) -> Result<LogEventInput> {
        Ok(LogEventInput {
            delivery_id: Some(delivery_id.to_string()),
            election_event_id: self.election_event_id.clone(),
            message_type: LogMessageType::Internal,
            user_id: None,
            username: None,
            tenant_id: self.tenant_id.clone(),
            body: LogEventBody::Plain(
                serde_json::to_string(&PendingRef {
                    pending_statement_ref: delivery_id.to_string(),
                })
                .context("Error serializing a pending statement reference")?,
            ),
            event_time_ms: None,
            details: None,
            admin_event: None,
            deferral: None,
        })
    }

    /// Writes it in the waiting table and its reference on the event queue, in
    /// one transaction of the queue database, under its scope's lock (so a
    /// direct post of the scope can't come between: `direct_post_gate`). Its
    /// log's direct posts are refused until it is stored. Returns its
    /// delivery ID.
    pub async fn queue(&self) -> Result<String> {
        self.queue_after(0).await
    }

    /// As [`PendingStatement::queue`], due only `delay_secs` later: a
    /// statement queued before its action's own transaction commits is not
    /// signed before that commit, nor after its rollback ([`cancel`]).
    ///
    /// A worker writes the waiting table itself. A process of the producer
    /// role (Harvest), which may not write it, calls the one function it may:
    /// `step_queue.queue_pending_statement`, which accepts only the kinds the
    /// producer role is allowed and checks them (`pgmq-broker`'s `setup.sql`).
    pub async fn queue_after(&self, delay_secs: i32) -> Result<String> {
        use crate::services::electoral_log_deferral::{lock_scopes, WAITING_TABLE};
        let delivery_id = self.origin.delivery_id();
        let reference = crate::services::electoral_log_dead_letter::event_message(
            &self.reference_event(&delivery_id)?,
        )?;
        let statement = serde_json::to_value(self).context("Error serializing a pending statement")?;
        let queue = crate::services::celery_app::Queue::ElectoralLogEvent.queue_name();
        let mut client = crate::services::database::get_queue_pool()
            .await
            .get()
            .await
            .context("Error connecting to the task-queue database")?;
        let tx = client
            .transaction()
            .await
            .context("Error starting the pending statement's transaction")?;
        lock_scopes(&*tx, &[self.scope()]).await?;
        let written = tx
            .execute(
                &format!(
                    "INSERT INTO {WAITING_TABLE} (delivery_id, scope, since_ms, ord, due_at, statement) \
                     VALUES ($1, $2, (extract(epoch FROM clock_timestamp()) * 1000)::bigint, 0, \
                     clock_timestamp() + make_interval(secs => $4::integer), $3)"
                ),
                &[&delivery_id, &self.scope(), &statement, &delay_secs],
            )
            .await;
        match written {
            Ok(_) => {
                tx.query_one(
                    "SELECT pgmq.send($1::text, $2::jsonb, $3::integer)",
                    &[&queue, &reference, &delay_secs],
                )
                .await
                .context("Error queueing a pending statement's reference")?;
                tx.commit()
                    .await
                    .context("Error committing a pending statement")?;
            }
            Err(error)
                if error.code() == Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE) =>
            {
                // The producer role: through its one allowed function.
                drop(tx);
                client
                    .query_one(
                        "SELECT step_queue.queue_pending_statement($1, $2, $3, $4)",
                        &[&delivery_id, &statement, &reference, &delay_secs],
                    )
                    .await
                    .context("Error queueing a pending statement through the producer function")?;
            }
            Err(error) => {
                return Err(anyhow::Error::new(error).context("Error writing a pending statement"))
            }
        }
        tracing::warn!(
            origin = self.origin.as_str(),
            delivery_id = delivery_id.as_str(),
            "The electoral-log entry is queued unsigned, to be signed and stored later"
        );
        Ok(delivery_id)
    }

    /// Reads and checks the pending statement a reference event names (see
    /// the module documentation), before anything is signed.
    pub fn read(
        input: &LogEventInput,
        stored: Option<StoredPending>,
        now: u64,
    ) -> Result<PendingStatement> {
        ensure!(
            input.message_type == LogMessageType::Internal,
            "Not an internal event"
        );
        let delivery_id = input.delivery_id.as_deref().unwrap_or_default();
        ensure!(
            pending_ref(&input.body.as_raw()).as_deref() == Some(delivery_id),
            "The reference names another delivery than its event's"
        );
        let stored = stored.ok_or_else(|| anyhow!("No worker queued a pending statement {delivery_id:?}"))?;
        let pending: PendingStatement =
            serde_json::from_value(stored.statement).context("Not a pending statement")?;
        ensure!(
            stored.scope == pending.scope(),
            "The pending statement is not of its row's scope"
        );
        pending.validate(input, stored.created_ms, now)?;
        Ok(pending)
    }

    fn validate(&self, input: &LogEventInput, created_ms: i64, now: u64) -> Result<()> {
        ensure!(
            self.version == PENDING_STATEMENT_VERSION,
            "Unknown pending statement version {}",
            self.version
        );
        ensure!(
            self.statement.origin() == self.origin,
            "A {} statement may not be queued by {}",
            statement_kind(&self.statement),
            self.origin.as_str()
        );
        Uuid::parse_str(&self.tenant_id).context("Invalid pending statement tenant")?;
        Uuid::parse_str(&self.election_event_id)
            .context("Invalid pending statement election event")?;
        ensure!(
            self.tenant_id == input.tenant_id && self.election_event_id == input.election_event_id,
            "The pending statement is bound to another tenant or election event than its event"
        );
        let delivery_id = input.delivery_id.as_deref().unwrap_or_default();
        let uuid = delivery_id
            .strip_prefix(DELIVERY_PREFIX)
            .and_then(|rest| rest.strip_prefix(self.origin.as_str()))
            .and_then(|rest| rest.strip_prefix(':'))
            .ok_or_else(|| anyhow!("The pending statement's delivery ID is not of its origin"))?;
        Uuid::parse_str(uuid).context("The pending statement's delivery ID is not of its origin")?;
        ensure!(self.occurred_at > 0, "The pending statement has no time");
        ensure!(
            self.occurred_at <= now.saturating_add(MAX_FUTURE_SECS),
            "The pending statement's time is in the future"
        );
        let written = u64::try_from(created_ms / 1000).unwrap_or(0);
        ensure!(
            self.occurred_at.saturating_add(MAX_LAG_SECS) >= written
                && self.occurred_at <= written.saturating_add(MAX_FUTURE_SECS),
            "The pending statement's time is not when it was written"
        );
        for identifier in self.statement.identifiers() {
            ensure!(
                identifier.chars().count() <= MAX_ID_CHARS
                    && !identifier.chars().any(char::is_control),
                "A pending statement identifier is too long or not printable"
            );
        }
        for text in self.statement.texts() {
            ensure!(
                text.chars().count() <= MAX_FIELD_CHARS,
                "A pending statement field is too long"
            );
        }
        Ok(())
    }

    /// The statement, signed with `sd` (the board's system signer) at the
    /// time it happened.
    pub fn sign(&self, sd: &SigningData) -> Result<Message> {
        electoral_log::with_timestamp(self.occurred_at, || {
            self.statement.message(&self.election_event_id, sd)
        })
    }
}

fn statement_kind(statement: &PendingStatementBody) -> &'static str {
    match statement {
        PendingStatementBody::CastVoteError { .. } => "cast-vote-error",
        PendingStatementBody::EnrollmentProblems { .. } => "enrollment-problems",
        PendingStatementBody::BallotPublicationFailure { .. } => "ballot-publication-failure",
        PendingStatementBody::AdminCertificateEvent { .. } => "admin-certificate-event",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::messages::newtypes::{BallotPublicationIdString, BallotPublicationStage};
    use voting_crypto::signing::{SigningKey, VerificationKey};

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT: &str = "fdd21db2-dd68-4974-90eb-7f2750b2b5df";
    const OTHER: &str = "6f0c5c2e-3f43-4a5e-9d55-0d6a3e1f7b21";

    fn failure() -> PendingStatementBody {
        PendingStatementBody::BallotPublicationFailure {
            failure: BallotPublicationFailure {
                publication_id: BallotPublicationIdString("publication".into()),
                task_id: "task".into(),
                stage: BallotPublicationStage::Publish,
                error: ErrorMessageString("refused".into()),
            },
            username: Some("admin".into()),
        }
    }

    /// As a worker writes it: the row and its reference event.
    fn queued(pending: &PendingStatement) -> (LogEventInput, StoredPending) {
        let delivery_id = pending.origin.delivery_id();
        (
            pending.reference_event(&delivery_id).unwrap(),
            StoredPending {
                statement: serde_json::to_value(pending).unwrap(),
                scope: pending.scope(),
                created_ms: i64::try_from(electoral_log::timestamp()).unwrap() * 1000,
            },
        )
    }

    fn reread(input: &LogEventInput, stored: &StoredPending) -> Result<PendingStatement> {
        PendingStatement::read(input, Some(stored.clone()), electoral_log::timestamp())
    }

    #[test]
    fn a_trusted_statement_is_read_back_and_signed_at_its_time() {
        let mut pending = PendingStatement::new(TENANT, EVENT, failure());
        let at = electoral_log::timestamp() - 30;
        pending.occurred_at = at;
        let (input, stored) = queued(&pending);
        assert!(input
            .delivery_id
            .as_deref()
            .unwrap()
            .starts_with("pending:ballot-publication:"));
        // The event carries a reference only, never the statement.
        assert!(!input.body.as_raw().contains("publication"));
        let read = reread(&input, &stored).unwrap();
        let key = SigningKey::generate().unwrap();
        let sd = SigningData::new(key.clone(), "", key.clone());
        let message = read.sign(&sd).unwrap();
        message
            .verify(&VerificationKey::from_sk(&key).unwrap())
            .unwrap();
        assert_eq!(message.statement.head.timestamp, at);
        assert_eq!(message.statement.head.event.0, EVENT);
        // Signed again later (it waited), the statement is the same bytes.
        use voting_crypto::RecordSerialize;
        let again = read.sign(&sd).unwrap();
        assert_eq!(
            again.statement.record_serialize().unwrap(),
            message.statement.record_serialize().unwrap()
        );
    }

    #[test]
    fn a_forged_foreign_or_disallowed_statement_is_refused() {
        let good = PendingStatement::new(TENANT, EVENT, failure());
        let (event, stored) = queued(&good);
        assert!(reread(&event, &stored).is_ok());

        // No row: a reference forged on the queue names nothing a worker wrote.
        assert!(PendingStatement::read(&event, None, electoral_log::timestamp()).is_err());

        // A reference to another delivery than its event's.
        let mut other_ref = event.clone();
        other_ref.delivery_id = Some(PendingOrigin::BallotPublication.delivery_id());
        assert!(reread(&other_ref, &stored).is_err());

        // Carried by an event of another election event or tenant.
        let mut other_event = event.clone();
        other_event.election_event_id = OTHER.into();
        assert!(reread(&other_event, &stored).is_err());
        let mut other_tenant = event.clone();
        other_tenant.tenant_id = OTHER.into();
        assert!(reread(&other_tenant, &stored).is_err());

        // A row of another scope.
        let mut foreign_row = stored.clone();
        foreign_row.scope = format!("{TENANT}/{OTHER}");
        assert!(reread(&event, &foreign_row).is_err());

        // A kind its origin may not queue.
        let mut wrong_origin = good.clone();
        wrong_origin.origin = PendingOrigin::CastVoteRefusal;
        let mut row = stored.clone();
        row.statement = serde_json::to_value(&wrong_origin).unwrap();
        assert!(reread(&event, &row).is_err());

        // An unknown origin, kind or version, or an extra field: not this type.
        let mut value = serde_json::to_value(&good).unwrap();
        value["origin"] = "anyone".into();
        row.statement = value;
        assert!(reread(&event, &row).is_err());
        for kind in [
            "external-api-request",
            "election-published",
            "cast-vote",
            "readiness-verified",
            "checkpoint",
        ] {
            let mut value = serde_json::to_value(&good).unwrap();
            value["statement"]["kind"] = kind.into();
            row.statement = value;
            assert!(reread(&event, &row).is_err(), "{kind}");
        }
        let mut value = serde_json::to_value(&good).unwrap();
        value["statement_bytes"] = "AAAA".into();
        row.statement = value;
        assert!(reread(&event, &row).is_err());
        let mut value = serde_json::to_value(&good).unwrap();
        value["version"] = 2.into();
        row.statement = value;
        assert!(reread(&event, &row).is_err());

        // Another producer's delivery ID.
        let mut squatting = event.clone();
        squatting.delivery_id = Some("readiness:entry".into());
        squatting.body = LogEventBody::Plain(r#"{"pending_statement_ref":"readiness:entry"}"#.into());
        assert!(reread(&squatting, &stored).is_err());

        // A time in the future, or long before the row was written.
        let mut future = good.clone();
        future.occurred_at = electoral_log::timestamp() + 3_600;
        let mut row = stored.clone();
        row.statement = serde_json::to_value(&future).unwrap();
        assert!(reread(&event, &row).is_err());
        let mut stale = good.clone();
        stale.occurred_at = electoral_log::timestamp() - MAX_LAG_SECS - 60;
        row.statement = serde_json::to_value(&stale).unwrap();
        assert!(reread(&event, &row).is_err());

        // Not an internal event.
        let mut keycloak = event.clone();
        keycloak.message_type = LogMessageType::KeycloakEvent("LOGIN".into());
        assert!(reread(&keycloak, &stored).is_err());

        // An identifier that isn't printable, a field too long.
        let control = PendingStatement::new(
            TENANT,
            EVENT,
            PendingStatementBody::CastVoteError {
                election_id: None,
                pseudonym: PseudonymHash(voting_crypto::hash::HashWrapper::new(
                    crate::services::insert_cast_vote::hash_voter_id("voter").unwrap(),
                )),
                error: "refused".into(),
                voter_id: "voter\n".into(),
                area_id: "area".into(),
            },
        );
        let (event, row) = queued(&control);
        assert!(reread(&event, &row).is_err());
        let huge = PendingStatement::new(
            TENANT,
            EVENT,
            PendingStatementBody::EnrollmentProblems {
                problems: "x".repeat(MAX_FIELD_CHARS + 1),
            },
        );
        let (event, row) = queued(&huge);
        assert!(reread(&event, &row).is_err());

        // A signed record's body is no reference.
        assert_eq!(pending_ref(r#"{"id":0,"message":[1]}"#), None);
    }

    fn certificate_event(admin: &SigningKey) -> PendingStatement {
        crate::services::electoral_log::admin_signed_certificate_event(
            admin,
            TENANT,
            EVENT,
            CertificateAuthEventAction::Import,
            vec!["CN=one".into()],
            "admin-id",
            Some("admin".into()),
        )
        .unwrap()
    }

    #[test]
    fn an_admin_signed_certificate_event_keeps_the_admin_signature_and_adds_only_the_boards() {
        let admin = SigningKey::generate().unwrap();
        let board = SigningKey::generate().unwrap();
        let pending = certificate_event(&admin);
        let (event, stored) = queued(&pending);
        let read = reread(&event, &stored).unwrap();
        assert_eq!(
            read.statement.admin(),
            Some((
                "admin-id",
                VerificationKey::from_sk(&admin).unwrap().to_b64_string().unwrap().as_str()
            ))
        );
        // The board's signer (the system key here) attests; the sender is the admin.
        let sd = SigningData::new(board.clone(), "", board.clone());
        let message = read.sign(&sd).unwrap();
        message
            .verify(&VerificationKey::from_sk(&board).unwrap())
            .unwrap();
        assert_eq!(message.sender.pk, VerificationKey::from_sk(&admin).unwrap());
        assert_eq!(message.statement.head.timestamp, pending.occurred_at);
        assert_eq!(message.user_id.as_deref(), Some("admin-id"));
    }

    #[test]
    fn a_tampered_or_foreign_admin_signed_event_is_never_attested() {
        let admin = SigningKey::generate().unwrap();
        let other = SigningKey::generate().unwrap();
        let board = SigningKey::generate().unwrap();
        let sd = SigningData::new(board.clone(), "", board);
        let pending = certificate_event(&admin);

        // Tampered: another subject than the one the admin signed.
        let mut tampered = pending.clone();
        if let PendingStatementBody::AdminCertificateEvent { subject_dns, .. } =
            &mut tampered.statement
        {
            *subject_dns = vec!["CN=two".into()];
        }
        assert!(tampered.sign(&sd).is_err());
        // Another time than the one signed.
        let mut moved = pending.clone();
        moved.occurred_at -= 1;
        assert!(moved.sign(&sd).is_err());
        // Another key named as the admin's.
        let mut foreign = pending.clone();
        if let PendingStatementBody::AdminCertificateEvent {
            admin_public_key, ..
        } = &mut foreign.statement
        {
            *admin_public_key = VerificationKey::from_sk(&other).unwrap().to_b64_string().unwrap();
        }
        assert!(foreign.sign(&sd).is_err());
        // Its own origin only.
        let mut wrong_origin = pending.clone();
        wrong_origin.origin = PendingOrigin::BallotPublication;
        let (event, mut row) = queued(&pending);
        row.statement = serde_json::to_value(&wrong_origin).unwrap();
        assert!(reread(&event, &row).is_err());
    }
}
