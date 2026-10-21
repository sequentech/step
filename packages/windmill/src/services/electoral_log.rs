// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::messaging::{latest_attempt_ids, ChannelFilter};
use crate::services::celery_app::try_get_celery_app;
use crate::services::database::{get_hasura_pool, PgConfig};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log_board::get_board_client;
use crate::services::electoral_log_board::get_board_signing_key;
use crate::services::electoral_log_board::get_event_board;
use crate::services::electoral_log_board::get_tenant_board;
use crate::services::electoral_log_board::TenantLog;
use crate::services::electoral_log_checkpoint_copies::{
    store_checkpoint_copy, CheckpointCopyConfig,
};
use crate::services::insert_cast_vote::hash_voter_id;
use crate::services::vault;
use crate::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
};
use crate::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};
use anyhow::{anyhow, ensure, Context, Result};
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
use sequent_core::election_config::manifest::sha256_hex;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::messaging::{MessageChannel, UNRESOLVED_AFTER_HOURS};
use sequent_core::util::retry::retry_with_exponential_backoff;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::time::Duration;
use strum_macros::{Display, EnumString};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, warn, Level};
use uuid::Uuid;
use velvet::pipes::report_manifest::csv_report_reader;
use voting_crypto::hash::HashWrapper;
use voting_crypto::hash::HASH_LENGTH_BYTES;
use voting_crypto::signing::{SigningKey, VerificationKey};
use voting_crypto::RecordDeserialize;
use voting_crypto::RistrettoCtx;

pub const ELECTORAL_LOG_ROWS_LIMIT: usize = 2500;
pub const MAX_ROWS_PER_PAGE: usize = 50;

/// Ballot_id input is the first half of the original hash which is stored in the electoral log.
pub const BALLOT_ID_LENGTH_BYTES: usize = HASH_LENGTH_BYTES / 2;
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
                    .await?;
                Ok(())
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
/// Prefix of the delivery IDs of continuation statements, unique per continued log.
const CONTINUATION_DELIVERY_PREFIX: &str = "electoral-log-continuation:";
/// Prefix of the delivery IDs of approval matrix versions: `<version>:<sha256>`.
const APPROVAL_MATRIX_DELIVERY_PREFIX: &str = "approval-matrix:";
/// Prefix of the delivery IDs of monitoring configuration changes:
/// `<generation>:<sha256 of the change>`.
const MONITORING_CONFIG_DELIVERY_PREFIX: &str = "monitoring-config:";
/// Prefix of the delivery IDs of configuration package imports:
/// `<external ID>:<revision>:<manifest sha256>`.
const CONFIGURATION_PACKAGE_DELIVERY_PREFIX: &str = "configuration-package:";
/// Prefix of the delivery IDs of report manifests: `<report manifest sha256>`.
const REPORT_GENERATED_DELIVERY_PREFIX: &str = "report-generated:";
/// Prefix of the delivery IDs of admin public keys: `<user ID>:<sha256 of the
/// public key>`.
const ADMIN_PK_DELIVERY_PREFIX: &str = "admin-public-key:";

/// The delivery id of an admin's public key on the tenant's log: one per
/// account and key.
pub fn admin_pk_delivery_id(user_id: &str, pk_der_b64: &str) -> String {
    format!(
        "{ADMIN_PK_DELIVERY_PREFIX}{user_id}:{}",
        sha256_hex(pk_der_b64.as_bytes())
    )
}

fn approval_matrix_delivery_id(version: u32, sha256: &str) -> String {
    format!("{APPROVAL_MATRIX_DELIVERY_PREFIX}{version}:{sha256}")
}

fn monitoring_config_delivery_id(details: &MonitoringConfigChangeDetails) -> Result<String> {
    let change = serde_json::to_vec(details).context("Error serializing the monitoring change")?;
    Ok(format!(
        "{MONITORING_CONFIG_DELIVERY_PREFIX}{}:{}",
        details.generation,
        sha256_hex(&change)
    ))
}

fn configuration_package_delivery_id(details: &ConfigurationPackageDetails) -> String {
    format!(
        "{CONFIGURATION_PACKAGE_DELIVERY_PREFIX}{}:{}:{}",
        details.external_id, details.revision, details.manifest_sha256
    )
}

fn report_generated_delivery_id(details: &ReportGeneratedDetails) -> String {
    format!(
        "{REPORT_GENERATED_DELIVERY_PREFIX}{}",
        details.report_manifest_sha256
    )
}

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
            log_uid: checkpoint.log_uid,
            tree_size: i64::try_from(checkpoint.tree_size)?,
            root: hex::encode(&checkpoint.root),
            reason: reason.to_string(),
            signer_pk: sd.system_public_key()?.to_b64_string()?,
            signature: sd.system_sign(&bytes)?.to_b64_string()?,
        },
    )
}

/// A reader of the log's CSV export: the backup an event export holds, or
/// the report, which starts with a comment line naming the configuration
/// when the event was imported from a signed one.
pub fn exported_log_reader<R: std::io::Read>(
    file: R,
) -> Result<csv::Reader<std::io::BufReader<R>>> {
    csv_report_reader(file).context("Failed to read the CSV file")
}

/// The entry of a row of a CSV export of the log, under a new delivery ID.
/// The row's `statement_kind` column must name the statement its message
/// reads as: a file written by a release that numbers the statements
/// differently (one exported before the electoral log moved to PostgreSQL,
/// whose new statements come after the checkpoint statements now) would be
/// read as other statements, so it is refused.
fn csv_log_entry(row: ElectoralLogRow) -> Result<LogEntry> {
    let bytes = general_purpose::STANDARD_NO_PAD.decode(&row.data)?;
    let message = Message::record_deserialize(&bytes).map_err(|err| {
        anyhow!(
            "Row {} ({}) of the electoral-log CSV can't be read, which happens with a file \
             written by a release that numbers the statements differently: {err:?}",
            row.id,
            row.statement_kind
        )
    })?;
    let mut stored: ElectoralLogMessage = (&message).try_into()?;
    ensure!(
        stored.statement_kind == row.statement_kind,
        "Row {} of the electoral-log CSV is a {} entry but its message reads as {}: the file \
         was written by a release that numbers the statements differently and can't be imported",
        row.id,
        row.statement_kind,
        stored.statement_kind
    );
    stored.created = row.created;
    stored.message = bytes;
    Ok(LogEntry {
        delivery_id: Uuid::new_v4().to_string(),
        message: stored,
    })
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
    /// Sign the complete successful snapshot and wait for its append. This is
    /// deliberately not a queued success: the operator needs retained evidence.
    pub async fn post_readiness_verified(
        &self,
        event_id: &str,
        report: &crate::domain::readiness::ReadinessReport,
        actor: &crate::services::audit_log::AuditActor,
    ) -> Result<()> {
        ensure!(
            report.passed,
            "Only successful readiness checks may be recorded"
        );
        let entry_id = report
            .log_entry_id
            .as_ref()
            .context("Readiness entry has no delivery identity")?;
        let entry = AuditLogEntry {
            kind: AuditStatementKind::ReadinessVerified,
            event_type: electoral_log::messages::statement::StatementEventType::SYSTEM,
            log_type: electoral_log::messages::statement::StatementLogType::INFO,
            description: format!(
                "Readiness verified against event {}",
                report.evidence.tested.event_id
            ),
            details_json: serde_json::to_string(
                &serde_json::json!({"actor": actor, "report": report}),
            )?,
            entry_id: entry_id.clone(),
        };
        let timestamp = chrono::DateTime::parse_from_rfc3339(&report.checked_at)?.timestamp();
        let message = Message::audit_message(
            EventIdString(event_id.into()),
            entry,
            u64::try_from(timestamp)?,
            &self.sd,
            actor.user_id.clone(),
            actor.username.clone(),
            None,
            None,
        )?;
        self.post_with_delivery_id(&message, format!("readiness:{entry_id}"))
            .await
    }

    #[instrument(err, name = "ElectoralLog::new")]
    pub async fn new(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id_opt: Option<&str>,
        elog_database: &str,
    ) -> Result<Self> {
        let election_event_id =
            election_event_id_opt.ok_or(anyhow!("Election event id is required"))?;

        let protocol_manager = get_board_signing_key(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            elog_database,
        )
        .await?;

        Ok(ElectoralLog {
            sd: SigningData::new(protocol_manager.clone(), "", protocol_manager.clone()),
            elog_database: elog_database.to_string(),
        })
    }

    #[instrument(skip(sender_sk), err)]
    pub async fn new_from_sk(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        elog_database: &str,
        sender_sk: &SigningKey,
    ) -> Result<Self> {
        let protocol_manager = get_board_signing_key(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.clone();

        Ok(ElectoralLog {
            sd: SigningData::new(sender_sk.clone(), "", system_sk),
            elog_database: elog_database.to_string(),
        })
    }

    /// Construct a system-authored audit message using an already loaded key.
    /// The empty sender name preserves the system identity used by `new` and
    /// `new_from_sk`; voter attribution belongs in the message's actor fields.
    pub fn for_system_with_signing_key(elog_database: &str, system_sk: &SigningKey) -> Self {
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
        system_sk: &SigningKey,
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
        voter_signing_key: &Option<VerificationKey>,
    ) -> Result<Self> {
        let protocol_manager =
            get_board_signing_key(hasura_transaction, tenant_id, Some(event_id), elog_database)
                .await?;
        let system_sk = protocol_manager.clone();

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
        let protocol_manager = get_board_signing_key(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.clone();

        let sk = vault::get_admin_user_signing_key(
            hasura_transaction,
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
        let protocol_manager =
            get_board_signing_key(hasura_transaction, tenant_id, Some(event_id), elog_database)
                .await?;
        let system_sk = protocol_manager.clone();
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
            event_time_ms: None,
            details: None,
            admin_event: None,
        };

        let celery_app = try_get_celery_app().await?;
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
        Ok(())
    }

    /// Posts an admin user's public key to the tenant's log, signed with the
    /// log's system key: the key is one per tenant and signs the admin's
    /// entries in every election event and in the tenant's log. The entry's
    /// delivery id is made from the account and the key, so posting the same
    /// key again stores nothing.
    ///
    /// Callers post once the admin's key is committed (`vault`), with a
    /// `tenant_log` whose system key is committed too.
    pub async fn post_admin_pk(
        tenant_log: &TenantLog,
        tenant_id: &str,
        user_id: &str,
        username: Option<String>,
        pk_der_b64: &str,
        elections_ids: Option<String>,
        user_area_id: Option<String>,
    ) -> Result<()> {
        let elog_database = tenant_log.board.as_str();
        let system_sk = tenant_log.system_key.clone();
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

        let ret = elog
            .post_with_delivery_id(&message, admin_pk_delivery_id(user_id, pk_der_b64))
            .await;

        if ret.is_err() {
            tracing::error!(
                "Unable to post public key for admin user {:?}, {:?}",
                message,
                ret
            );
        }

        ret
    }

    #[instrument(skip_all)]
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
            event_time_ms: None,
            details: None,
            admin_event: None,
        };
        let celery_app = try_get_celery_app().await?;
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
        Ok(())
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
            event_time_ms: None,
            details: None,
            admin_event: None,
        };
        let celery_app = try_get_celery_app().await?;
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

    /// Posts a new version of the election event's approval matrix, once per
    /// version and digest.
    #[instrument(skip(self))]
    pub async fn post_approval_matrix_updated(
        &self,
        event_id: String,
        version: u32,
        sha256: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let delivery_id = approval_matrix_delivery_id(version, &sha256);
        let message = Message::approval_matrix_updated_message(
            EventIdString(event_id),
            ApprovalMatrixVersion(version),
            ApprovalMatrixDigestString(sha256),
            &self.sd,
            user_id,
            username,
        )?;

        self.post_with_delivery_id(&message, delivery_id).await
    }

    /// Posts a change to the election event's monitoring dashboards, once per
    /// generation and change.
    #[instrument(skip(self))]
    pub async fn post_monitoring_config_changed(
        &self,
        event_id: String,
        details: MonitoringConfigChangeDetails,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let delivery_id = monitoring_config_delivery_id(&details)?;
        let message = Message::monitoring_config_changed_message(
            EventIdString(event_id),
            details,
            &self.sd,
            user_id,
            username,
        )?;

        self.post_with_delivery_id(&message, delivery_id).await
    }

    /// Posts a signed configuration package's import, once per package
    /// revision and manifest.
    #[instrument(skip(self))]
    pub async fn post_configuration_package(
        &self,
        event_id: String,
        details: ConfigurationPackageImportDetails,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let delivery_id = configuration_package_delivery_id(&details.package);
        let message = Message::configuration_package_with_content_hash_message(
            EventIdString(event_id),
            details,
            &self.sd,
            user_id,
            username,
        )?;

        self.post_with_delivery_id(&message, delivery_id).await
    }

    /// Posts the hash manifest of a generated report, once per manifest.
    #[instrument(skip(self))]
    pub async fn post_report_generated(
        &self,
        event_id: String,
        details: ReportGeneratedDetails,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let delivery_id = report_generated_delivery_id(&details);
        let message = Message::report_generated_message(
            EventIdString(event_id),
            details,
            &self.sd,
            user_id,
            username,
        )?;

        self.post_with_delivery_id(&message, delivery_id).await
    }

    /// Posts a publication. For an event imported from a signed
    /// configuration package the entry carries `configuration`: the
    /// manifest SHA-256 and the digest of each design published.
    #[instrument(skip(self))]
    pub async fn post_election_published(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        ballot_pub_id: String,
        configuration: Option<PublishedConfiguration>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let message = self.election_published_message(
            event_id,
            election_ids_vec,
            ballot_pub_id,
            configuration,
            user_id,
            username,
        )?;
        self.post(&message).await
    }

    fn election_published_message(
        &self,
        event_id: String,
        election_ids_vec: Option<Vec<String>>,
        ballot_pub_id: String,
        configuration: Option<PublishedConfiguration>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Message> {
        Message::election_published_message(
            EventIdString(event_id),
            ElectionIdString(flatten_election_ids(election_ids_vec)),
            BallotPublicationIdString(ballot_pub_id),
            configuration,
            &self.sd,
            user_id,
            username,
        )
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

    #[instrument(skip_all)]
    pub async fn post_keycloak_event(
        &self,
        event_id: String,
        event_type: String,
        error_message: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let error_message = ErrorMessageString(crate::services::privacy_signin::safe_error_text(
            &error_message,
        ));
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
        let checkpoint = crate::services::electoral_log_board::get_event_store(election_event_id)
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
            ElectoralLogCheckpointV2 {
                log_uid: checkpoint.log_uid.hyphenated().to_string(),
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
                checkpoint.log_uid, checkpoint.tree_size
            ),
        )
        .await
        .context("The checkpoint was stored but could not be recorded in the log")?;
        Ok(checkpoint)
    }

    /// Record that this board continues a log, whose last checkpoint is `previous`,
    /// as an imported event's board continues the board of the event it was exported
    /// from. Recorded once per previous log.
    #[instrument(skip(self), err)]
    pub async fn post_continuation(
        &self,
        election_event_id: &str,
        previous: &electoral_log::proofs::Checkpoint,
    ) -> Result<()> {
        let message = Message::electoral_log_continuation_message(
            EventIdString(election_event_id.to_string()),
            ElectoralLogContinuation {
                previous_log_name: previous.log_name.clone(),
                previous_log_uid: previous.log_uid.hyphenated().to_string(),
                tree_size: previous.tree_size,
                root: hex::encode(&previous.root),
            },
            &self.sd,
        )?;
        self.post_with_delivery_id(
            &message,
            format!("{CONTINUATION_DELIVERY_PREFIX}{}", previous.log_uid),
        )
        .await
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
                let client = get_board_client().await?;
                client.append(self.elog_database.as_str(), &ms).await?;
                Ok(())
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
        let error_message = ErrorMessageString(crate::services::privacy_signin::safe_error_text(
            &error_message,
        ));
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
        let mut reader = exported_log_reader(logs_file)?;
        let mut entries = reader
            .deserialize::<ElectoralLogRow>()
            .map(|row| csv_log_entry(row.context("Failed to read electoral-log CSV row")?));
        get_board_client()
            .await?
            .append_iter(&self.elog_database, &mut entries)
            .await?;
        Ok(())
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
    /// Filter only (never a column or a sort key): the first minute of an
    /// inclusive range of `created` (RFC 3339 instant).
    CreatedFrom,
    /// Filter only: the last minute of an inclusive range of `created`.
    CreatedTo,
    /// Filter only: the first minute of a range of `statement_timestamp`.
    StatementTimestampFrom,
    /// Filter only: the last minute of a range of `statement_timestamp`.
    StatementTimestampTo,
    /// Filter only: messages sent to voters on this channel (any case).
    Channel,
    /// Filter only: messages sent to voters with this attempt outcome.
    Outcome,
}

impl OrderField {
    /// The range keys filter a column but aren't one, so they never sort.
    pub fn is_column(&self) -> bool {
        !matches!(
            self,
            OrderField::CreatedFrom
                | OrderField::CreatedTo
                | OrderField::StatementTimestampFrom
                | OrderField::StatementTimestampTo
                | OrderField::Channel
                | OrderField::Outcome
        )
    }
}

/// The start of the minute of a Unix time in seconds. Every zone's offset is
/// a whole number of minutes, so a minute is the same in every zone.
fn minute_start(ts: i64) -> i64 {
    ts - ts.rem_euclid(60)
}

/// Unix seconds of an RFC 3339 instant.
fn parse_instant(value: &str) -> Result<i64> {
    let datetime = ISO8601::to_date_utc(value)
        .map_err(|err| anyhow!("Failed to parse timestamp {value:?}: {err:?}"))?;
    Ok(datetime.timestamp())
}

/// A range of instants inclusive to the minute at both ends: `from` and `to`
/// are RFC 3339 instants (the Admin Portal turns the wall times it shows in
/// the chosen zone into instants), and a time is inside when it falls in or
/// after `from`'s minute and in or before `to`'s minute.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinuteRange {
    /// The first second inside, if bounded.
    pub start: Option<i64>,
    /// The first second after the range, if bounded.
    pub end: Option<i64>,
}

impl MinuteRange {
    pub fn parse(from: Option<&str>, to: Option<&str>) -> Result<Self> {
        let bound = |value: Option<&str>| -> Result<Option<i64>> {
            value
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(parse_instant)
                .transpose()
        };
        Ok(MinuteRange {
            start: bound(from)?.map(minute_start),
            end: bound(to)?.map(|ts| minute_start(ts) + 60),
        })
    }

    /// The minute of one instant (the single-value filter).
    pub fn minute_of(value: &str) -> Result<Self> {
        Self::parse(Some(value), Some(value))
    }

    pub fn contains(&self, ts: i64) -> bool {
        self.start.map_or(true, |start| start <= ts) && self.end.map_or(true, |end| ts < end)
    }

    pub fn is_unbounded(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }

    /// The range as filters of `column`.
    pub fn filters(&self, column: NumberColumn) -> impl Iterator<Item = Filter> {
        let start = self
            .start
            .map(|start| Filter::Number(column, NumberComparison::GreaterThanOrEqual, start));
        let end = self
            .end
            .map(|end| Filter::Number(column, NumberComparison::LessThan, end));
        start.into_iter().chain(end)
    }
}

/// The log a request reads: one election event's, or the tenant's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOf<'a> {
    Event(&'a str),
    Tenant,
}

/// An absent or `null` text as the empty one.
fn text_or_empty<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct GetElectoralLogBody {
    pub tenant_id: String,
    /// Empty, absent or `null` for the tenant's log.
    #[serde(default, deserialize_with = "text_or_empty")]
    pub election_event_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filter: Option<HashMap<OrderField, String>>,
    pub order_by: Option<HashMap<OrderField, OrderDirection>>,
    pub election_id: Option<String>,
    pub area_ids: Option<Vec<String>>,
    pub only_with_user: Option<bool>,
    pub statement_kind: Option<StatementType>,
    /// Elections whose rows may be listed, besides rows of no election. The
    /// server sets it from the caller's permission labels; `None` lists the
    /// rows of every election.
    #[serde(skip)]
    pub permitted_election_ids: Option<Vec<String>>,
    /// Only messages sent to voters, of this channel and outcome, such as
    /// the failed WhatsApp messages the event tab links to. Also given as
    /// the `channel` and `outcome` keys of `filter`.
    #[serde(default)]
    pub communications: Option<CommunicationsFilter>,
}

impl GetElectoralLogBody {
    /// The log the request reads.
    pub fn log(&self) -> LogOf<'_> {
        match self.election_event_id.trim() {
            "" => LogOf::Tenant,
            event => LogOf::Event(event),
        }
    }

    /// The board of the log the request reads.
    pub fn board(&self) -> Result<String> {
        let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
        Ok(match self.log() {
            LogOf::Event(event) => get_event_board(&self.tenant_id, event, &slug),
            LogOf::Tenant => get_tenant_board(&self.tenant_id, &slug),
        })
    }

    /// The communications filter, from `communications` or from the
    /// `channel` and `outcome` keys of `filter`.
    pub fn communications_filter(&self) -> Option<CommunicationsFilter> {
        let from_filter = |field: OrderField| {
            self.filter
                .as_ref()
                .and_then(|filter| filter.get(&field))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let channel = from_filter(OrderField::Channel);
        let outcome = from_filter(OrderField::Outcome);
        match (&self.communications, channel, outcome) {
            (Some(filter), None, None) => Some(filter.clone()),
            (given, channel, outcome) if channel.is_some() || outcome.is_some() => {
                let given = given.clone().unwrap_or_default();
                Some(CommunicationsFilter {
                    channel: channel.or(given.channel),
                    outcome: outcome.or(given.outcome),
                })
            }
            _ => None,
        }
    }
}

/// Selects `SendCommunications` entries by the ledger's current state of
/// the message they logged (see `list_communications`).
#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct CommunicationsFilter {
    /// A channel, in any case (`WHATSAPP` or `whatsapp`), or `none` for
    /// messages recorded without a channel ("No contact channel").
    pub channel: Option<String>,
    /// The message's current outcome: `QUEUED`, `ACCEPTED`, `DELIVERED`,
    /// `FAILED`, `UNKNOWN` (not yet Unresolved) or `UNRESOLVED`.
    pub outcome: Option<String>,
}

impl GetElectoralLogBody {
    fn as_query(&self) -> Result<LogQuery> {
        self.as_query_within(&PgConfig::from_env()?)
    }

    fn as_query_within(&self, limits: &PgConfig) -> Result<LogQuery> {
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
                    // A single value means its minute.
                    OrderField::Created => query
                        .filters
                        .extend(MinuteRange::minute_of(value)?.filters(NumberColumn::Created)),
                    OrderField::StatementTimestamp => query.filters.extend(
                        MinuteRange::minute_of(value)?.filters(NumberColumn::StatementTimestamp),
                    ),
                    OrderField::CreatedFrom => query.filters.extend(
                        MinuteRange::parse(Some(value), None)?.filters(NumberColumn::Created),
                    ),
                    OrderField::CreatedTo => query.filters.extend(
                        MinuteRange::parse(None, Some(value))?.filters(NumberColumn::Created),
                    ),
                    OrderField::StatementTimestampFrom => query.filters.extend(
                        MinuteRange::parse(Some(value), None)?
                            .filters(NumberColumn::StatementTimestamp),
                    ),
                    OrderField::StatementTimestampTo => query.filters.extend(
                        MinuteRange::parse(None, Some(value))?
                            .filters(NumberColumn::StatementTimestamp),
                    ),
                    // Matched on the decoded message (`CommunicationsFilter`).
                    OrderField::Channel | OrderField::Outcome => {}
                    // The statement's own types, matched whole in any case.
                    OrderField::EventType | OrderField::LogType => {
                        let wanted = value.trim().to_uppercase();
                        if !wanted.is_empty() {
                            query.filters.push(Filter::Text(
                                field.to_string().parse()?,
                                SqlCompOperators::Equal,
                                wanted,
                            ));
                        }
                    }
                    OrderField::Description | OrderField::Message => {}
                }
            }
        }
        if self.election_id.is_some() || self.area_ids.is_some() {
            query.visibility = Some(LogVisibility {
                election_id: self.election_id.clone(),
                area_ids: self.area_ids.clone().unwrap_or_default(),
            });
        }
        if let Some(election_ids) = &self.permitted_election_ids {
            query
                .filters
                .push(Filter::ElectionScope(election_ids.clone()));
        }
        if let Some(kind) = &self.statement_kind {
            query.filters.push(Filter::Text(
                ElectoralLogVarCharColumn::StatementKind,
                SqlCompOperators::Equal,
                kind.to_string(),
            ));
        }
        let mut order: Vec<_> = self
            .order_by
            .iter()
            .flatten()
            .filter(|(field, _)| field.is_column())
            .map(|(field, direction)| (field.to_string(), direction.to_string()))
            .collect();
        if !order.is_empty() {
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
        let deserialized_message = Message::record_deserialize(&elog_msg.message)
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

/// At most this many `SendCommunications` entries are read for one page of
/// a communications filter, so a page view costs a bounded amount whatever
/// the event's size.
pub const COMMUNICATIONS_SCAN_LIMIT: usize = 20_000;

impl CommunicationsFilter {
    /// The ledger channel this filter asks for: a channel in any case, or
    /// `none` for messages recorded without a channel ("No contact
    /// channel").
    pub fn ledger_channel(&self) -> Result<ChannelFilter> {
        match self.channel.as_deref().map(str::trim) {
            None | Some("") => Ok(ChannelFilter::Any),
            Some(channel) if channel.eq_ignore_ascii_case("none") => Ok(ChannelFilter::NoChannel),
            Some(channel) => Ok(ChannelFilter::Channel(
                channel
                    .to_uppercase()
                    .parse::<MessageChannel>()
                    .map_err(|_| anyhow!("unknown channel {channel}"))?,
            )),
        }
    }

    /// The ledger row a `SendCommunications` entry logged (`message_id`).
    pub fn message_id_of(row: &ElectoralLogRow) -> Option<String> {
        let message = serde_json::from_str::<serde_json::Value>(&row.message).ok()?;
        let body = message["statement"]["body"]["SendCommunications"].as_str()?;
        let body = serde_json::from_str::<serde_json::Value>(body).ok()?;
        body["message_id"].as_str().map(str::to_string)
    }
}

/// The entries matching a communications filter, by the ledger's
/// **current** state: the latest attempt of each message of the event on
/// the channel and with the outcome asked for (as the event tab counts
/// them), matched to its entry through `message_id`. The total is the
/// number of such messages, the event tab's count. Entries are read a page
/// at a time, at most [`COMMUNICATIONS_SCAN_LIMIT`], and only until the
/// requested page is filled. Entries without a `message_id` (the first
/// release's) don't match.
async fn list_communications(
    input: GetElectoralLogBody,
    filter: CommunicationsFilter,
) -> Result<DataList<ElectoralLogRow>> {
    let tenant_id = uuid::Uuid::parse_str(&input.tenant_id)?;
    let event_id = uuid::Uuid::parse_str(&input.election_event_id)?;
    let channel = filter.ledger_channel()?;
    let outcome = filter
        .outcome
        .as_deref()
        .map(str::trim)
        .filter(|outcome| !outcome.is_empty());
    let wanted: std::collections::HashSet<String> = {
        let mut client = get_hasura_pool().await.get().await?;
        let tx = client.transaction().await?;
        let ids = latest_attempt_ids(
            &tx,
            &tenant_id,
            &event_id,
            channel,
            outcome,
            chrono::Utc::now() - chrono::Duration::hours(UNRESOLVED_AFTER_HOURS),
        )
        .await?;
        tx.commit().await?;
        ids.into_iter().map(|id| id.to_string()).collect()
    };
    let total = i64::try_from(wanted.len())?;
    let page = i64::from(PgConfig::from_env()?.low_sql_limit);
    let wanted_offset = usize::try_from(input.offset.unwrap_or(0).max(0))?;
    let wanted_limit = usize::try_from(
        input
            .limit
            .unwrap_or(PgConfig::from_env()?.default_sql_limit.into())
            .max(0),
    )?;
    let mut matches = vec![];
    let mut offset = 0;
    let mut scanned = 0;
    while !wanted.is_empty()
        && matches.len() < wanted_offset + wanted_limit
        && scanned < COMMUNICATIONS_SCAN_LIMIT
    {
        let batch = Box::pin(list_electoral_log(GetElectoralLogBody {
            statement_kind: Some(StatementType::SendCommunications),
            communications: None,
            filter: input.filter.clone().map(|mut filter| {
                filter.remove(&OrderField::Channel);
                filter.remove(&OrderField::Outcome);
                filter
            }),
            limit: Some(page),
            offset: Some(offset),
            ..input.clone()
        }))
        .await?;
        let read = batch.items.len();
        scanned += read;
        matches.extend(batch.items.into_iter().filter(|row| {
            CommunicationsFilter::message_id_of(row).is_some_and(|id| wanted.contains(&id))
        }));
        if (read as i64) < page {
            break;
        }
        offset += page;
    }
    if scanned >= COMMUNICATIONS_SCAN_LIMIT {
        warn!(scanned, "communications filter stopped at its scan limit");
    }
    Ok(DataList {
        items: matches
            .into_iter()
            .skip(wanted_offset)
            .take(wanted_limit)
            .collect(),
        total: TotalAggregate {
            aggregate: Aggregate { count: total },
        },
    })
}

#[instrument(skip_all)]
pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
    if let Some(filter) = input.communications_filter() {
        return list_communications(input, filter).await;
    }
    let client = get_board_client().await?;
    let board = input.board()?;
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

#[instrument(skip_all)]
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
#[instrument(skip_all)]
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

#[instrument(skip_all)]
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
        let databases = crate::services::electoral_log_board::get_event_databases().await?;
        databases.create_event(&tenant, &event).await?;
        let client = get_board_client().await?;
        client.create_board(&board).await?;
        let key = SigningKey::generate()?;
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
            tenant_id: tenant.clone(),
            election_event_id: event.clone(),
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
        let store = databases.store(&event).await?;
        let checkpoint = store.journal().checkpoint(&board).await?;
        assert_eq!(checkpoint.tree_size, 1);
        let report = store.audit(&board, &[checkpoint]).await?;
        assert!(report.is_clean(), "{:?}", report.findings());
        client.delete_board(&board).await?;
        assert_eq!(count_electoral_log(input).await?, 0);
        databases.drop_event(&tenant, &event).await?;
        assert!(!databases.has_event(&event).await?);
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

#[cfg(test)]
mod election_published_tests {
    use super::*;

    fn log() -> ElectoralLog {
        let key = SigningKey::generate().unwrap();
        ElectoralLog::for_system_with_signing_key("board", &key)
    }

    fn message(configuration: Option<PublishedConfiguration>) -> Message {
        log()
            .election_published_message(
                "event".to_string(),
                Some(vec!["election".to_string()]),
                "publication".to_string(),
                configuration,
                Some("admin-id".to_string()),
                Some("admin".to_string()),
            )
            .unwrap()
    }

    #[test]
    fn a_signed_configurations_publication_is_one_entry_with_its_manifest_and_designs() {
        let configuration = PublishedConfiguration {
            external_id: "ov-2028".to_string(),
            revision: 3,
            manifest_sha256: "ab".repeat(32),
            design_digests: vec![ConfigurationDesignDigest {
                area: "Post 1".to_string(),
                election: "national".to_string(),
                sha256: "cd".repeat(32),
            }],
        };
        let message = message(Some(configuration.clone()));
        assert!(matches!(
            message.statement.head.kind,
            StatementType::ElectionPublish
        ));
        match &message.statement.body {
            StatementBody::ElectionPublishWithConfiguration(election, publication, logged) => {
                assert_eq!(election.0.as_deref(), Some("election"));
                assert_eq!(publication.0, "publication");
                assert_eq!(logged, &configuration);
            }
            other => panic!("unexpected body {other:?}"),
        }
        assert_eq!(message.user_id.as_deref(), Some("admin-id"));
        assert_eq!(message.username.as_deref(), Some("admin"));
        assert_eq!(message.election_id.as_deref(), Some("election"));

        let stored = ElectoralLogMessage::try_from(&message).unwrap();
        assert_eq!(stored.statement_kind, "ElectionPublish");
        let read = Message::record_deserialize(&stored.message).unwrap();
        assert!(matches!(
            read.statement.body,
            StatementBody::ElectionPublishWithConfiguration(_, _, logged) if logged == configuration
        ));
    }

    #[test]
    fn a_publication_without_a_package_is_the_entry_older_logs_hold() {
        let message = message(None);
        assert!(matches!(
            message.statement.head.kind,
            StatementType::ElectionPublish
        ));
        assert!(matches!(
            message.statement.body,
            StatementBody::ElectionPublish(..)
        ));
    }
}

#[cfg(test)]
mod range_filter_tests {
    use super::*;
    use electoral_log::domain::{OrderColumn, SortDirection};

    fn ts(rfc3339: &str) -> i64 {
        ISO8601::to_date_utc(rfc3339).unwrap().timestamp()
    }

    fn body(filter: &[(OrderField, &str)]) -> GetElectoralLogBody {
        GetElectoralLogBody {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            filter: Some(
                filter
                    .iter()
                    .map(|(field, value)| (field.clone(), value.to_string()))
                    .collect(),
            ),
            ..Default::default()
        }
    }

    /// The time filters of the body's query, as (column, comparison, value).
    fn time_filters(body: &GetElectoralLogBody) -> Vec<(String, String, i64)> {
        let mut filters: Vec<_> = body
            .as_query_within(&PgConfig::default())
            .unwrap()
            .filters
            .into_iter()
            .filter_map(|filter| match filter {
                Filter::Number(column, comparison, value) => {
                    Some((column.to_string(), comparison.to_string(), value))
                }
                Filter::Text(..) | Filter::ElectionScope(..) => None,
            })
            .collect();
        filters.sort();
        filters
    }

    fn filter(column: &str, comparison: &str, value: i64) -> (String, String, i64) {
        (column.to_string(), comparison.to_string(), value)
    }

    /// Two configurations: the wall times are entered in the chosen zone
    /// (Manila, +08:00; Kathmandu, +05:45) and arrive as instants.
    #[test]
    fn the_range_includes_both_ends_to_the_minute_in_the_chosen_zone() {
        for offset in ["+08:00", "+05:45"] {
            let from = format!("2028-04-08T00:00:00{offset}");
            let to = format!("2028-04-09T23:59:00{offset}");
            let range = MinuteRange::parse(Some(&from), Some(&to)).unwrap();

            assert!(range.contains(ts(&from)));
            assert!(range.contains(ts(&format!("2028-04-09T23:59:59{offset}"))));
            assert!(!range.contains(ts(&format!("2028-04-10T00:00:00{offset}"))));
            assert!(!range.contains(ts(&format!("2028-04-07T23:59:59{offset}"))));
        }
    }

    #[test]
    fn seconds_in_the_bounds_still_cover_their_whole_minute() {
        let range = MinuteRange::parse(
            Some("2028-04-08T10:15:42+08:00"),
            Some("2028-04-08T10:20:05+08:00"),
        )
        .unwrap();
        assert!(range.contains(ts("2028-04-08T10:15:00+08:00")));
        assert!(range.contains(ts("2028-04-08T10:20:59+08:00")));
        assert!(!range.contains(ts("2028-04-08T10:14:59+08:00")));
        assert!(!range.contains(ts("2028-04-08T10:21:00+08:00")));
    }

    #[test]
    fn an_open_end_is_unbounded() {
        let range = MinuteRange::parse(Some("2028-04-08T10:15:00Z"), None).unwrap();
        assert!(range.contains(i64::MAX));
        assert!(!range.contains(ts("2028-04-08T10:14:59Z")));
        assert!(MinuteRange::parse(None, Some(" ")).unwrap().is_unbounded());
        assert_eq!(
            MinuteRange::parse(None, Some(" "))
                .unwrap()
                .filters(NumberColumn::Created)
                .count(),
            0
        );
    }

    #[test]
    fn the_list_filters_created_from_and_to_inclusive_to_the_minute() {
        let filters = time_filters(&body(&[
            (OrderField::CreatedFrom, "2028-04-08T00:00:00+08:00"),
            (OrderField::CreatedTo, "2028-04-09T23:59:00+08:00"),
        ]));

        assert_eq!(
            filters,
            vec![
                filter("created", "<", ts("2028-04-09T16:00:00Z")),
                filter("created", ">=", ts("2028-04-07T16:00:00Z")),
            ]
        );
    }

    #[test]
    fn the_list_filters_the_statement_timestamp_range() {
        let filters = time_filters(&body(&[
            (
                OrderField::StatementTimestampFrom,
                "2028-04-08T09:30:00-04:00",
            ),
            (
                OrderField::StatementTimestampTo,
                "2028-04-08T09:30:00-04:00",
            ),
        ]));

        let start = ts("2028-04-08T13:30:00Z");
        assert_eq!(
            filters,
            vec![
                filter("statement_timestamp", "<", start + 60),
                filter("statement_timestamp", ">=", start),
            ]
        );
    }

    #[test]
    fn a_single_value_still_means_its_minute() {
        let filters = time_filters(&body(&[(OrderField::Created, "2028-04-08T10:15:42Z")]));
        assert_eq!(
            filters,
            vec![
                filter("created", "<", ts("2028-04-08T10:16:00Z")),
                filter("created", ">=", ts("2028-04-08T10:15:00Z")),
            ]
        );
    }

    #[test]
    fn an_unreadable_bound_is_refused() {
        let body = body(&[(OrderField::CreatedFrom, "yesterday")]);
        assert!(body.as_query_within(&PgConfig::default()).is_err());
    }

    #[test]
    fn the_range_keys_arrive_in_the_filter_and_never_sort() {
        let body: GetElectoralLogBody = serde_json::from_value(serde_json::json!({
            "tenant_id": "tenant",
            "election_event_id": "event",
            "filter": {"created_from": "2028-04-08T00:00:00+08:00"},
            "order_by": {"created_from": "desc", "statement_timestamp_to": "asc"},
        }))
        .unwrap();
        assert_eq!(
            time_filters(&body),
            vec![filter("created", ">=", ts("2028-04-07T16:00:00Z"))]
        );
        let query = body.as_query_within(&PgConfig::default()).unwrap();
        assert_eq!(
            format!("{:?}", query.order),
            format!("{:?}", LogQuery::default().order)
        );
    }

    #[test]
    fn a_range_key_in_the_sort_is_left_out() {
        let body: GetElectoralLogBody = serde_json::from_value(serde_json::json!({
            "tenant_id": "tenant",
            "election_event_id": "event",
            "order_by": {"created_to": "desc", "created": "asc"},
        }))
        .unwrap();
        let query = body.as_query_within(&PgConfig::default()).unwrap();
        assert_eq!(
            format!("{:?}", query.order),
            format!("{:?}", vec![(OrderColumn::Created, SortDirection::Asc)])
        );
    }
}

#[cfg(test)]
mod csv_import_tests {
    use super::*;
    use voting_crypto::RecordSerialize;

    fn row(message: &Message, statement_kind: &str) -> ElectoralLogRow {
        ElectoralLogRow {
            id: 12,
            created: 1_841_000_123,
            statement_timestamp: message.statement.head.timestamp as i64,
            statement_kind: statement_kind.to_string(),
            message: String::new(),
            data: general_purpose::STANDARD_NO_PAD.encode(message.record_serialize().unwrap()),
            user_id: None,
            username: None,
        }
    }

    fn checkpoint_message() -> Message {
        let key = SigningKey::generate().unwrap();
        let log = ElectoralLog::for_system_with_signing_key("board", &key);
        Message::electoral_log_checkpoint_message(
            EventIdString("event".to_string()),
            ElectoralLogCheckpointV2 {
                log_uid: Uuid::new_v4().hyphenated().to_string(),
                tree_size: 4,
                root: "ab".repeat(32),
                reason: ElectoralLogCheckpointReason::Periodic,
            },
            &log.sd,
        )
        .unwrap()
    }

    #[test]
    fn a_row_keeps_its_signed_bytes_and_creation_time() {
        let message = checkpoint_message();
        let entry = csv_log_entry(row(&message, "ElectoralLogCheckpoint")).unwrap();
        assert_eq!(entry.message.statement_kind, "ElectoralLogCheckpoint");
        assert_eq!(entry.message.created, 1_841_000_123);
        assert_eq!(entry.message.message, message.record_serialize().unwrap());
        assert!(!entry.delivery_id.is_empty());
    }

    /// An export written when the monitoring, signing and later statements
    /// came straight after `ExternalReconciliation`: its rows name the
    /// statement they held, which their messages no longer read as.
    #[test]
    fn a_row_numbered_by_another_release_is_refused() {
        let message = checkpoint_message();
        let error = csv_log_entry(row(&message, "SigningRequestCreated"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("SigningRequestCreated"), "{error}");
        assert!(error.contains("ElectoralLogCheckpoint"), "{error}");
    }

    #[test]
    fn a_row_whose_message_does_not_read_is_refused() {
        let mut row = row(&checkpoint_message(), "ElectoralLogCheckpoint");
        row.data = general_purpose::STANDARD_NO_PAD.encode([30u8, 1, 2, 3]);
        assert!(csv_log_entry(row).is_err());
    }
}

#[cfg(test)]
mod delivery_id_tests {
    use super::*;

    fn monitoring_change(
        generation: u64,
        mode: MonitoringDashboardMode,
    ) -> MonitoringConfigChangeDetails {
        MonitoringConfigChangeDetails {
            origin: MonitoringConfigOrigin::Editor,
            preset: None,
            mode,
            generation,
            revisions: vec![],
        }
    }

    #[test]
    fn each_writer_names_what_it_records() {
        assert_eq!(
            approval_matrix_delivery_id(3, &"ab".repeat(32)),
            format!("approval-matrix:3:{}", "ab".repeat(32))
        );
        let package = ConfigurationPackageDetails {
            action: ConfigurationPackageAction::Imported,
            external_id: "ov-2028".to_string(),
            revision: 4,
            manifest_sha256: "cd".repeat(32),
            design_digests: vec![],
        };
        assert_eq!(
            configuration_package_delivery_id(&package),
            format!("configuration-package:ov-2028:4:{}", "cd".repeat(32))
        );
        let report = ReportGeneratedDetails {
            report_type: "ACTIVITY_LOGS".to_string(),
            document_id: None,
            report_manifest_sha256: "ef".repeat(32),
            external_id: "ov-2028".to_string(),
            revision: 4,
            manifest_sha256: "cd".repeat(32),
        };
        assert_eq!(
            report_generated_delivery_id(&report),
            format!("report-generated:{}", "ef".repeat(32))
        );
    }

    #[test]
    fn a_monitoring_change_is_named_by_its_generation_and_content() {
        let saved = monitoring_delivery(7, MonitoringDashboardMode::Configured);
        assert!(saved.starts_with("monitoring-config:7:"), "{saved}");
        assert_eq!(
            saved,
            monitoring_delivery(7, MonitoringDashboardMode::Configured)
        );
        assert_ne!(
            saved,
            monitoring_delivery(7, MonitoringDashboardMode::Legacy)
        );
        assert_ne!(
            saved,
            monitoring_delivery(8, MonitoringDashboardMode::Configured)
        );
    }

    fn monitoring_delivery(generation: u64, mode: MonitoringDashboardMode) -> String {
        monitoring_config_delivery_id(&monitoring_change(generation, mode)).unwrap()
    }
}

#[cfg(test)]
mod communications_filter_tests {
    use super::CommunicationsFilter;

    #[test]
    fn communications_select_a_ledger_channel_or_no_channel() {
        use crate::postgres::messaging::ChannelFilter;
        use sequent_core::types::messaging::MessageChannel;
        let filter = |channel: Option<&str>| CommunicationsFilter {
            channel: channel.map(str::to_string),
            outcome: Some("FAILED".to_string()),
        };
        assert_eq!(
            filter(Some("whatsapp")).ledger_channel().unwrap(),
            ChannelFilter::Channel(MessageChannel::WHATSAPP)
        );
        assert_eq!(
            filter(Some("NONE")).ledger_channel().unwrap(),
            ChannelFilter::NoChannel
        );
        assert_eq!(filter(None).ledger_channel().unwrap(), ChannelFilter::Any);
        assert!(filter(Some("fax")).ledger_channel().is_err());
    }

    #[test]
    fn an_entry_is_matched_to_its_ledger_row() {
        let body = r#"{"channel":"whatsapp","message_id":"m-1","outcome":"ACCEPTED"}"#;
        let row = super::ElectoralLogRow {
            id: 1,
            created: 0,
            statement_timestamp: 0,
            statement_kind: "SendCommunications".to_string(),
            message: serde_json::json!({"statement": {"body": {"SendCommunications": body}}})
                .to_string(),
            data: String::new(),
            user_id: None,
            username: None,
        };
        assert_eq!(
            CommunicationsFilter::message_id_of(&row).as_deref(),
            Some("m-1")
        );
        let first_release = super::ElectoralLogRow {
            message: serde_json::json!({"statement": {"body": {"SendCommunications":
                r#"{"channel":"sms","state":"ACCEPTED"}"#}}})
            .to_string(),
            ..row
        };
        assert_eq!(CommunicationsFilter::message_id_of(&first_release), None);
    }
}

#[cfg(test)]
mod permitted_election_tests {
    use super::*;

    fn body(permitted_election_ids: Option<Vec<String>>) -> GetElectoralLogBody {
        GetElectoralLogBody {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            permitted_election_ids,
            ..Default::default()
        }
    }

    /// The election scopes of the body's query.
    fn election_scopes(body: &GetElectoralLogBody) -> Vec<Vec<String>> {
        body.as_query_within(&PgConfig::default())
            .unwrap()
            .filters
            .into_iter()
            .filter_map(|filter| match filter {
                Filter::ElectionScope(elections) => Some(elections),
                Filter::Text(..) | Filter::Number(..) => None,
            })
            .collect()
    }

    #[test]
    fn log_query_keeps_rows_of_no_election_and_of_permitted_elections() {
        let elections = vec!["north-1".to_string(), "north-2".to_string()];
        assert_eq!(election_scopes(&body(Some(elections.clone()))), [elections]);
    }

    #[test]
    fn log_query_keeps_rows_of_no_election_when_no_election_is_permitted() {
        assert_eq!(election_scopes(&body(Some(vec![]))), [Vec::<String>::new()]);
    }

    #[test]
    fn log_query_scope_is_combined_with_the_request_filters() {
        let mut input = body(Some(vec!["north-1".to_string()]));
        input.election_id = Some("north-1".to_string());
        let query = input.as_query_within(&PgConfig::default()).unwrap();
        assert_eq!(election_scopes(&input), [vec!["north-1".to_string()]]);
        assert_eq!(
            query
                .visibility
                .and_then(|visibility| visibility.election_id),
            Some("north-1".to_string())
        );
    }

    #[test]
    fn log_query_is_unchanged_when_every_election_is_permitted() {
        let query = body(None).as_query_within(&PgConfig::default()).unwrap();
        assert!(query.filters.is_empty());
        assert!(query.visibility.is_none());
    }

    #[test]
    fn request_body_cannot_set_the_permitted_elections() {
        let input: GetElectoralLogBody = serde_json::from_value(serde_json::json!({
            "tenant_id": "tenant",
            "election_event_id": "event",
            "permitted_election_ids": ["south-1"]
        }))
        .unwrap();
        assert_eq!(input.permitted_election_ids, None);
    }
}
