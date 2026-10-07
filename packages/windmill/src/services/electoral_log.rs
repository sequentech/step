// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, PgConfig};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log_checkpoint_copies::{
    store_checkpoint_copy, CheckpointCopyConfig,
};
use crate::services::insert_cast_vote::hash_voter_id;
use crate::services::protocol_manager::get_board_client;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::vault;
use crate::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
};
use crate::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};
use anyhow::{anyhow, ensure, Context, Result};
use b4::messages::message::Signer;
use base64::engine::general_purpose;
use base64::Engine;
use deadpool_postgres::Transaction;
use electoral_log::domain::{
    Filter, LogEntry, LogQuery, LogVisibility, NumberColumn, NumberComparison,
};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{CertificateAuthEventAction, *};
use electoral_log::messages::statement::{StatementBody, StatementType};
use electoral_log::{
    ElectoralLogMessage, ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use rust_decimal::prelude::ToPrimitive;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::retry::retry_with_exponential_backoff;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::time::Duration;
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::HashWrapper;
use strand::hash::STRAND_HASH_LENGTH_BYTES;
use strand::serialization::StrandDeserialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strum_macros::{Display, EnumString};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, warn, Level};
use uuid::Uuid;

pub const ELECTORAL_LOG_ROWS_LIMIT: usize = 2500;
pub const MAX_ROWS_PER_PAGE: usize = 50;

/// Ballot_id input is the first half of the original hash which is stored in the electoral log.
pub const BALLOT_ID_LENGTH_BYTES: usize = STRAND_HASH_LENGTH_BYTES / 2;
/// Ballot_id input is in HEX, each byte is represented in 2 chars.
pub const BALLOT_ID_LENGTH_CHARS: usize = BALLOT_ID_LENGTH_BYTES * 2;

/// Record failures after the publication transaction has ended, so rollback
/// cannot erase the diagnostic and the external write holds no DB connection.
#[instrument(skip_all, err)]
pub async fn log_ballot_publication_failure(
    task: &TasksExecution,
    publication_id: &str,
    stage: BallotPublicationStage,
    error_message: &str,
) -> Result<()> {
    let event_id = task
        .election_event_id
        .as_deref()
        .context("Publication task has no election event")?;
    let log = {
        let mut db = get_hasura_pool().await.get().await?;
        let tx = db.transaction().await?;
        let event = get_election_event_by_id(&tx, &task.tenant_id, event_id).await?;
        let board = get_election_event_board(event.bulletin_board_reference)
            .context("Election event is missing its electoral-log board")?;
        let log = ElectoralLog::new(&tx, &task.tenant_id, Some(event_id), &board).await?;
        tx.commit().await?;
        log
    };
    let message = Message::ballot_publication_failure_message(
        EventIdString(event_id.to_owned()),
        BallotPublicationFailure {
            publication_id: BallotPublicationIdString(publication_id.to_owned()),
            task_id: task.id.clone(),
            stage,
            error: ErrorMessageString(error_message.to_owned()),
        },
        &log.sd,
        Some(task.executed_by_user.clone()),
    )?;
    log.post(&message).await
}

/// Identifies the admin user whose request caused a voter password change.
/// The password itself must never be added to this context or to the
/// electoral-log message built from it.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElectoralLogAdminContext {
    pub user_id: String,
    pub username: Option<String>,
    pub authorized_election_ids: Option<Vec<String>>,
    pub area_id: Option<String>,
}

impl ElectoralLogAdminContext {
    pub fn from_claims(claims: &JwtClaims) -> Self {
        Self {
            user_id: claims.hasura_claims.user_id.clone(),
            username: claims.preferred_username.clone(),
            authorized_election_ids: claims.hasura_claims.authorized_election_ids.clone(),
            area_id: claims.hasura_claims.area_id.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoterPasswordChangeSource {
    AdminPortal,
    VoterInformationLetter,
}

impl VoterPasswordChangeSource {
    fn event_type(self) -> &'static str {
        match self {
            Self::AdminPortal => "UPDATE_PASSWORD: ADMIN_PORTAL",
            Self::VoterInformationLetter => "UPDATE_PASSWORD: VOTER_INFORMATION_LETTER",
        }
    }
}

#[derive(Serialize)]
struct ElectoralLogUser<'a> {
    user_id: &'a str,
    username: Option<&'a str>,
}

#[derive(Serialize)]
struct VoterPasswordChangeBody<'a> {
    action: &'static str,
    source: VoterPasswordChangeSource,
    voter: ElectoralLogUser<'a>,
    initiated_by: ElectoralLogUser<'a>,
}

fn voter_password_change_body(
    voter_id: &str,
    voter_username: Option<&str>,
    admin: &ElectoralLogAdminContext,
    source: VoterPasswordChangeSource,
) -> Result<String> {
    serde_json::to_string(&VoterPasswordChangeBody {
        action: "voter_password_changed",
        source,
        voter: ElectoralLogUser {
            user_id: voter_id,
            username: voter_username,
        },
        initiated_by: ElectoralLogUser {
            user_id: &admin.user_id,
            username: admin.username.as_deref(),
        },
    })
    .context("Failed to serialize voter password-change electoral-log details")
}

/// A signed voter password-change log entry that can be persisted after the
/// Hasura transaction which prepared it commits.
///
/// This contains no password or generated voter credential. It is serializable
/// so a task can durably retain the exact signed message until delivery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreparedVoterPasswordChangeLog {
    board: String,
    delivery_id: String,
    message: ElectoralLogMessage,
}

impl PreparedVoterPasswordChangeLog {
    #[instrument(skip(self), err)]
    pub async fn post(&self) -> Result<()> {
        let entries = [LogEntry {
            delivery_id: self.delivery_id.clone(),
            message: self.message.clone(),
        }];
        retry_with_exponential_backoff(
            || async {
                get_board_client()
                    .await?
                    .append(&self.board, &entries)
                    .await
            },
            5,
            Duration::from_millis(100),
        )
        .await
    }
}

/// Prepares a signed password-change audit entry using the caller's Hasura
/// transaction. The caller can therefore commit its document and durable task
/// state before performing the external electoral-log write.
#[instrument(skip_all, err)]
pub async fn prepare_voter_password_change(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    voter_username: Option<String>,
    admin: &ElectoralLogAdminContext,
    source: VoterPasswordChangeSource,
) -> Result<PreparedVoterPasswordChangeLog> {
    let election_event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await
        .context("Failed to get election event for the password-change electoral log")?;
    let board = get_election_event_board(election_event.bulletin_board_reference)
        .context("Election event is missing its electoral-log board")?;
    let electoral_log = ElectoralLog::for_admin_user(
        hasura_transaction,
        &board,
        tenant_id,
        election_event_id,
        &admin.user_id,
        admin.username.clone(),
        admin.authorized_election_ids.clone(),
        admin.area_id.clone(),
    )
    .await
    .context("Failed to initialize the admin-signed password-change electoral log")?;
    let body = voter_password_change_body(voter_id, voter_username.as_deref(), admin, source)?;
    let message = electoral_log
        .build_keycloak_event_message(
            election_event_id.to_string(),
            source.event_type().to_string(),
            body,
            Some(voter_id.to_string()),
            voter_username,
            None,
        )
        .context("Failed to build the voter password-change electoral-log entry")?;

    Ok(PreparedVoterPasswordChangeLog {
        board,
        delivery_id: Uuid::new_v4().to_string(),
        message,
    })
}

/// Posts a signed electoral-log entry after an admin-triggered voter password
/// change succeeds. Callers that already hold a Hasura transaction should use
/// `prepare_voter_password_change` and post the result only after committing.
#[instrument(skip_all, err)]
pub async fn post_voter_password_change(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    voter_username: Option<String>,
    admin: &ElectoralLogAdminContext,
    source: VoterPasswordChangeSource,
) -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get Hasura client for the password-change electoral log")?;
    let transaction = client
        .transaction()
        .await
        .context("Failed to start password-change electoral-log transaction")?;
    let prepared = prepare_voter_password_change(
        &transaction,
        tenant_id,
        election_event_id,
        voter_id,
        voter_username,
        admin,
        source,
    )
    .await?;
    transaction
        .commit()
        .await
        .context("Failed to commit the password-change electoral-log transaction")?;
    prepared
        .post()
        .await
        .context("Failed to post the voter password-change electoral-log entry")
}

/// What an administrator did with one or more secret voter attributes.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoterSecretAttributeAction {
    /// A stored value was decrypted and shown in the Admin Portal.
    Reveal,
    /// A value was created or replaced.
    Set,
    /// A stored value was removed.
    Clear,
    /// Values were imported from a CSV file.
    Import,
    /// Decrypted values were written into an export document.
    Export,
    /// Decrypted values were injected into an email or SMS template.
    Communication,
    /// Decrypted values were injected into a per-voter report.
    Report,
}

impl VoterSecretAttributeAction {
    fn event_type(self) -> &'static str {
        match self {
            Self::Reveal => "VOTER_SECRET_ATTRIBUTE: REVEAL",
            Self::Set => "VOTER_SECRET_ATTRIBUTE: SET",
            Self::Clear => "VOTER_SECRET_ATTRIBUTE: CLEAR",
            Self::Import => "VOTER_SECRET_ATTRIBUTE: IMPORT",
            Self::Export => "VOTER_SECRET_ATTRIBUTE: EXPORT",
            Self::Communication => "VOTER_SECRET_ATTRIBUTE: COMMUNICATION",
            Self::Report => "VOTER_SECRET_ATTRIBUTE: REPORT",
        }
    }
}

/// The subject of a secret-attribute audit entry. Values are never part of
/// it: only which attributes, for which voter, and which document or task
/// consumed them.
#[derive(Clone, Debug, Default)]
pub struct VoterSecretAttributeAudit<'a> {
    pub voter_id: Option<&'a str>,
    pub voter_username: Option<&'a str>,
    pub attribute_names: &'a [String],
    pub document_id: Option<&'a str>,
}

#[derive(Serialize)]
struct VoterSecretAttributeAuditBody<'a> {
    action: VoterSecretAttributeAction,
    attribute_names: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voter: Option<ElectoralLogUser<'a>>,
    initiated_by: ElectoralLogUser<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_id: Option<&'a str>,
}

fn voter_secret_attribute_audit_body(
    action: VoterSecretAttributeAction,
    audit: &VoterSecretAttributeAudit<'_>,
    admin: &ElectoralLogAdminContext,
) -> Result<String> {
    let mut attribute_names: Vec<&str> = audit.attribute_names.iter().map(String::as_str).collect();
    attribute_names.sort_unstable();
    attribute_names.dedup();
    serde_json::to_string(&VoterSecretAttributeAuditBody {
        action,
        attribute_names,
        voter: audit.voter_id.map(|user_id| ElectoralLogUser {
            user_id,
            username: audit.voter_username,
        }),
        initiated_by: ElectoralLogUser {
            user_id: &admin.user_id,
            username: admin.username.as_deref(),
        },
        document_id: audit.document_id,
    })
    .context("Failed to serialize voter secret-attribute electoral-log details")
}

/// Posts an admin-signed electoral-log entry recording who revealed, changed,
/// cleared, imported or consumed which secret voter attributes. Callers post
/// it before handing out or storing a value, so a failure to record the
/// action stops the action.
#[instrument(skip_all, err)]
pub async fn post_voter_secret_attribute_audit(
    tenant_id: &str,
    election_event_id: &str,
    admin: &ElectoralLogAdminContext,
    action: VoterSecretAttributeAction,
    audit: VoterSecretAttributeAudit<'_>,
) -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get Hasura client for the secret-attribute electoral log")?;
    let transaction = client
        .transaction()
        .await
        .context("Failed to start secret-attribute electoral-log transaction")?;
    let prepared = prepare_voter_secret_attribute_audit(
        &transaction,
        tenant_id,
        election_event_id,
        admin,
        action,
        audit,
    )
    .await?;
    transaction
        .commit()
        .await
        .context("Failed to commit the secret-attribute electoral-log transaction")?;
    prepared
        .post()
        .await
        .context("Failed to post the secret-attribute electoral-log entry")
}

/// Imports create their event and signing context in the caller's transaction.
/// Build the audit there so those uncommitted records are visible.
#[instrument(skip_all, err)]
pub async fn post_voter_secret_attribute_audit_with_transaction(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    admin: &ElectoralLogAdminContext,
    action: VoterSecretAttributeAction,
    audit: VoterSecretAttributeAudit<'_>,
) -> Result<()> {
    prepare_voter_secret_attribute_audit(
        transaction,
        tenant_id,
        election_event_id,
        admin,
        action,
        audit,
    )
    .await?
    .post()
    .await
    .context("Failed to post the secret-attribute electoral-log entry")
}

async fn prepare_voter_secret_attribute_audit(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    admin: &ElectoralLogAdminContext,
    action: VoterSecretAttributeAction,
    audit: VoterSecretAttributeAudit<'_>,
) -> Result<PreparedVoterPasswordChangeLog> {
    let election_event = get_election_event_by_id(&transaction, tenant_id, election_event_id)
        .await
        .context("Failed to get election event for the secret-attribute electoral log")?;
    let board = get_election_event_board(election_event.bulletin_board_reference)
        .context("Election event is missing its electoral-log board")?;
    let electoral_log = ElectoralLog::for_admin_user(
        &transaction,
        &board,
        tenant_id,
        election_event_id,
        &admin.user_id,
        admin.username.clone(),
        admin.authorized_election_ids.clone(),
        admin.area_id.clone(),
    )
    .await
    .context("Failed to initialize the admin-signed secret-attribute electoral log")?;
    let body = voter_secret_attribute_audit_body(action, &audit, admin)?;
    let message = electoral_log
        .build_keycloak_event_message(
            election_event_id.to_string(),
            action.event_type().to_string(),
            body,
            audit.voter_id.map(str::to_string),
            audit.voter_username.map(str::to_string),
            None,
        )
        .context("Failed to build the secret-attribute electoral-log entry")?;
    Ok(PreparedVoterPasswordChangeLog {
        board,
        delivery_id: Uuid::new_v4().to_string(),
        message,
    })
}

/// Prefix of the delivery IDs of checkpoint statements, which are unique per log size.
const CHECKPOINT_DELIVERY_PREFIX: &str = "electoral-log-checkpoint:";

/// Sign a checkpoint publication with the system key of `sd`.
pub fn sign_checkpoint(
    sd: &SigningData,
    checkpoint: &electoral_log::proofs::Checkpoint,
    reason: ElectoralLogCheckpointReason,
) -> Result<crate::postgres::electoral_log_checkpoint::PublishedCheckpoint> {
    let bytes = electoral_log::proofs::checkpoint_signing_bytes(checkpoint, reason)?;
    Ok(
        crate::postgres::electoral_log_checkpoint::PublishedCheckpoint {
            board_name: checkpoint.log_name.clone(),
            log_id: checkpoint.log_id,
            tree_size: i64::try_from(checkpoint.tree_size)?,
            root: hex::encode(&checkpoint.root),
            reason: reason.to_string(),
            signer_pk: sd.system_pk()?.to_der_b64_string()?,
            signature: sd.system_sign(&bytes)?.to_b64_string()?,
        },
    )
}

pub struct ElectoralLog {
    pub(crate) sd: SigningData,
    pub(crate) elog_database: String,
}

pub fn flatten_election_ids(election_ids: Option<Vec<String>>) -> Option<String> {
    election_ids
        .map(|ids| {
            if ids.len() == 1 {
                Some(ids[0].clone())
            } else {
                None
            }
        })
        .flatten()
}

impl ElectoralLog {
    #[instrument(err, name = "ElectoralLog::new")]
    pub async fn new(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id_opt: Option<&str>,
        elog_database: &str,
    ) -> Result<Self> {
        let election_event_id =
            election_event_id_opt.ok_or(anyhow!("Election event id is required"))?;

        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            elog_database,
        )
        .await?;

        Ok(ElectoralLog {
            sd: SigningData::new(
                protocol_manager.get_signing_key().clone(),
                "",
                protocol_manager.get_signing_key().clone(),
            ),
            elog_database: elog_database.to_string(),
        })
    }

    #[instrument(skip(sender_sk), err)]
    pub async fn new_from_sk(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        elog_database: &str,
        sender_sk: &StrandSignatureSk,
    ) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.get_signing_key().clone();

        Ok(ElectoralLog {
            sd: SigningData::new(sender_sk.clone(), "", system_sk),
            elog_database: elog_database.to_string(),
        })
    }

    /// Construct a system-authored audit message using an already loaded key.
    /// The empty sender name preserves the system identity used by `new` and
    /// `new_from_sk`; voter attribution belongs in the message's actor fields.
    pub fn for_system_with_signing_key(elog_database: &str, system_sk: &StrandSignatureSk) -> Self {
        Self {
            sd: SigningData::new(system_sk.clone(), "", system_sk.clone()),
            elog_database: elog_database.to_string(),
        }
    }

    /// Reuses an election's already loaded system signing key. This is the
    /// same signing identity as `for_voter`, without another database lookup.
    pub fn for_voter_with_signing_key(
        elog_database: &str,
        user_id: &str,
        system_sk: &StrandSignatureSk,
    ) -> Self {
        Self {
            sd: SigningData::new(system_sk.clone(), user_id, system_sk.clone()),
            elog_database: elog_database.to_string(),
        }
    }

    /// Returns an electoral log whose posts will have the given voter
    /// as the signing sender, as well as the system signer.
    ///
    /// The sender signing private key is obtained from the vault.
    ///
    /// We need to pass in the log database because the vault
    /// will post a public key message if it needs to generates
    /// a signing key.
    #[instrument(skip(voter_signing_key), err)]
    pub async fn for_voter(
        hasura_transaction: &Transaction<'_>,
        elog_database: &str,
        tenant_id: &str,
        event_id: &str,
        user_id: &str,
        voter_signing_key: &Option<StrandSignaturePk>,
    ) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.get_signing_key().clone();

        Ok(ElectoralLog {
            sd: SigningData::new(system_sk.clone(), user_id, system_sk),
            elog_database: elog_database.to_string(),
        })
    }

    /// Returns an electoral log whose posts will have the given admin
    /// user as the signing sender, as well as the system signer.
    ///
    /// The sender signing private key is obtained from the vault.
    ///
    /// We need to pass in the log database because the vault
    /// will post a public key message if it needs to generates
    /// a signing key.
    #[instrument(err, skip(hasura_transaction))]
    pub async fn for_admin_user(
        hasura_transaction: &Transaction<'_>,
        elog_database: &str,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        username: Option<String>,
        election_ids_vec: Option<Vec<String>>,
        user_area_id: Option<String>,
    ) -> Result<Self> {
        let election_ids = flatten_election_ids(election_ids_vec);
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.get_signing_key().clone();

        let sk = vault::get_admin_user_signing_key(
            hasura_transaction,
            elog_database,
            tenant_id,
            user_id,
            username,
            election_ids,
            user_area_id,
        )
        .await?;

        Ok(ElectoralLog {
            sd: SigningData::new(sk, "", system_sk),
            elog_database: elog_database.to_string(),
        })
    }

    /// Posts a voter's public key
    #[instrument(err)]
    pub async fn post_voter_pk(
        hasura_transaction: &Transaction<'_>,
        elog_database: &str,
        tenant_id: &str,
        event_id: &str,
        user_id: &str,
        pk_der_b64: &str,
        area_id: &str,
    ) -> Result<()> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.get_signing_key().clone();
        let sd = SigningData::new(system_sk.clone(), "", system_sk.clone());

        let pseudonym = hash_voter_id(&user_id)?;
        let message = Message::voter_public_key_message(
            TenantIdString(tenant_id.to_string()),
            EventIdString(event_id.to_string()),
            PseudonymHash(HashWrapper::new(pseudonym)),
            PublicKeyDerB64(pk_der_b64.to_string()),
            &sd,
            Some(user_id.to_string()),
            None, /* username */
            Some(area_id.to_string()),
        )?;
        let board_message: ElectoralLogMessage = (&message).try_into().with_context(|| {
            "Error converting Message::cast_vote_message into ElectoralLogMessage"
        })?;
        let input = LogEventInput {
            delivery_id: Some(Uuid::new_v4().to_string()),
            election_event_id: event_id.to_string(),
            message_type: LogMessageType::Internal,
            user_id: Some(user_id.to_string()),
            username: None,
            tenant_id: tenant_id.to_string(),
            body: LogEventBody::Plain(
                serde_json::to_string(&board_message)
                    .with_context(|| "Error serializing ElectoralLogMessage")?,
            ),
        };

        let celery_app = get_celery_app().await;
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
        Ok(())
    }

    /// Posts an admin user's public key
    ///
    /// Because admin users are cross election event entities, a
    /// dummy election event id will be used instead, with value
    /// electoral_log::messages::Message:GENERIC_EVENT.
    ///
    /// FIXME: it may be necessary to implement a tenant-wide electoral
    /// log to save this type of message. An admin user could be created
    /// in the context of one event and the notification will only
    /// be present in its log, even if the corresponding signing private key
    /// would be used in other events.
    pub async fn post_admin_pk(
        hasura_transaction: &Transaction<'_>,
        elog_database: &str,
        tenant_id: &str,
        user_id: &str,
        username: Option<String>,
        pk_der_b64: &str,
        elections_ids: Option<String>,
        user_area_id: Option<String>,
    ) -> Result<()> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            None,
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.get_signing_key().clone();
        let sd = SigningData::new(system_sk.clone(), "", system_sk.clone());

        let message = Message::admin_public_key_message(
            TenantIdString(tenant_id.to_string()),
            Some(user_id.to_string()),
            username,
            PublicKeyDerB64(pk_der_b64.to_string()),
            &sd,
            elections_ids,
            user_area_id,
        )?;

        let elog = ElectoralLog {
            sd,
            elog_database: elog_database.to_string(),
        };

        let ret = elog.post(&message).await;

        if ret.is_err() {
            tracing::error!(
                "Unable to post public key for admin user {:?}, {:?}",
                message,
                ret
            );
        }

        ret
    }

    #[instrument(skip(self, pseudonym_h))]
    pub async fn post_cast_vote_error(
        &self,
        tenant_id: String,
        event_id: String,
        election_id: Option<String>,
        pseudonym_h: PseudonymHash,
        error: String,
        voter_ip: String,
        voter_country: String,
        voter_id: String,
        voter_username: Option<String>,
        area_id: String,
    ) -> Result<()> {
        let event = EventIdString(event_id.clone());
        let election = ElectionIdString(election_id);
        let error = CastVoteErrorString(error);
        let ip = VoterIpString(voter_ip);
        let country = VoterCountryString(voter_country);

        let message = Message::cast_vote_error_message(
            event,
            election,
            pseudonym_h,
            error,
            &self.sd,
            ip,
            country,
            Some(voter_id.clone()),
            area_id,
        )?;
        let board_message: ElectoralLogMessage = (&message).try_into().with_context(|| {
            "Error converting Message::cast_vote_error_message into ElectoralLogMessage"
        })?;
        let input = LogEventInput {
            delivery_id: Some(Uuid::new_v4().to_string()),
            election_event_id: event_id,
            message_type: LogMessageType::Internal,
            user_id: Some(voter_id),
            username: voter_username.clone(),
            tenant_id,
            body: LogEventBody::Plain(
                serde_json::to_string(&board_message)
                    .with_context(|| "Error serializing post cast vote")?,
            ),
        };
        let celery_app = get_celery_app().await;
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn post_phone_blacklist_entry_created(
        &self,
        event_id: String,
        number_e164: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let message = Message::phone_blacklist_entry_created_message(
            event,
            PhoneE164String(number_e164),
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_phone_blacklist_entry_deleted(
        &self,
        event_id: String,
        number_e164: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let message = Message::phone_blacklist_entry_deleted_message(
            event,
            PhoneE164String(number_e164),
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip_all, fields(direction = %direction, api_name = %api_name), err)]
    pub async fn post_external_api_request(
        &self,
        tenant_id: String,
        event_id: String,
        election_id: Option<String>,
        voter_id: Option<String>,
        voter_username: Option<String>,
        direction: ExtApiRequestDirection,
        api_name: ExtApiName,
        operation: String,
    ) -> Result<()> {
        let event = EventIdString(event_id.clone());
        let election = ElectionIdString(election_id);

        let message = Message::external_api_request_message(
            event,
            election,
            &self.sd,
            voter_id.clone(),
            voter_username.clone(),
            direction,
            api_name,
            operation,
        )?;

        let board_message: ElectoralLogMessage = (&message).try_into().with_context(|| {
            "Error converting Message::external_api_request_message into ElectoralLogMessage"
        })?;
        let input = LogEventInput {
            delivery_id: Some(Uuid::new_v4().to_string()),
            election_event_id: event_id,
            message_type: LogMessageType::Internal,
            user_id: voter_id,
            username: voter_username,
            tenant_id,
            body: LogEventBody::Plain(
                serde_json::to_string(&board_message)
                    .with_context(|| "Error serializing ElectoralLogMessage")?,
            ),
        };
        let celery_app = get_celery_app().await;
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
        Ok(())
    }

    /// Posts a third-party voter registry reconciliation run event (patch
    /// generation or applying the Sequent-side diff) — see
    /// `windmill::services::external::reconciliation`. Named for the general
    /// capability, not the specific integration (Datafix) that first needed
    /// it. `artifact` carries the JSON of old/new values applied, for a
    /// `ChangesApplied` entry (`None` for `PatchGenerated`, which has nothing
    /// to apply yet).
    #[instrument(skip(self, artifact), fields(kind = %kind), err)]
    pub async fn post_external_reconciliation(
        &self,
        event_id: String,
        kind: ExternalReconciliationKind,
        sequence: i64,
        generated_at: i64,
        input_sha256: String,
        output_sha256: Option<String>,
        artifact: Option<Vec<u8>>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);

        let message = Message::external_reconciliation_message(
            event,
            kind,
            ExternalReconciliationSequenceString(sequence.to_string()),
            ExternalReconciliationGeneratedAtString(generated_at.to_string()),
            ExternalReconciliationInputHashString(input_sha256),
            ExternalReconciliationOutputHashString(output_sha256),
            artifact,
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_results_publication_action(
        &self,
        event_id: String,
        details: ResultsPublicationDetails,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let message = Message::results_publication_action_message(
            EventIdString(event_id),
            details,
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_published(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        ballot_pub_id: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids.clone());
        let ballot_pub_id = BallotPublicationIdString(ballot_pub_id);

        let message = Message::election_published_message(
            event,
            election,
            ballot_pub_id,
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_open(
        &self,
        event_id: String,
        election_id: Option<String>,
        elections_ids: Option<Vec<String>>,
        voting_channel: VotingChannelString,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = election_id.map(|id| ElectionIdString(Some(id)));
        let message = Message::election_open_message(
            event,
            election,
            elections_ids,
            voting_channel,
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_pause(
        &self,
        event_id: String,
        election_id: Option<String>,
        voting_channel: VotingChannelString,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = election_id.map(|id| ElectionIdString(Some(id)));

        let message = Message::election_pause_message(
            event,
            election,
            voting_channel,
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_close(
        &self,
        event_id: String,
        election_id: Option<String>,
        elections_ids: Option<Vec<String>>,
        voting_channel: VotingChannelString,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = election_id.map(|id| ElectionIdString(Some(id)));

        let message = Message::election_close_message(
            event,
            election,
            elections_ids,
            voting_channel,
            &self.sd,
            user_id,
            username,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_keycloak_event(
        &self,
        event_id: String,
        event_type: String,
        error_message: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let error_message = ErrorMessageString(error_message);
        let event_type = KeycloakEventTypeString(event_type);
        let message = Message::keycloak_user_event(
            event,
            event_type,
            error_message,
            user_id,
            username,
            &self.sd,
            None,
        )?;
        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_keygen(
        &self,
        event_id: String,
        user_id: Option<String>,
        username: Option<String>,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);

        let message = Message::keygen_message(event, &self.sd, user_id, username, election_id)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion_start(
        &self,
        event_id: String,
        user_id: Option<String>,
        username: Option<String>,
        election_ids_vec: Option<Vec<String>>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);

        let message =
            Message::key_insertion_start(event, &self.sd, user_id, username, election_ids)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion(
        &self,
        event_id: String,
        trustee_name: String,
        user_id: Option<String>,
        username: Option<String>,
        election_ids_vec: Option<Vec<String>>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let trustee_name = TrusteeNameString(trustee_name);
        let election_ids = flatten_election_ids(election_ids_vec);

        let message = Message::key_insertion_message(
            event,
            trustee_name,
            &self.sd,
            user_id,
            username,
            election_ids,
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_open(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids);

        let message = Message::tally_open_message(event, election, &self.sd, user_id, username)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub(crate) async fn post_tally_close(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids);

        let message = Message::tally_close_message(event, election, &self.sd, user_id, username)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_send_template(
        &self,
        message: Option<String>,
        event_id: String,
        user_id: Option<String>,
        username: Option<String>,
        election_id: Option<String>,
        area_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::send_template(
            event, election, &self.sd, user_id, username, message, area_id,
        )
        .map_err(|e| anyhow!("Error sending template: {e:?}"))?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_resumed_with_resolution(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        resolution_ids: Vec<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids);

        let message =
            Message::tally_resumed_with_resolution(event, election, resolution_ids, &self.sd)
                .map_err(|e| anyhow!("Error posting tally resumed with resolution: {e:?}"))?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_paused_pending_resolution(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        resolution_ids: Vec<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids);

        let message =
            Message::tally_paused_pending_resolutions(event, election, resolution_ids, &self.sd)
                .map_err(|e| anyhow!("Error posting tally paused pending resolution: {e:?}"))?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_tie_resolved(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        contest_id: String,
        resolution_id: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids);
        let contest = ContestIdString(contest_id);

        let message = Message::tally_tie_resolved(
            event,
            election,
            contest,
            resolution_id,
            &self.sd,
            user_id,
            username,
        )
        .map_err(|e| anyhow!("Error posting tally tie resolved: {e:?}"))?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_tie_resolution_updated(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        contest_id: String,
        resolution_id: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election_ids = flatten_election_ids(election_ids_vec);
        let election = ElectionIdString(election_ids);
        let contest = ContestIdString(contest_id);

        let message = Message::tally_tie_resolution_updated(
            event,
            election,
            contest,
            resolution_id,
            &self.sd,
            user_id,
            username,
        )
        .map_err(|e| anyhow!("Error posting tally tie resolution updated: {e:?}"))?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_certificate_auth_event(
        &self,
        event_id: String,
        action: CertificateAuthEventAction,
        subject_dns: Vec<String>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let message = Message::certificate_auth_event_message(
            event,
            action,
            subject_dns,
            &self.sd,
            user_id,
            username,
        )?;
        self.post(&message).await
    }

    /// Publish a signed checkpoint of this event's electoral log.
    ///
    /// The checkpoint is stored in the Hasura database, outside the electoral-log
    /// database, on its own connection so a caller's transaction is never aborted by
    /// it, and a statement recording it is appended to the log once per log size.
    #[instrument(skip(self), err)]
    pub async fn publish_checkpoint(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        reason: ElectoralLogCheckpointReason,
    ) -> Result<electoral_log::proofs::Checkpoint> {
        let checkpoint =
            crate::services::protocol_manager::get_electoral_log_store(&self.elog_database)
                .await?
                .journal()
                .checkpoint(&self.elog_database)
                .await?;
        let published = sign_checkpoint(&self.sd, &checkpoint, reason)?;
        // Copy first, so that with the `required` policy nothing is published without
        // its write-once copy.
        store_checkpoint_copy(
            &CheckpointCopyConfig::from_env()?,
            tenant_id,
            election_event_id,
            &published,
        )
        .await?;
        let mut hasura_db_client = get_hasura_pool()
            .await
            .get()
            .await
            .context("Error acquiring hasura connection")?;
        let hasura_transaction = hasura_db_client.transaction().await?;
        let stored = crate::postgres::electoral_log_checkpoint::insert_electoral_log_checkpoint(
            &hasura_transaction,
            tenant_id,
            election_event_id,
            &published,
        )
        .await?;
        ensure!(
            stored.root == published.root,
            "A different checkpoint was already published at size {}: the log may have been \
             rolled back or forked",
            checkpoint.tree_size
        );
        hasura_transaction.commit().await?;
        // Record the stored publication in the log. The delivery ID is fixed per log size,
        // so publishing the same size again, or retrying, records it once.
        let message = Message::electoral_log_checkpoint_message(
            EventIdString(election_event_id.to_string()),
            ElectoralLogCheckpoint {
                log_id: checkpoint.log_id,
                tree_size: checkpoint.tree_size,
                root: published.root.clone(),
                reason: stored
                    .reason
                    .parse()
                    .with_context(|| format!("Unknown checkpoint reason {}", stored.reason))?,
            },
            &self.sd,
        )?;
        self.post_with_delivery_id(
            &message,
            format!(
                "{CHECKPOINT_DELIVERY_PREFIX}{}:{}",
                checkpoint.log_id, checkpoint.tree_size
            ),
        )
        .await
        .context("The checkpoint was stored but could not be recorded in the log")?;
        Ok(checkpoint)
    }

    #[instrument(skip(self), err)]
    async fn post(&self, message: &Message) -> Result<()> {
        self.post_with_delivery_id(message, Uuid::new_v4().to_string())
            .await
    }

    /// Append a message under a delivery ID; a repeated delivery ID appends nothing.
    #[instrument(skip(self, message), err)]
    async fn post_with_delivery_id(&self, message: &Message, delivery_id: String) -> Result<()> {
        let board_message: ElectoralLogMessage = message.try_into()?;
        let ms = vec![LogEntry {
            delivery_id,
            message: board_message,
        }];

        retry_with_exponential_backoff(
            // The closure we want to call repeatedly
            || async {
                let mut client = get_board_client().await?;
                client.append(self.elog_database.as_str(), &ms).await
            },
            // Maximum number of retries:
            5,
            // Initial backoff:
            Duration::from_millis(100),
        )
        .await
    }

    /// Builds a keycloak event message and returns the resulting ElectoralLogMessage.
    pub fn build_keycloak_event_message(
        &self,
        event_id: String,
        event_type: String,
        error_message: String,
        user_id: Option<String>,
        username: Option<String>,
        area_id: Option<String>,
    ) -> Result<ElectoralLogMessage> {
        let event = EventIdString(event_id);
        let error_message = ErrorMessageString(error_message);
        let event_type = KeycloakEventTypeString(event_type);
        let message = &Message::keycloak_user_event(
            event,
            event_type,
            error_message,
            user_id,
            username,
            &self.sd,
            area_id,
        )?;
        let board_message: ElectoralLogMessage = message.try_into()?;
        Ok(board_message)
    }

    /// Builds a send-template message and returns the resulting ElectoralLogMessage.
    pub fn build_send_template_message(
        &self,
        message_body: Option<String>,
        event_id: String,
        user_id: Option<String>,
        username: Option<String>,
        election_id: Option<String>,
        area_id: Option<String>,
    ) -> Result<ElectoralLogMessage> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);
        let message = &Message::send_template(
            event,
            election,
            &self.sd,
            user_id,
            username,
            message_body,
            area_id,
        )
        .map_err(|e| anyhow!("Error creating send template message: {:?}", e))?;
        let board_message: ElectoralLogMessage = message.try_into()?;
        Ok(board_message)
    }

    #[instrument(skip(self))]
    pub async fn import_from_csv(&self, logs_file: &NamedTempFile) -> Result<()> {
        let mut reader = csv::Reader::from_reader(logs_file);
        let mut entries = reader.deserialize::<ElectoralLogRow>().map(|row| {
            let row = row.context("Failed to read electoral-log CSV row")?;
            let bytes = general_purpose::STANDARD_NO_PAD.decode(&row.data)?;
            let message = Message::strand_deserialize(&bytes)
                .map_err(|err| anyhow!("Failed to deserialize message: {err:?}"))?;
            let mut stored: ElectoralLogMessage = (&message).try_into()?;
            stored.created = row.created;
            stored.message = bytes;
            Ok(LogEntry {
                delivery_id: Uuid::new_v4().to_string(),
                message: stored,
            })
        });
        get_board_client()
            .await?
            .append_iter(&self.elog_database, &mut entries)
            .await
    }
}

// Fields accepted by the electoral-log API
#[derive(Debug, Deserialize, Hash, PartialEq, Eq, EnumString, Display, Clone)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OrderField {
    Id,
    Created,
    StatementTimestamp,
    StatementKind,
    Message,
    UserId,
    Username,
    BallotId,
    SenderPk,
    LogType,
    EventType,
    Description,
    Version,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct GetElectoralLogBody {
    pub tenant_id: String,
    pub election_event_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filter: Option<HashMap<OrderField, String>>,
    pub order_by: Option<HashMap<OrderField, OrderDirection>>,
    pub election_id: Option<String>,
    pub area_ids: Option<Vec<String>>,
    pub only_with_user: Option<bool>,
    pub statement_kind: Option<StatementType>,
}

impl GetElectoralLogBody {
    fn as_query(&self) -> Result<LogQuery> {
        let limits = PgConfig::from_env()?;
        let mut query = LogQuery {
            limit: self
                .limit
                .unwrap_or(limits.default_sql_limit.into())
                .min(limits.low_sql_limit.into()),
            offset: self.offset.unwrap_or(0).max(0),
            only_with_user: self.only_with_user.unwrap_or(false),
            ..LogQuery::default()
        };
        if let Some(filters) = &self.filter {
            for (field, value) in filters {
                match field {
                    OrderField::Id => query.filters.push(Filter::Number(
                        NumberColumn::Id,
                        NumberComparison::Equal,
                        value.parse()?,
                    )),
                    OrderField::SenderPk
                    | OrderField::UserId
                    | OrderField::Username
                    | OrderField::BallotId
                    | OrderField::StatementKind
                    | OrderField::Version => {
                        query.filters.push(Filter::Text(
                            field.to_string().parse()?,
                            SqlCompOperators::Like,
                            value.clone(),
                        ));
                    }
                    OrderField::StatementTimestamp | OrderField::Created => {
                        let timestamp = ISO8601::to_date_utc(value)?.timestamp();
                        let column = match field {
                            OrderField::Created => NumberColumn::Created,
                            _ => NumberColumn::StatementTimestamp,
                        };
                        query.filters.push(Filter::Number(
                            column,
                            NumberComparison::GreaterThanOrEqual,
                            timestamp,
                        ));
                        query.filters.push(Filter::Number(
                            column,
                            NumberComparison::LessThan,
                            timestamp + 60,
                        ));
                    }
                    OrderField::EventType
                    | OrderField::LogType
                    | OrderField::Description
                    | OrderField::Message => {}
                }
            }
        }
        if self.election_id.is_some() || self.area_ids.is_some() {
            query.visibility = Some(LogVisibility {
                election_id: self.election_id.clone(),
                area_ids: self.area_ids.clone().unwrap_or_default(),
            });
        }
        if let Some(kind) = &self.statement_kind {
            query.filters.push(Filter::Text(
                ElectoralLogVarCharColumn::StatementKind,
                SqlCompOperators::Equal,
                kind.to_string(),
            ));
        }
        if let Some(order) = &self.order_by {
            let mut order: Vec<_> = order
                .iter()
                .map(|(field, direction)| (field.to_string(), direction.to_string()))
                .collect();
            order.sort();
            query.order = order
                .into_iter()
                .map(|(field, direction)| Ok((field.parse()?, direction.parse()?)))
                .collect::<Result<_>>()?;
        }
        query.validate()?;
        Ok(query)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElectoralLogRow {
    pub id: i64,
    pub created: i64,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: String,
    pub data: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StatementHeadDataString {
    pub event: String,
    pub kind: String,
    pub timestamp: i64,
    pub event_type: String,
    pub log_type: String,
    pub description: String,
}

impl ElectoralLogRow {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn created(&self) -> i64 {
        self.created
    }

    pub fn statement_timestamp(&self) -> i64 {
        self.statement_timestamp
    }

    pub fn statement_kind(&self) -> &str {
        &self.statement_kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_ref().map(|s| s.as_str())
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_ref().map(|s| s.as_str())
    }

    pub fn statement_head_data(&self) -> Result<StatementHeadDataString> {
        let message: serde_json::Value = deserialize_str(&self.message).map_err(|err| {
            anyhow!(format!(
                "{:?}, Failed to parse message: {}",
                err, self.message
            ))
        })?;

        let Some(statement) = message.get("statement") else {
            return Err(anyhow!(
                "Failed to get statement from message: {}",
                self.message
            ));
        };

        let Some(head) = statement.get("head") else {
            return Err(anyhow!(
                "Failed to get head from statement: {}",
                self.message
            ));
        };

        let data: StatementHeadDataString = deserialize_value(head.clone())
            .map_err(|err| anyhow!(format!("{:?}, Failed to parse head: {}", err, head)))?;

        Ok(data)
    }
}

impl TryFrom<ElectoralLogMessage> for ElectoralLogRow {
    type Error = anyhow::Error;

    fn try_from(elog_msg: ElectoralLogMessage) -> Result<Self, Self::Error> {
        let serialized = general_purpose::STANDARD_NO_PAD.encode(elog_msg.message.clone());
        let deserialized_message = Message::strand_deserialize(&elog_msg.message)
            .map_err(|e| anyhow!("Error deserializing message: {e:?}"))?;

        Ok(ElectoralLogRow {
            id: elog_msg.id,
            created: elog_msg.created,
            statement_timestamp: elog_msg.statement_timestamp,
            statement_kind: elog_msg.statement_kind.clone(),
            message: serde_json::to_string_pretty(&deserialized_message)
                .with_context(|| "Error serializing message to json")?,
            data: serialized,
            user_id: elog_msg.user_id.clone(),
            username: elog_msg.username.clone(),
        })
    }
}

/// A cast vote as voters see it in the ballot locator. It carries nothing that
/// identifies the voter: no username, IP address, country or signed message.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct CastVoteEntry {
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub ballot_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CastVoteMessagesOutput {
    pub list: Vec<CastVoteEntry>,
    pub total: usize,
}

impl CastVoteEntry {
    pub fn from_elog_message(entry: &ElectoralLogMessage) -> Self {
        CastVoteEntry {
            statement_timestamp: entry.statement_timestamp,
            statement_kind: StatementType::CastVote.to_string(),
            ballot_id: entry.ballot_id.clone().unwrap_or_default(),
        }
    }
}

/// Fields voters can sort the ballot locator by.
pub const CAST_VOTE_ORDER_FIELDS: [OrderField; 4] = [
    OrderField::Id,
    OrderField::StatementTimestamp,
    OrderField::StatementKind,
    OrderField::BallotId,
];

/// Refuses a ballot-locator sort by a field voters cannot see, such as the
/// username.
pub fn check_cast_vote_order_by(
    order_by: Option<&HashMap<OrderField, OrderDirection>>,
) -> Result<()> {
    let mut refused: Vec<String> = order_by
        .into_iter()
        .flat_map(HashMap::keys)
        .filter(|field| !CAST_VOTE_ORDER_FIELDS.contains(field))
        .map(ToString::to_string)
        .collect();
    refused.sort();
    ensure!(
        refused.is_empty(),
        "Cast votes cannot be sorted by {}",
        refused.join(", ")
    );
    Ok(())
}

#[instrument(err)]
pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
    let client = get_board_client().await?;
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let board = get_event_board(&input.tenant_id, &input.election_event_id, &slug);
    let query = input.as_query()?;
    let items = client
        .query(&board, &query)
        .await?
        .into_iter()
        .map(ElectoralLogRow::try_from)
        .collect::<Result<Vec<_>>>()?;
    let count = client.count(&board, &query).await?;
    Ok(DataList {
        items,
        total: TotalAggregate {
            aggregate: Aggregate { count },
        },
    })
}

#[instrument]
pub fn get_cols_match_count_and_select(
    election_id: &str,
    user_id: &str,
    ballot_id_filter: &str,
) -> (WhereClauseBTreeMap, WhereClauseBTreeMap) {
    let cols_match_count = BTreeMap::from([
        (
            ElectoralLogVarCharColumn::StatementKind,
            (SqlCompOperators::Equal, StatementType::CastVote.to_string()),
        ),
        (
            ElectoralLogVarCharColumn::ElectionId,
            (SqlCompOperators::Equal, election_id.to_string()),
        ),
    ]);
    let mut cols_match_select = cols_match_count.clone();
    // Restrict the SQL query to user_id and ballot_id in case of filtering
    if !ballot_id_filter.is_empty() {
        cols_match_select.insert(
            ElectoralLogVarCharColumn::UserId,
            (SqlCompOperators::Equal, user_id.to_string()),
        );
        cols_match_select.insert(
            ElectoralLogVarCharColumn::BallotId,
            (SqlCompOperators::Like, ballot_id_filter.to_string()),
        );
    }

    (cols_match_count, cols_match_select)
}

/// Returns the entries for statement_kind = "CastVote" which ballot_id matches the input
/// ballot_id_filter is restricted to be an even number of characters, so that can be converted
/// to a byte array
#[instrument(err)]
pub async fn list_cast_vote_messages(
    input: GetElectoralLogBody,
    ballot_id_filter: &str,
    user_id: &str,
    username: &str,
) -> Result<CastVoteMessagesOutput> {
    ensure!(
        ballot_id_filter.chars().count() % 2 == 0 && ballot_id_filter.is_ascii(),
        "Incorrect ballot_id, the length must be an even number of characters"
    );
    check_cast_vote_order_by(input.order_by.as_ref())?;
    // The limits are used to cut the output after filtering the ballot id.
    // Because ballot_id cannot be filtered at SQL level the sql limit is constant
    let output_limit: i64 = input.limit.unwrap_or(MAX_ROWS_PER_PAGE as i64);
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(
        input.tenant_id.as_str(),
        input.election_event_id.as_str(),
        &slug,
    );
    info!("database name = {board_name}");
    let order_by = input.order_by.clone();
    let election_id = input.election_id.clone().unwrap_or_default();

    let limit: i64 = match ballot_id_filter.is_empty() {
        false => ELECTORAL_LOG_ROWS_LIMIT as i64, // When there is a filter, need to fetch all entries by batches.
        true => input.limit.unwrap_or(MAX_ROWS_PER_PAGE as i64),
    };
    let mut offset: i64 = input.offset.unwrap_or(0);
    let mut list: Vec<CastVoteEntry> = Vec::with_capacity(MAX_ROWS_PER_PAGE); // Filtered messages.
    let (cols_match_count, cols_match_select) =
        get_cols_match_count_and_select(&election_id, user_id, ballot_id_filter);
    let mut client = get_board_client().await?;
    let total = client
        .count_electoral_log_messages(&board_name, Some(cols_match_count))
        .await?
        .to_u64()
        .unwrap_or(0) as usize;
    let mut filter_matched = false; // Exit at the first match if the filter is not empty
    while (list.len() as i64) < output_limit && (offset < total as i64) && !filter_matched {
        let electoral_log_messages = client
            .get_electoral_log_messages_filtered(
                &board_name,
                Some(cols_match_select.clone()),
                None,
                None,
                Some(limit),
                Some(offset),
                order_by.clone(),
            )
            .await
            .map_err(|err| anyhow!("Failed to get filtered messages: {:?}", err))?;

        let t_entries = electoral_log_messages.len();
        info!("Got {t_entries} entries. Offset: {offset}, limit: {limit}, total: {total}");
        for message in electoral_log_messages.iter() {
            list.push(CastVoteEntry::from_elog_message(message));
            // If there is a filter, exit at the first match
            filter_matched = !ballot_id_filter.is_empty();
            if (list.len() as i64) >= output_limit || filter_matched {
                break;
            }
        }
        offset += limit;
    }

    Ok(CastVoteMessagesOutput { list, total })
}

#[instrument(err)]
pub async fn count_electoral_log(input: GetElectoralLogBody) -> Result<i64> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let board = get_event_board(&input.tenant_id, &input.election_event_id, &slug);
    get_board_client()
        .await?
        .count(&board, &input.as_query()?)
        .await
}

#[cfg(test)]
mod password_change_tests {
    use super::*;
    use serde_json::json;

    fn admin() -> ElectoralLogAdminContext {
        ElectoralLogAdminContext {
            user_id: "admin-id".to_string(),
            username: Some("admin-user".to_string()),
            authorized_election_ids: Some(vec!["election-id".to_string()]),
            area_id: Some("area-id".to_string()),
        }
    }

    fn prepared_message() -> ElectoralLogMessage {
        ElectoralLogMessage {
            id: 0,
            created: 1_785_700_000,
            sender_pk: "sender-public-key".to_string(),
            statement_timestamp: 1_785_700_000,
            statement_kind: "keycloak_user_event".to_string(),
            message: vec![1, 2, 3, 4],
            version: "1".to_string(),
            user_id: Some("voter-id".to_string()),
            username: Some("voter-user".to_string()),
            election_id: None,
            area_id: None,
            ballot_id: None,
        }
    }

    #[test]
    fn password_change_body_identifies_subject_actor_and_source_without_a_credential() {
        let body = voter_password_change_body(
            "voter-id",
            Some("voter-user"),
            &admin(),
            VoterPasswordChangeSource::VoterInformationLetter,
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();

        assert_eq!(
            body,
            json!({
                "action": "voter_password_changed",
                "source": "voter_information_letter",
                "voter": {
                    "user_id": "voter-id",
                    "username": "voter-user",
                },
                "initiated_by": {
                    "user_id": "admin-id",
                    "username": "admin-user",
                },
            })
        );
        assert_eq!(body.as_object().unwrap().len(), 4);
    }

    #[test]
    fn password_change_event_type_distinguishes_admin_portal_and_vil_changes() {
        assert_eq!(
            VoterPasswordChangeSource::AdminPortal.event_type(),
            "UPDATE_PASSWORD: ADMIN_PORTAL"
        );
        assert_eq!(
            VoterPasswordChangeSource::VoterInformationLetter.event_type(),
            "UPDATE_PASSWORD: VOTER_INFORMATION_LETTER"
        );
    }

    #[test]
    fn prepared_password_change_log_round_trips_for_durable_task_annotations() {
        let prepared = PreparedVoterPasswordChangeLog {
            board: "event-board".to_string(),
            delivery_id: "delivery-id".to_string(),
            message: prepared_message(),
        };

        let serialized = serde_json::to_value(&prepared).unwrap();
        let restored: PreparedVoterPasswordChangeLog = serde_json::from_value(serialized).unwrap();

        assert_eq!(restored, prepared);
    }
}

#[cfg(test)]
mod voter_secret_attribute_audit_tests {
    use super::*;

    #[test]
    fn audit_body_names_attributes_and_actors_but_never_values() {
        let admin = ElectoralLogAdminContext {
            user_id: "admin-id".to_string(),
            username: Some("admin".to_string()),
            authorized_election_ids: None,
            area_id: None,
        };
        let names = vec![
            "reference".to_string(),
            "code".to_string(),
            "code".to_string(),
        ];
        let body = voter_secret_attribute_audit_body(
            VoterSecretAttributeAction::Reveal,
            &VoterSecretAttributeAudit {
                voter_id: Some("voter-id"),
                voter_username: Some("voter"),
                attribute_names: &names,
                document_id: None,
            },
            &admin,
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["action"], "reveal");
        assert_eq!(
            body["attribute_names"],
            serde_json::json!(["code", "reference"])
        );
        assert_eq!(body["voter"]["user_id"], "voter-id");
        assert_eq!(body["initiated_by"]["username"], "admin");
        assert!(body.get("document_id").is_none());
    }
}

#[cfg(test)]
mod postgres_wiring_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a disposable ELECTORAL_LOG_PG_* database and ENV_SLUG"]
    async fn prepared_append_list_count_and_delete_use_the_postgres_store() -> Result<()> {
        let tenant = Uuid::new_v4().to_string();
        let event = Uuid::new_v4().to_string();
        let slug = std::env::var("ENV_SLUG")?;
        let board = get_event_board(&tenant, &event, &slug);
        let client = get_board_client().await?;
        client.create_board(&board).await?;
        let key = StrandSignatureSk::generate()?;
        let log = ElectoralLog::for_system_with_signing_key(&board, &key);
        let message = log.build_keycloak_event_message(
            event.clone(),
            "LOGIN".into(),
            "ok".into(),
            Some("user".into()),
            Some("username".into()),
            None,
        )?;
        let prepared = PreparedVoterPasswordChangeLog {
            board: board.clone(),
            delivery_id: Uuid::new_v4().to_string(),
            message: message.clone(),
        };
        prepared.post().await?;
        prepared.post().await?;
        let input = GetElectoralLogBody {
            tenant_id: tenant,
            election_event_id: event,
            ..Default::default()
        };
        assert_eq!(count_electoral_log(input.clone()).await?, 1);
        let page = list_electoral_log(input.clone()).await?;
        assert_eq!(page.total.aggregate.count, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(
            general_purpose::STANDARD_NO_PAD.decode(&page.items[0].data)?,
            message.message
        );
        let mut other = input.clone();
        other.tenant_id = Uuid::new_v4().to_string();
        assert_eq!(count_electoral_log(other).await?, 0);
        let store = crate::services::protocol_manager::get_electoral_log_store(&board).await?;
        let checkpoint = store.journal().checkpoint(&board).await?;
        assert_eq!(checkpoint.tree_size, 1);
        let report = store.audit(&board, &[checkpoint]).await?;
        assert!(report.is_clean(), "{:?}", report.findings());
        client.delete_board(&board).await?;
        assert_eq!(count_electoral_log(input).await?, 0);
        Ok(())
    }
}

#[cfg(test)]
mod cast_vote_entry_tests {
    use super::*;

    fn cast_vote_record() -> ElectoralLogMessage {
        ElectoralLogMessage {
            id: 7,
            created: 1_785_700_000,
            sender_pk: "sender-public-key".to_string(),
            statement_timestamp: 1_785_700_001,
            statement_kind: StatementType::CastVote.to_string(),
            message: vec![1, 2, 3, 4],
            version: "1".to_string(),
            user_id: Some("voter-id".to_string()),
            username: Some("voter-user".to_string()),
            election_id: Some("election-id".to_string()),
            area_id: Some("area-id".to_string()),
            ballot_id: Some("abcd".to_string()),
        }
    }

    #[test]
    fn voters_receive_only_the_ballot_id_timestamp_and_kind() {
        let entry = CastVoteEntry::from_elog_message(&cast_vote_record());

        assert_eq!(
            serde_json::to_value(&entry).unwrap(),
            serde_json::json!({
                "statement_timestamp": 1_785_700_001,
                "statement_kind": "CastVote",
                "ballot_id": "abcd",
            })
        );
    }

    #[test]
    fn voters_can_sort_only_by_fields_they_see() {
        assert!(check_cast_vote_order_by(None).is_ok());
        for field in CAST_VOTE_ORDER_FIELDS {
            let order_by = HashMap::from([(field, OrderDirection::Desc)]);
            assert!(check_cast_vote_order_by(Some(&order_by)).is_ok());
        }

        for field in [
            OrderField::Username,
            OrderField::UserId,
            OrderField::Message,
        ] {
            let order_by = HashMap::from([
                (OrderField::Id, OrderDirection::Desc),
                (field.clone(), OrderDirection::Asc),
            ]);
            let error = check_cast_vote_order_by(Some(&order_by))
                .unwrap_err()
                .to_string();
            assert!(error.contains(&field.to_string()), "{error}");
        }
    }
}
