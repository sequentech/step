// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Unsigned electoral-log statements, queued by trusted internal producers
//! whose entry is best-effort while the board's signer is unavailable.
//!
//! A producer whose entry is a record made after its action (not a
//! condition of it) and that already treated a failure to record as
//! non-fatal may, when the board's signer is unavailable
//! ([`electoral_log::signer::SignerUnavailable`]), queue a
//! [`PendingStatement`] instead of losing the entry. It is plainly unsigned
//! data: no signature, no placeholder. The batch task (`tasks::electoral_log`)
//! validates it ([`PendingStatement::validate`]), builds the statement from
//! its typed fields with the same builder the producer uses, at the time it
//! happened, signs it with the board's system signer, and stores it under the
//! queued delivery ID; while the signer is still unavailable it waits
//! durably like any event (`services::electoral_log_deferral`).
//!
//! Producers whose rule is that a failure to record stops the action never
//! queue: they keep failing visibly.
//!
//! It travels as an `Internal` event whose body is `{"pending_statement": ..}`
//! (no new message type): a worker of an earlier release reads that body as
//! a malformed signed record and dead-letters it (replayable once workers are
//! updated); it never stores it.
//!
//! What a pending statement can make the board sign is bounded: one of the
//! kinds below, each only from its own origin, for the tenant and election
//! event the queued event names, under a delivery ID of that origin, made
//! from typed fields (never statement bytes), signed by the board's system
//! signer only (never an administrator's or a voter's key), with a time
//! that is not in the future. Anything else is refused and dead-lettered,
//! never signed.

use crate::tasks::electoral_log::{LogEventBody, LogEventInput, LogMessageType};
use anyhow::{anyhow, ensure, Context, Result};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{
    BallotPublicationFailure, CastVoteErrorString, ElectionIdString, ErrorMessageString,
    EventIdString, ExtApiName, ExtApiRequestDirection, KeycloakEventTypeString, PseudonymHash,
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
/// The only key of an `Internal` body that carries a pending statement.
pub const PENDING_BODY_KEY: &str = "pending_statement";

/// Whether an `Internal` event's body carries a pending statement (rather
/// than a signed record).
pub fn is_pending_body(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.as_object().map(|object| object.contains_key(PENDING_BODY_KEY)))
        .unwrap_or(false)
}

/// The body of the `Internal` event that carries a pending statement.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingBody {
    pending_statement: PendingStatement,
}
/// The event type of an enrollment-windows problem entry
/// (`services::enrollment_windows`).
pub const ENROLLMENT_PROBLEM_EVENT_TYPE: &str =
    crate::services::enrollment_windows::PROBLEM_LOG_EVENT_TYPE;

/// Which trusted producer queued a statement. Each may queue one kind.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PendingOrigin {
    /// A refused ballot's audit (`insert_cast_vote`, `post_cast_vote_error`).
    CastVoteRefusal,
    /// A Datafix request's result (`external::utils`).
    ExternalApi,
    /// The enrollment-windows problems (`enrollment_windows`).
    EnrollmentWindows,
    /// A ballot publication's failure (`log_ballot_publication_failure`).
    BallotPublication,
}

impl PendingOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            PendingOrigin::CastVoteRefusal => "cast-vote-refusal",
            PendingOrigin::ExternalApi => "external-api",
            PendingOrigin::EnrollmentWindows => "enrollment-windows",
            PendingOrigin::BallotPublication => "ballot-publication",
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
    /// `Message::external_api_request_message`, for Datafix only.
    ExternalApiRequest {
        election_id: Option<String>,
        voter_id: Option<String>,
        voter_username: Option<String>,
        direction: ExtApiRequestDirection,
        api_name: ExtApiName,
        operation: String,
    },
    /// `Message::keycloak_user_event` of type [`ENROLLMENT_PROBLEM_EVENT_TYPE`].
    EnrollmentProblems { problems: String },
    /// `Message::ballot_publication_failure_message`.
    BallotPublicationFailure {
        failure: BallotPublicationFailure,
        username: Option<String>,
    },
}

impl PendingStatementBody {
    /// The origin allowed to queue this kind.
    pub fn origin(&self) -> PendingOrigin {
        match self {
            PendingStatementBody::CastVoteError { .. } => PendingOrigin::CastVoteRefusal,
            PendingStatementBody::ExternalApiRequest { .. } => PendingOrigin::ExternalApi,
            PendingStatementBody::EnrollmentProblems { .. } => PendingOrigin::EnrollmentWindows,
            PendingStatementBody::BallotPublicationFailure { .. } => {
                PendingOrigin::BallotPublication
            }
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
            PendingStatementBody::ExternalApiRequest {
                election_id,
                voter_id,
                voter_username,
                direction,
                api_name,
                operation,
            } => Message::external_api_request_message(
                event,
                ElectionIdString(election_id.clone()),
                sd,
                voter_id.clone(),
                voter_username.clone(),
                direction.clone(),
                api_name.clone(),
                operation.clone(),
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
            PendingStatementBody::ExternalApiRequest {
                election_id,
                voter_id,
                voter_username,
                ..
            } => vec![
                optional(election_id),
                optional(voter_id),
                optional(voter_username),
            ],
            PendingStatementBody::EnrollmentProblems { .. } => vec![],
            PendingStatementBody::BallotPublicationFailure { failure, username } => vec![
                failure.publication_id.0.as_str(),
                failure.task_id.as_str(),
                optional(username),
            ],
        }
    }

    /// The free-text fields, to bound their size.
    fn texts(&self) -> Vec<&str> {
        match self {
            PendingStatementBody::CastVoteError { error, .. } => vec![error],
            PendingStatementBody::ExternalApiRequest { operation, .. } => vec![operation],
            PendingStatementBody::EnrollmentProblems { problems } => vec![problems],
            PendingStatementBody::BallotPublicationFailure { failure, .. } => {
                vec![failure.error.0.as_str()]
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

    /// The event-queue event that carries it, under a new delivery ID of its
    /// origin.
    pub fn event(&self) -> Result<LogEventInput> {
        Ok(LogEventInput {
            delivery_id: Some(self.origin.delivery_id()),
            election_event_id: self.election_event_id.clone(),
            message_type: LogMessageType::Internal,
            user_id: None,
            username: None,
            tenant_id: self.tenant_id.clone(),
            body: LogEventBody::Plain(
                serde_json::to_string(&PendingBody {
                    pending_statement: self.clone(),
                })
                .context("Error serializing a pending statement")?,
            ),
            event_time_ms: None,
            details: None,
            admin_event: None,
            deferral: None,
        })
    }

    /// Queue it on the event queue, for the batch task to sign and store.
    pub async fn queue(&self) -> Result<()> {
        let input = self.event()?;
        tracing::warn!(
            origin = self.origin.as_str(),
            delivery_id = input.delivery_id.as_deref().unwrap_or_default(),
            "The board's signer is unavailable: the electoral-log statement is queued unsigned, to be signed later"
        );
        crate::services::celery_app::try_get_celery_app()
            .await?
            .send_task(crate::tasks::electoral_log::enqueue_electoral_log_event::new(input))
            .await
            .context("Error queueing an unsigned electoral-log statement")?;
        Ok(())
    }

    /// Reads and checks the pending statement an event carries, before
    /// anything is signed:
    ///
    /// - the event is an `Internal` event whose body is exactly
    ///   `{"pending_statement": <this type>}` (unknown fields, kinds and
    ///   origins refused) at [`PENDING_STATEMENT_VERSION`];
    /// - the statement's kind is one this module allows, and its origin is
    ///   the one allowed to queue that kind (Datafix only, for an external
    ///   API request);
    /// - it is bound to the event: the same tenant and election event (both
    ///   UUIDs), and a delivery ID `pending:<its origin>:<UUID>`, so it can't
    ///   take another producer's delivery ID (and stop that producer's own
    ///   entry, delivery IDs being idempotent);
    /// - its time is set, not more than [`MAX_FUTURE_SECS`] ahead of `now`,
    ///   and not more than [`MAX_LAG_SECS`] before the event was first queued
    ///   to wait (its deferral), or before `now` for one not yet deferred;
    /// - identifiers are at most [`MAX_ID_CHARS`] printable characters, and
    ///   no text field is longer than [`MAX_FIELD_CHARS`].
    pub fn read(input: &LogEventInput, now: u64) -> Result<PendingStatement> {
        ensure!(
            input.message_type == LogMessageType::Internal,
            "Not an internal event"
        );
        let body: PendingBody =
            serde_json::from_str(&input.body.as_raw()).context("Not a pending statement")?;
        let pending = body.pending_statement;
        pending.validate(input, now)?;
        Ok(pending)
    }

    fn validate(&self, input: &LogEventInput, now: u64) -> Result<()> {
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
        if let PendingStatementBody::ExternalApiRequest { api_name, .. } = &self.statement {
            ensure!(
                *api_name == ExtApiName::Datafix,
                "Only Datafix requests may be queued unsigned"
            );
        }
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
        let queued = input
            .deferral
            .and_then(|deferral| u64::try_from(deferral.since_ms / 1000).ok())
            .unwrap_or(now);
        ensure!(
            self.occurred_at.saturating_add(MAX_LAG_SECS) >= queued,
            "The pending statement's time is too long before it was queued"
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
        PendingStatementBody::ExternalApiRequest { .. } => "external-api-request",
        PendingStatementBody::EnrollmentProblems { .. } => "enrollment-problems",
        PendingStatementBody::BallotPublicationFailure { .. } => "ballot-publication-failure",
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

    fn reread(input: &LogEventInput) -> Result<PendingStatement> {
        PendingStatement::read(input, electoral_log::timestamp())
    }

    #[test]
    fn a_trusted_statement_is_read_back_and_signed_at_its_time() {
        let mut pending = PendingStatement::new(TENANT, EVENT, failure());
        let at = electoral_log::timestamp() - 30;
        pending.occurred_at = at;
        let input = pending.event().unwrap();
        assert!(input
            .delivery_id
            .as_deref()
            .unwrap()
            .starts_with("pending:ballot-publication:"));
        let read = reread(&input).unwrap();
        let key = SigningKey::generate().unwrap();
        let sd = SigningData::new(key.clone(), "", key.clone());
        let message = read.sign(&sd).unwrap();
        message
            .verify(&VerificationKey::from_sk(&key).unwrap())
            .unwrap();
        assert_eq!(message.statement.head.timestamp, at);
        // Signed again later (it waited), the statement is the same bytes.
        use voting_crypto::RecordSerialize;
        let again = read.sign(&sd).unwrap();
        assert_eq!(
            again.statement.record_serialize().unwrap(),
            message.statement.record_serialize().unwrap()
        );
        assert_eq!(message.statement.head.event.0, EVENT);
    }

    #[test]
    fn a_forged_foreign_or_disallowed_statement_is_refused() {
        let good = PendingStatement::new(TENANT, EVENT, failure());
        let event = good.event().unwrap();
        assert!(reread(&event).is_ok());

        // Bound to another election event or tenant than its event.
        let mut other_event = event.clone();
        other_event.election_event_id = OTHER.into();
        assert!(reread(&other_event).is_err());
        let mut other_tenant = event.clone();
        other_tenant.tenant_id = OTHER.into();
        assert!(reread(&other_tenant).is_err());

        // A kind its origin may not queue.
        let mut wrong_origin = good.clone();
        wrong_origin.origin = PendingOrigin::CastVoteRefusal;
        let mut forged = event.clone();
        forged.body = LogEventBody::Plain(
            serde_json::to_string(&PendingBody {
                pending_statement: wrong_origin,
            })
            .unwrap(),
        );
        assert!(reread(&forged).is_err());

        // An unknown origin or kind, or an extra field: not this type.
        let body = |value: serde_json::Value| {
            let mut object = serde_json::Map::new();
            object.insert(PENDING_BODY_KEY.to_string(), value);
            LogEventBody::Plain(serde_json::Value::Object(object).to_string())
        };
        let mut unknown = event.clone();
        let mut value = serde_json::to_value(&good).unwrap();
        value["origin"] = "anyone".into();
        unknown.body = body(value);
        assert!(reread(&unknown).is_err());
        for kind in ["election-published", "cast-vote", "readiness-verified", "checkpoint"] {
            let mut value = serde_json::to_value(&good).unwrap();
            value["statement"]["kind"] = kind.into();
            unknown.body = body(value);
            assert!(reread(&unknown).is_err(), "{kind}");
        }
        let mut value = serde_json::to_value(&good).unwrap();
        value["statement_bytes"] = "AAAA".into();
        unknown.body = body(value);
        assert!(reread(&unknown).is_err());
        let mut value = serde_json::to_value(&good).unwrap();
        value["version"] = 2.into();
        unknown.body = body(value);
        assert!(reread(&unknown).is_err());
        // A signed record's body is not a pending statement.
        assert!(!is_pending_body(r#"{"id":0,"message":[1]}"#));
        assert!(is_pending_body(&event.body.as_raw()));

        // Another producer's delivery ID.
        let mut squatting = event.clone();
        squatting.delivery_id = Some("readiness:entry".into());
        assert!(reread(&squatting).is_err());
        let mut other_origin = event.clone();
        other_origin.delivery_id = Some(PendingOrigin::CastVoteRefusal.delivery_id());
        assert!(reread(&other_origin).is_err());

        // A time in the future.
        let mut future = good.clone();
        future.occurred_at = electoral_log::timestamp() + 3_600;
        let mut later = event.clone();
        later.body = LogEventBody::Plain(
            serde_json::to_string(&PendingBody {
                pending_statement: future,
            })
            .unwrap(),
        );
        assert!(reread(&later).is_err());

        // Not an internal event.
        let mut keycloak = event.clone();
        keycloak.message_type = LogMessageType::KeycloakEvent("LOGIN".into());
        assert!(reread(&keycloak).is_err());

        // Too old for when it was queued.
        let mut stale = good.clone();
        stale.occurred_at = electoral_log::timestamp() - MAX_LAG_SECS - 60;
        let mut old = event.clone();
        old.body = LogEventBody::Plain(
            serde_json::to_string(&PendingBody {
                pending_statement: stale,
            })
            .unwrap(),
        );
        assert!(reread(&old).is_err());

        // An identifier that is not printable.
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
        assert!(reread(&control.event().unwrap()).is_err());

        // A request to another API.
        let other_api = PendingStatement::new(
            TENANT,
            EVENT,
            PendingStatementBody::ExternalApiRequest {
                election_id: None,
                voter_id: None,
                voter_username: None,
                direction: ExtApiRequestDirection::Outbound,
                api_name: ExtApiName::Other,
                operation: "x".into(),
            },
        );
        assert!(reread(&other_api.event().unwrap()).is_err());

        // A field too long.
        let huge = PendingStatement::new(
            TENANT,
            EVENT,
            PendingStatementBody::EnrollmentProblems {
                problems: "x".repeat(MAX_FIELD_CHARS + 1),
            },
        );
        assert!(reread(&huge.event().unwrap()).is_err());
    }
}
