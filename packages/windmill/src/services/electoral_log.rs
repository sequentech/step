// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, PgConfig};
use crate::services::election_event_board::get_election_event_board;
use crate::services::insert_cast_vote::hash_voter_id;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::protocol_manager::{create_named_param, get_board_client, get_immudb_client};
use crate::services::vault;
use crate::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
};
use crate::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};
use anyhow::{anyhow, ensure, Context, Result};
use b3::messages::message::Signer;
use base64::engine::general_purpose;
use base64::Engine;
use deadpool_postgres::Transaction;
use electoral_log::assign_value;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{CertificateAuthEventAction, *};
use electoral_log::messages::statement::{StatementBody, StatementType};
use electoral_log::{
    ElectoralLogMessage, ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, TxMode};
use rust_decimal::prelude::ToPrimitive;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::date::ISO8601;
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
use tokio_stream::StreamExt;
use tracing::{event, info, instrument, warn, Level};

pub const IMMUDB_ROWS_LIMIT: usize = 2500;
pub const MAX_ROWS_PER_PAGE: usize = 50;

/// Ballot_id input is the first half of the original hash which is stored in the electoral log.
pub const BALLOT_ID_LENGTH_BYTES: usize = STRAND_HASH_LENGTH_BYTES / 2;
/// Ballot_id input is in HEX, each byte is represented in 2 chars.
pub const BALLOT_ID_LENGTH_CHARS: usize = BALLOT_ID_LENGTH_BYTES * 2;

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
    message: ElectoralLogMessage,
}

impl PreparedVoterPasswordChangeLog {
    #[instrument(skip(self), err)]
    pub async fn post(&self) -> Result<()> {
        retry_with_exponential_backoff(
            || async {
                let mut client = get_board_client().await?;
                let mut columns_matcher = WhereClauseBTreeMap::new();
                columns_matcher.insert(
                    ElectoralLogVarCharColumn::StatementKind,
                    (SqlCompOperators::Equal, self.message.statement_kind.clone()),
                );
                columns_matcher.insert(
                    ElectoralLogVarCharColumn::SenderPk,
                    (SqlCompOperators::Equal, self.message.sender_pk.clone()),
                );
                columns_matcher.insert(
                    ElectoralLogVarCharColumn::Version,
                    (SqlCompOperators::Equal, self.message.version.clone()),
                );
                if let Some(user_id) = &self.message.user_id {
                    columns_matcher.insert(
                        ElectoralLogVarCharColumn::UserId,
                        (SqlCompOperators::Equal, user_id.clone()),
                    );
                }

                let existing = client
                    .get_electoral_log_messages_filtered::<String, String>(
                        &self.board,
                        Some(columns_matcher),
                        Some(self.message.created),
                        Some(self.message.created),
                        Some(100),
                        None,
                        None,
                    )
                    .await?;
                if existing
                    .iter()
                    .any(|candidate| same_electoral_log_message(candidate, &self.message))
                {
                    return Ok(());
                }

                client
                    .insert_electoral_log_messages(&self.board, &vec![self.message.clone()])
                    .await
            },
            5,
            Duration::from_millis(100),
        )
        .await
    }
}

fn same_electoral_log_message(left: &ElectoralLogMessage, right: &ElectoralLogMessage) -> bool {
    left.created == right.created
        && left.sender_pk == right.sender_pk
        && left.statement_timestamp == right.statement_timestamp
        && left.statement_kind == right.statement_kind
        && left.message == right.message
        && left.version == right.version
        && left.user_id == right.user_id
        && left.username == right.username
        && left.election_id == right.election_id
        && left.area_id == right.area_id
        && left.ballot_id == right.ballot_id
}

/// Prepares a signed password-change audit entry using the caller's Hasura
/// transaction. The caller can therefore commit its document and durable task
/// state before performing the external Immudb write.
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

    Ok(PreparedVoterPasswordChangeLog { board, message })
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

    #[instrument(skip(self, pseudonym_h, vote_h))]
    pub async fn post_cast_vote(
        &self,
        tenant_id: String,
        event_id: String,
        election_id: Option<String>,
        pseudonym_h: PseudonymHash,
        vote_h: CastVoteHash,
        voter_ip: String,
        voter_country: String,
        voter_id: String,
        voter_username: Option<String>,
        area_id: String,
    ) -> Result<()> {
        let event = EventIdString(event_id.clone());
        let election = ElectionIdString(election_id);
        let ip = VoterIpString(voter_ip);
        let country = VoterCountryString(voter_country);

        let message = Message::cast_vote_message(
            event,
            election,
            pseudonym_h,
            vote_h,
            &self.sd,
            ip,
            country,
            Some(voter_id.clone()),
            voter_username.clone(),
            area_id,
        )?;
        let board_message: ElectoralLogMessage = (&message).try_into().with_context(|| {
            "Error converting Message::cast_vote_message into ElectoralLogMessage"
        })?;
        let input = LogEventInput {
            election_event_id: event_id,
            message_type: LogMessageType::Internal,
            user_id: Some(voter_id),
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

    #[instrument(skip(self), err)]
    async fn post(&self, message: &Message) -> Result<()> {
        let board_message: ElectoralLogMessage = message.try_into()?;
        let ms = vec![board_message];

        retry_with_exponential_backoff(
            // The closure we want to call repeatedly
            || async {
                let mut client = get_board_client().await?;
                client
                    .insert_electoral_log_messages(self.elog_database.as_str(), &ms)
                    .await
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
        let batch_size: usize = PgConfig::from_env()?.default_sql_batch_size.try_into()?;
        let mut rdr = csv::Reader::from_reader(logs_file);

        let mut client = get_board_client().await?;
        client.open_session(self.elog_database.as_str()).await?;
        let tx = client.new_tx(TxMode::ReadWrite).await?;

        // Allocate a vector with capacity equal to the batch size.
        let mut messages: Vec<ElectoralLogMessage> = Vec::with_capacity(batch_size);

        for result in rdr.deserialize() {
            let row: ElectoralLogRow =
                result.map_err(|err| anyhow::Error::new(err).context("Failed to read CSV row"))?;
            let message: &Message =
                &Message::strand_deserialize(&general_purpose::STANDARD_NO_PAD.decode(&row.data)?)
                    .map_err(|err| anyhow!("Failed to deserialize message: {:?}", err))?;
            let electoral_log_message: ElectoralLogMessage = message.try_into()?;
            messages.push(electoral_log_message);

            // Once we reach the batch size, flush the batch.
            if messages.len() >= batch_size {
                client
                    .insert_electoral_log_messages_batch(&tx, &messages)
                    .await?;
                messages.clear();
            }
        }

        // Flush any remaining messages that didn't complete a full batch.
        if !messages.is_empty() {
            client
                .insert_electoral_log_messages_batch(&tx, &messages)
                .await?;
        }

        client.commit(&tx).await?;
        client.close_session().await?;
        Ok(())
    }
}

// Enumeration for the valid fields in the immudb table
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
    // Returns the SQL clauses related to the request along with the parameters
    #[instrument(ret)]
    fn as_sql(&self, to_count: bool) -> Result<(String, Vec<NamedParam>)> {
        let mut clauses = Vec::new();
        let mut params = Vec::new();

        // Handle filters
        if let Some(filters_map) = &self.filter {
            let mut where_clauses = Vec::new();

            for (field, value) in filters_map {
                info!("field = ?: {field}, value = ?: {value}");
                let param_name = format!("param_{field}");
                match field {
                    OrderField::Id => { // sql INTEGER type
                        let int_value: i64 = value.parse()?;
                        where_clauses.push(format!("id = @{}", param_name));
                        params.push(create_named_param(param_name, Value::N(int_value)));
                    }
                    OrderField::SenderPk | OrderField::UserId | OrderField::Username | OrderField::BallotId | OrderField::StatementKind | OrderField::Version => { // sql VARCHAR type
                        where_clauses.push(format!("{field} LIKE @{}", param_name));
                        params.push(create_named_param(param_name, Value::S(value.to_string())));
                    }
                    OrderField::StatementTimestamp | OrderField::Created => { // sql TIMESTAMP type
                        // these have their own column and are inside of Message´s column as well
                        let datetime = ISO8601::to_date_utc(&value)
                            .map_err(|err| anyhow!("Failed to parse timestamp: {:?}", err))?;
                        let ts: i64 = datetime.timestamp();
                        let ts_end: i64 = ts + 60; // Search along that minute, the second is not specified by the front.
                        let param_name_end = format!("{param_name}_end");
                        where_clauses.push(format!("{field} >= @{} AND {field} < @{}", param_name, param_name_end));
                        params.push(create_named_param(param_name, Value::Ts(ts)));
                        params.push(create_named_param(param_name_end, Value::Ts(ts_end)));
                    }
                    OrderField::EventType | OrderField::LogType | OrderField::Description // these have no column but are inside of Message
                    | OrderField::Message => {} // Message column is sql BLOB type and it´s encrypted so we can't search it without expensive operations
                }
            }

            if !where_clauses.is_empty() {
                clauses.push(format!("WHERE {}", where_clauses.join(" AND ")));
            }
        };

        // Build a single extra clause.
        // This clause returns rows if:
        // - @election_filter is non-empty and matches election_id, OR
        // - @area_filter is non-empty and matches area_id, OR
        // - Both election_id and area_id are either '' or NULL. (General to all elections log)
        let mut extra_where_clauses = Vec::new();
        if self.election_id.is_some() || self.area_ids.is_some() {
            let mut conds = Vec::new();

            if let Some(election) = &self.election_id {
                if !election.is_empty() {
                    params.push(create_named_param(
                        "param_election".to_string(),
                        Value::S(election.clone()),
                    ));
                    conds.push("election_id LIKE @param_election".to_string());
                }
            }

            if let Some(area_ids) = &self.area_ids {
                if !area_ids.is_empty() {
                    let placeholders: Vec<String> = area_ids
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("@param_area{}", i))
                        .collect();
                    for (i, area) in area_ids.into_iter().enumerate() {
                        let param_name = format!("param_area{}", i);
                        params.push(create_named_param(
                            param_name.clone(),
                            Value::S(area.clone()),
                        ));
                    }
                    conds.push(format!(
                        "(@param_area0 <> '' AND area_id IN ({}))",
                        placeholders.join(", ")
                    ));
                }
            }

            // if neither filter matches, return logs where both fields are empty or NULL.
            conds.push(
                "((election_id = '' OR election_id IS NULL) AND (area_id = '' OR area_id IS NULL))"
                    .to_string(),
            );

            extra_where_clauses.push(format!("({})", conds.join(" OR ")));
        }

        // Handle only_with_user
        if self.only_with_user.unwrap_or(false) {
            extra_where_clauses.push("(user_id IS NOT NULL AND user_id <> '')".to_string());
        }

        // Handle
        if let Some(statement_kind) = &self.statement_kind {
            params.push(create_named_param(
                "param_statement_kind".to_string(),
                Value::S(statement_kind.to_string()),
            ));
            extra_where_clauses.push("(statement_kind = @param_statement_kind)".to_string());
        }

        if !extra_where_clauses.is_empty() {
            match clauses.len() {
                0 => {
                    clauses.push(format!("WHERE {}", extra_where_clauses.join(" AND ")));
                }
                _ => {
                    let where_clause = clauses.pop().ok_or(anyhow!("Empty clause"))?;
                    clauses.push(format!(
                        "{} AND {}",
                        where_clause,
                        extra_where_clauses.join(" AND ")
                    ));
                }
            }
        }

        // Handle order_by
        if !to_count && self.order_by.is_some() {
            let order_by_clauses: Vec<String> = self
                .order_by
                .as_ref()
                .ok_or(anyhow!("Empty order clause"))?
                .iter()
                .map(|(field, direction)| format!("{field} {direction}"))
                .collect();
            if order_by_clauses.len() > 0 {
                clauses.push(format!("ORDER BY {}", order_by_clauses.join(", ")));
            }
        }

        // Handle limit
        if !to_count {
            let limit_param_name = String::from("limit");
            let limit_value = self
                .limit
                .unwrap_or(PgConfig::from_env()?.default_sql_limit.into());
            let limit = std::cmp::min(limit_value, PgConfig::from_env()?.low_sql_limit.into());
            clauses.push(format!("LIMIT @{limit_param_name}"));
            params.push(create_named_param(limit_param_name, Value::N(limit)));
        }

        // Handle offset
        if !to_count && self.offset.is_some() {
            let offset_param_name = String::from("offset");
            let offset = std::cmp::max(self.offset.unwrap_or(0), 0);
            clauses.push(format!("OFFSET @{}", offset_param_name));
            params.push(create_named_param(offset_param_name, Value::N(offset)));
        }

        Ok((clauses.join(" "), params))
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

impl TryFrom<&Row> for ElectoralLogRow {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut id = 0;
        let mut created: i64 = 0;
        let mut sender_pk = String::from("");
        let mut statement_timestamp: i64 = 0;
        let mut statement_kind = String::from("");
        let mut message = vec![];
        let mut user_id = None;
        let mut username = None;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            match column.as_str() {
                c if c.ends_with(".id)") => {
                    assign_value!(Value::N, value, id)
                }
                c if c.ends_with(".created)") => {
                    assign_value!(Value::Ts, value, created)
                }
                c if c.ends_with(".sender_pk)") => {
                    assign_value!(Value::S, value, sender_pk)
                }
                c if c.ends_with(".statement_timestamp)") => {
                    assign_value!(Value::Ts, value, statement_timestamp)
                }
                c if c.ends_with(".statement_kind)") => {
                    assign_value!(Value::S, value, statement_kind)
                }
                c if c.ends_with(".message)") => {
                    assign_value!(Value::Bs, value, message)
                }
                c if c.ends_with(".user_id)") => match value.value.as_ref() {
                    Some(Value::S(inner)) => user_id = Some(inner.clone()),
                    Some(Value::Null(_)) => user_id = None,
                    None => user_id = None,
                    _ => return Err(anyhow!("invalid column value for 'user_id'")),
                },
                c if c.ends_with(".username)") => match value.value.as_ref() {
                    Some(Value::S(inner)) => username = Some(inner.clone()),
                    Some(Value::Null(_)) => username = None,
                    None => username = None,
                    _ => return Err(anyhow!("invalid column value for 'username'")),
                },
                _ => return Err(anyhow!("invalid column found '{}'", column.as_str())),
            }
        }

        let deserialized_message =
            Message::strand_deserialize(&message).with_context(|| "Error deserializing message")?;
        let serialized = general_purpose::STANDARD_NO_PAD.encode(message);
        Ok(ElectoralLogRow {
            id,
            created,
            statement_timestamp,
            statement_kind,
            message: serde_json::to_string_pretty(&deserialized_message)
                .with_context(|| "Error serializing message to json")?,
            data: serialized,
            user_id,
            username,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CastVoteEntry {
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub ballot_id: String,
    pub username: Option<String>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CastVoteMessagesOutput {
    pub list: Vec<CastVoteEntry>,
    pub total: usize,
}

impl CastVoteEntry {
    pub fn from_elog_message(entry: &ElectoralLogMessage) -> Result<Option<Self>, anyhow::Error> {
        let ballot_id = entry.ballot_id.clone().unwrap_or_default();
        let username = entry.username.clone();
        let message: &Message = &Message::strand_deserialize(&entry.message)
            .map_err(|err| anyhow!("Failed to deserialize message: {:?}", err))?;
        let message = Some(message.to_string());

        Ok(Some(CastVoteEntry {
            statement_timestamp: entry.statement_timestamp,
            statement_kind: StatementType::CastVote.to_string(),
            ballot_id,
            username,
            message,
        }))
    }
}

#[instrument(err)]
pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
    let mut client: Client = get_immudb_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(
        input.tenant_id.as_str(),
        input.election_event_id.as_str(),
        &slug,
    );

    event!(Level::INFO, "database name = {board_name}");
    client.open_session(&board_name).await?;
    let (clauses, params) = input.as_sql(false)?;
    let (clauses_to_count, count_params) = input.as_sql(true)?;
    info!("clauses ?:= {clauses}");
    let sql = format!(
        r#"
        SELECT
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            user_id,
            username
        FROM electoral_log_messages
        {clauses}
        "#,
    );
    info!("query: {sql}");
    let sql_query_response = client.streaming_sql_query(&sql, params).await?;

    let limit: usize = input.limit.unwrap_or(IMMUDB_ROWS_LIMIT as i64).try_into()?;
    info!("list_electoral_log: limit = {}", limit);
    let mut rows: Vec<ElectoralLogRow> = Vec::with_capacity(limit);
    let mut resp_stream = sql_query_response.into_inner();
    while let Some(streaming_batch) = resp_stream.next().await {
        let items = streaming_batch?
            .rows
            .iter()
            .map(ElectoralLogRow::try_from)
            .collect::<Result<Vec<ElectoralLogRow>>>()?;
        rows.extend(items);
    }

    let sql = format!(
        r#"
        SELECT
            COUNT(*)
        FROM electoral_log_messages
        {clauses_to_count}
        "#,
    );
    let sql_query_response = client.sql_query(&sql, count_params).await?;
    let mut rows_iter = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(Aggregate::try_from);

    let aggregate = rows_iter
        // get the first item
        .next()
        // unwrap the Result and Option
        .ok_or(anyhow!("No aggregate found"))??;

    client.close_session().await?;
    Ok(DataList {
        items: rows,
        total: TotalAggregate {
            aggregate: aggregate,
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
        false => IMMUDB_ROWS_LIMIT as i64, // When there is a filter, need to fetch all entries by batches.
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
            match CastVoteEntry::from_elog_message(&message)? {
                Some(entry) if !ballot_id_filter.is_empty() => {
                    // If there is filter exit at the first match
                    filter_matched = true;
                    list.push(entry);
                }
                Some(entry) => {
                    // Add all the entries till the limit, when there is no filter
                    list.push(entry);
                }
                None => {}
            }
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
    let mut client = get_immudb_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(
        input.tenant_id.as_str(),
        input.election_event_id.as_str(),
        &slug,
    );

    info!("board name: {board_name}");
    client.open_session(&board_name).await?;

    let (clauses_to_count, count_params) = input.as_sql(true)?;
    let sql = format!(
        r#"
        SELECT COUNT(*)
        FROM electoral_log_messages
        {clauses_to_count}
        "#,
    );

    info!("query: {sql}");

    let sql_query_response = client.sql_query(&sql, count_params).await?;

    let mut rows_iter = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(Aggregate::try_from);
    let aggregate = rows_iter
        .next()
        .ok_or_else(|| anyhow!("No aggregate found"))??;

    client.close_session().await?;
    Ok(aggregate.count as i64)
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
            message: prepared_message(),
        };

        let serialized = serde_json::to_value(&prepared).unwrap();
        let restored: PreparedVoterPasswordChangeLog = serde_json::from_value(serialized).unwrap();

        assert_eq!(restored, prepared);
    }

    #[test]
    fn duplicate_detection_ignores_immudb_row_id_but_compares_signed_message() {
        let prepared = prepared_message();
        let mut inserted = prepared.clone();
        inserted.id = 42;

        assert!(same_electoral_log_message(&inserted, &prepared));

        inserted.message.push(5);
        assert!(!same_electoral_log_message(&inserted, &prepared));
    }
}
