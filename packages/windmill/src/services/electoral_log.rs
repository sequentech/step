// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::celery_app::get_celery_app;
use crate::services::database::PgConfig;
use crate::services::insert_cast_vote::hash_voter_id;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::protocol_manager::{create_named_param, get_board_client, get_immudb_client};
use crate::services::vault;
use crate::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventInput, INTERNAL_MESSAGE_TYPE,
};
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::{anyhow, ensure, Context, Result};
use b3::messages::message::Signer;
use base64::engine::general_purpose;
use base64::Engine;
use deadpool_postgres::Transaction;
use electoral_log::client::types::*;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::*;
use electoral_log::messages::statement::{StatementBody, StatementType};
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, TxMode};
use rust_decimal::prelude::ToPrimitive;
use sequent_core::serialization::deserialize_with_path;
use serde::{Deserialize, Serialize};
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::HashWrapper;
use strand::hash::STRAND_HASH_LENGTH_BYTES;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use tempfile::NamedTempFile;
use tokio_stream::StreamExt;
use tracing::{info, instrument, warn};

pub const IMMUDB_ROWS_LIMIT: usize = 2500;
pub const MAX_ROWS_PER_PAGE: usize = 50;

/// Ballot_id input is the first half of the original hash which is stored in the electoral log.
pub const BALLOT_ID_LENGTH_BYTES: usize = STRAND_HASH_LENGTH_BYTES / 2;
/// Ballot_id input is in HEX, each byte is represented in 2 chars.
pub const BALLOT_ID_LENGTH_CHARS: usize = BALLOT_ID_LENGTH_BYTES * 2;

pub struct ElectoralLog {
    pub(crate) sd: SigningData,
    pub(crate) elog_database: String,
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
        voter_signing_key: &Option<StrandSignatureSk>,
    ) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(event_id),
            elog_database,
        )
        .await?;
        let system_sk = protocol_manager.get_signing_key().clone();

        let sk = voter_signing_key.clone().unwrap_or(system_sk.clone());

        Ok(ElectoralLog {
            sd: SigningData::new(sk, user_id, system_sk),
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
    #[instrument(err)]
    pub async fn for_admin_user(
        hasura_transaction: &Transaction<'_>,
        elog_database: &str,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        username: Option<String>,
        elections_ids: Option<String>,
        user_area_id: Option<String>,
    ) -> Result<Self> {
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
            elections_ids,
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
            message_type: INTERNAL_MESSAGE_TYPE.into(),
            user_id: Some(user_id.to_string()),
            username: None,
            tenant_id: tenant_id.to_string(),
            body: serde_json::to_string(&board_message)
                .with_context(|| "Error serializing ElectoralLogMessage")?,
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
            message_type: INTERNAL_MESSAGE_TYPE.into(),
            user_id: Some(voter_id),
            username: voter_username,
            tenant_id,
            body: serde_json::to_string(&board_message)
                .with_context(|| "Error serializing ElectoralLogMessage")?,
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
            message_type: INTERNAL_MESSAGE_TYPE.into(),
            user_id: Some(voter_id),
            username: voter_username.clone(),
            tenant_id,
            body: serde_json::to_string(&board_message)
                .with_context(|| "Error serializing post cast vote")?,
        };
        let celery_app = get_celery_app().await;
        celery_app
            .send_task(enqueue_electoral_log_event::new(input))
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn post_election_published(
        &self,
        event_id: String,
        election_id: Option<String>,
        ballot_pub_id: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id.clone());
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
        elections_ids: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);

        let message =
            Message::key_insertion_start(event, &self.sd, user_id, username, elections_ids)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion(
        &self,
        event_id: String,
        trustee_name: String,
        user_id: Option<String>,
        username: Option<String>,
        elections_ids: String,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let trustee_name = TrusteeNameString(trustee_name);

        let message = Message::key_insertion_message(
            event,
            trustee_name,
            &self.sd,
            user_id,
            username,
            Some(elections_ids),
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_open(
        &self,
        event_id: String,
        election_id: Option<String>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::tally_open_message(event, election, &self.sd, user_id, username)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub(crate) async fn post_tally_close(
        &self,
        event_id: String,
        election_id: Option<String>,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

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

    async fn post(&self, message: &Message) -> Result<()> {
        let board_message: ElectoralLogMessage = message.try_into()?;
        let ms = vec![board_message];

        let mut client = get_board_client().await?;
        client
            .insert_electoral_log_messages(self.elog_database.as_str(), &ms)
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StatementHeadDataString {
    pub event: String,
    pub kind: String,
    pub timestamp: i64,
    pub event_type: String,
    pub log_type: String,
    pub description: String,
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

// Removing this step would inprove the performance, i.e. return the final type directly from ElectoralLogMessage.
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
        let message: serde_json::Value = deserialize_with_path::deserialize_str(&self.message)
            .map_err(|err| {
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

        let data: StatementHeadDataString = deserialize_with_path::deserialize_value(head.clone())
            .map_err(|err| anyhow!(format!("{:?}, Failed to parse head: {}", err, head)))?;

        Ok(data)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CastVoteEntry {
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub ballot_id: String,
    pub username: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CastVoteMessagesOutput {
    pub list: Vec<CastVoteEntry>,
    pub total: usize,
}

impl CastVoteEntry {
    pub fn from_elog_message(
        entry: &ElectoralLogMessage,
        input_username: &str,
    ) -> Result<Option<Self>, anyhow::Error> {
        let ballot_id = entry.ballot_id.clone().unwrap_or_default();
        let username = entry.username.clone().filter(|s| s.eq(input_username)); // Keep other usernames anonymous on the table
        Ok(Some(CastVoteEntry {
            statement_timestamp: entry.statement_timestamp,
            statement_kind: StatementType::CastVote.to_string(),
            ballot_id,
            username,
        }))
    }
}

#[instrument(err)]
pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
    let mut client = get_board_client().await?;
    let board_name = get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());
    info!("database name = {board_name}");
    let cols_match_select = input.as_where_clause_map()?;
    let order_by = input.order_by.clone();
    let (min_ts, max_ts) = input.get_min_max_ts()?;
    let limit: i64 = input.limit.unwrap_or(IMMUDB_ROWS_LIMIT as i64);
    let offset: i64 = input.offset.unwrap_or(0);
    let mut rows: Vec<ElectoralLogRow> = vec![];
    let electoral_log_messages = client
        .get_electoral_log_messages_filtered(
            &board_name,
            Some(cols_match_select.clone()),
            min_ts,
            max_ts,
            Some(limit),
            Some(offset),
            order_by.clone(),
        )
        .await
        .map_err(|err| anyhow!("Failed to get filtered messages: {:?}", err))?;

    let t_entries = electoral_log_messages.len();
    info!("Got {t_entries} entries. Offset: {offset}, limit: {limit}");
    for message in electoral_log_messages {
        rows.push(message.try_into()?);
    }

    Ok(DataList {
        items: rows,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: t_entries as i64,
            },
        },
    })
}

/// Returns the entries for statement_kind = "CastVote" which ballot_id matches the input.
/// ballot_id_filter is restricted to be an even number of characters, so thatnit can be converted
/// to a byte array.
#[instrument(err, skip_all)]
pub async fn list_cast_vote_messages_and_count(
    input: GetElectoralLogBody,
    ballot_id_filter: &str,
    user_id: &str,
    username: &str,
) -> Result<CastVoteMessagesOutput> {
    ensure!(
        ballot_id_filter.chars().count() % 2 == 0 && ballot_id_filter.is_ascii(),
        "Incorrect ballot_id, the length must be an even number of characters"
    );
    let election_id = input.election_id.clone().unwrap_or_default();
    let (cols_match_count, cols_match_select) =
        input.as_cast_vote_count_and_select_clauses(&election_id, user_id, ballot_id_filter);

    let (data_res, count_res) = tokio::join!(
        list_cast_vote_messages(
            input.clone(),
            ballot_id_filter,
            user_id,
            username,
            cols_match_select
        ),
        async {
            let mut client = get_board_client().await?;
            let board_name =
                get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());
            info!("database name = {board_name}");
            let total: usize = client
                .count_electoral_log_messages(&board_name, Some(cols_match_count))
                .await?
                .to_usize()
                .unwrap_or(0);
            Ok(total)
        }
    );

    let mut data = data_res.map_err(|e| anyhow!("Eror listing electoral log: {e:?}"))?;
    data.total =
        count_res.map_err(|e: anyhow::Error| anyhow!("Error counting electoral log: {e:?}"))?;

    Ok(data)
}

#[instrument(err)]
pub async fn list_cast_vote_messages(
    input: GetElectoralLogBody,
    ballot_id_filter: &str,
    user_id: &str,
    username: &str,
    cols_match_select: WhereClauseOrdMap,
) -> Result<CastVoteMessagesOutput> {
    // The limits are used to cut the output after filtering the ballot id.
    // Because ballot_id cannot be filtered at SQL level the sql limit is constant
    let output_limit: i64 = input.limit.unwrap_or(MAX_ROWS_PER_PAGE as i64);
    let board_name = get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());
    info!("database name = {board_name}");
    let order_by = input.order_by.clone();

    let limit: i64 = match ballot_id_filter.is_empty() {
        false => IMMUDB_ROWS_LIMIT as i64, // When there is a filter, need to fetch all entries by batches.
        true => input.limit.unwrap_or(MAX_ROWS_PER_PAGE as i64),
    };
    let mut offset: i64 = input.offset.unwrap_or(0);
    let mut list: Vec<CastVoteEntry> = Vec::with_capacity(MAX_ROWS_PER_PAGE); // Filtered messages.

    let mut client = get_board_client().await?;
    let mut exit = false; // Exit at the first match if the filter is not empty or when the query returns 0 entries
    while (list.len() as i64) < output_limit && !exit {
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
        info!("Got {t_entries} entries. Offset: {offset}, limit: {limit}");
        for message in electoral_log_messages.iter() {
            match CastVoteEntry::from_elog_message(&message, username)? {
                Some(entry) if !ballot_id_filter.is_empty() => {
                    // If there is filter exit at the first match
                    exit = true;
                    list.push(entry);
                }
                Some(entry) => {
                    // Add all the entries till the limit, when there is no filter
                    list.push(entry);
                }
                None => {}
            }
            if (list.len() as i64) >= output_limit || exit {
                break;
            }
        }
        exit = exit || t_entries == 0;
        offset += limit;
    }
    Ok(CastVoteMessagesOutput { list, total: 0 })
}

#[instrument(err)]
pub async fn count_electoral_log(input: GetElectoralLogBody) -> Result<i64> {
    let mut client = get_board_client().await?;
    let board_name = get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());
    info!("database name = {board_name}");
    let cols_match_count = input.as_where_clause_map()?;
    let total = client
        .count_electoral_log_messages(&board_name, Some(cols_match_count))
        .await?
        .to_u64()
        .unwrap_or(0) as i64;
    Ok(total)
}
