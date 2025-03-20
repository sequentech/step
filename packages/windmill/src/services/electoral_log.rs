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
use crate::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};
use anyhow::{anyhow, Context, Result};
use b3::messages::message::{self, Signer as _};
use base64::engine::general_purpose;
use base64::Engine;
use deadpool_postgres::Transaction;
use electoral_log::assign_value;
use electoral_log::messages::message::Message;
use electoral_log::messages::message::SigningData;
use electoral_log::messages::newtypes::ErrorMessageString;
use electoral_log::messages::newtypes::KeycloakEventTypeString;
use electoral_log::messages::newtypes::*;
use electoral_log::messages::statement::StatementHead;
use electoral_log::ElectoralLogMessage;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue, TxMode};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use sequent_core::serialization::deserialize_with_path;
use sequent_core::services::date::ISO8601;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::HashWrapper;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use strum_macros::{Display, EnumString, ToString};
use tempfile::NamedTempFile;
use tokio_stream::{Stream, StreamExt};
use tracing::{event, info, instrument, Level};
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
    pub async fn post_external_api_request(
        &self,
        tenant_id: String,
        event_id: String,
        election_id: Option<String>,
        voter_id: String,
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
            Some(voter_id.clone()),
            voter_username.clone(),
            direction,
            api_name,
            operation,
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

// Enumeration for the valid fields in the immudb table
#[derive(Debug, Deserialize, Hash, PartialEq, Eq, EnumString, Display)]
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
    SenderPk,
    LogType,
    EventType,
    Description,
    Version,
}

#[derive(Deserialize, Debug)]
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
                    OrderField::SenderPk | OrderField::UserId | OrderField::Username | OrderField::StatementKind | OrderField::Version => { // sql VARCHAR type
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

        if !extra_where_clauses.is_empty() {
            match clauses.len() {
                0 => {
                    clauses.push(format!("WHERE {}", extra_where_clauses.join(" AND ")));
                }
                _ => {
                    let where_clause = clauses.pop().unwrap();
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
                .unwrap()
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
            let offset = std::cmp::max(self.offset.unwrap(), 0);
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

pub const IMMUDB_ROWS_LIMIT: usize = 25_000;

#[instrument(err)]
pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
    let mut client: Client = get_immudb_client().await?;
    let board_name = get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());

    event!(Level::INFO, "database name = {board_name}");
    info!("input = {:?}", input);
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

#[instrument(err)]
pub async fn count_electoral_log(input: GetElectoralLogBody) -> Result<i64> {
    let mut client = get_immudb_client().await?;
    let board_name = get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());

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
