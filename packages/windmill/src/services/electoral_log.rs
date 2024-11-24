// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::PgConfig;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::protocol_manager::{create_named_param, get_board_client, get_immudb_client};
use crate::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose;
use base64::Engine;

use electoral_log::messages::message::Message;
use electoral_log::messages::message::SigningData;
use electoral_log::messages::newtypes::ErrorMessageString;
use electoral_log::messages::newtypes::KeycloakEventTypeString;
use electoral_log::messages::newtypes::*;
use electoral_log::messages::statement::StatementHead;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::serialization::deserialize_with_path;
use strand::hash::HashWrapper;

use crate::services::insert_cast_vote::hash_voter_id;
use crate::services::protocol_manager::get_event_board;
use crate::services::vault;
use b3::messages::message::{self, Signer as _};
use electoral_log::assign_value;
use electoral_log::ElectoralLogMessage;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue};
use sequent_core::serialization::base64::{Base64Deserialize, Base64Serialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use strum_macros::{Display, EnumString, ToString};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
pub struct ElectoralLog {
    pub(crate) sd: SigningData,
    pub(crate) elog_database: String,
}

impl ElectoralLog {
    #[instrument(err)]
    pub async fn new(elog_database: &str) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;

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
    pub async fn new_from_sk(elog_database: &str, sender_sk: &StrandSignatureSk) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;
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
    #[instrument(err)]
    pub async fn for_voter(
        elog_database: &str,
        tenant_id: &str,
        event_id: &str,
        user_id: &str,
    ) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;
        let system_sk = protocol_manager.get_signing_key().clone();

        let sk = vault::get_voter_signing_key(elog_database, tenant_id, event_id, user_id).await?;

        Ok(ElectoralLog {
            sd: SigningData::new(sk, "", system_sk),
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
        elog_database: &str,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;
        let system_sk = protocol_manager.get_signing_key().clone();

        let sk = vault::get_admin_user_signing_key(elog_database, tenant_id, user_id).await?;

        Ok(ElectoralLog {
            sd: SigningData::new(sk, "", system_sk),
            elog_database: elog_database.to_string(),
        })
    }

    /// Posts a voter's public key
    #[instrument(err)]
    pub async fn post_voter_pk(
        elog_database: &str,
        tenant_id: &str,
        event_id: &str,
        user_id: &str,
        pk_der_b64: &str,
    ) -> Result<()> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;
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
        )?;

        let elog = ElectoralLog {
            sd,
            elog_database: elog_database.to_string(),
        };

        let ret = elog.post(&message).await;

        if ret.is_err() {
            tracing::error!(
                "Unable to post public key for voter {:?}, {:?}",
                message,
                ret
            );
        }

        ret
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
        elog_database: &str,
        tenant_id: &str,
        user_id: &str,
        pk_der_b64: &str,
    ) -> Result<()> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;
        let system_sk = protocol_manager.get_signing_key().clone();
        let sd = SigningData::new(system_sk.clone(), "", system_sk.clone());

        let message = Message::admin_public_key_message(
            TenantIdString(tenant_id.to_string()),
            Some(user_id.to_string()),
            PublicKeyDerB64(pk_der_b64.to_string()),
            &sd,
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
        event_id: String,
        election_id: Option<String>,
        pseudonym_h: PseudonymHash,
        vote_h: CastVoteHash,
        voter_ip: String,
        voter_country: String,
        voter_id: String,
    ) -> Result<()> {
        let event = EventIdString(event_id);
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
            Some(voter_id),
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self, pseudonym_h))]
    pub async fn post_cast_vote_error(
        &self,
        event_id: String,
        election_id: Option<String>,
        pseudonym_h: PseudonymHash,
        error: String,
        voter_ip: String,
        voter_country: String,
        voter_id: String,
    ) -> Result<()> {
        let event = EventIdString(event_id);
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
            Some(voter_id),
        )?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_published(
        &self,
        event_id: String,
        election_id: Option<String>,
        ballot_pub_id: String,
        user_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);
        let ballot_pub_id = BallotPublicationIdString(ballot_pub_id);

        let message =
            Message::election_published_message(event, election, ballot_pub_id, &self.sd, user_id)?;

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
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = election_id.map(|id| ElectionIdString(Some(id)));

        let message =
            Message::election_pause_message(event, election, voting_channel, &self.sd, user_id)?;

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
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let error_message = ErrorMessageString(error_message);
        let event_type = KeycloakEventTypeString(event_type);
        let message =
            Message::keycloak_user_event(event, event_type, error_message, user_id, &self.sd)?;
        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_keygen(&self, event_id: String, user_id: Option<String>) -> Result<()> {
        let event = EventIdString(event_id);

        let message = Message::keygen_message(event, &self.sd, user_id)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion_start(
        &self,
        event_id: String,
        user_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);

        let message = Message::key_insertion_start(event, &self.sd, user_id)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion(
        &self,
        event_id: String,
        trustee_name: String,
        user_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let trustee_name = TrusteeNameString(trustee_name);

        let message = Message::key_insertion_message(event, trustee_name, &self.sd, user_id)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_open(
        &self,
        event_id: String,
        election_id: Option<String>,
        user_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::tally_open_message(event, election, &self.sd, user_id)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub(crate) async fn post_tally_close(
        &self,
        event_id: String,
        election_id: Option<String>,
        user_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::tally_close_message(event, election, &self.sd, user_id)?;

        self.post(&message).await
    }

    #[instrument(skip(self))]
    pub async fn post_send_template(
        &self,
        message: Option<String>,
        event_id: String,
        user_id: Option<String>,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::send_template(event, election, &self.sd, user_id, message)
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

    #[instrument(skip(self))]
    pub async fn import_from_csv(&self, logs_file: &NamedTempFile) -> Result<()> {
        let mut client = get_board_client().await?;
        let mut rows = Vec::new();

        let mut rdr = csv::Reader::from_reader(logs_file);

        for result in rdr.deserialize() {
            info!("FFFF {:?}", result);
            let row: ElectoralLogRow =
                result.map_err(|err| anyhow::Error::new(err).context("Failed to read CSV row"))?;
            rows.push(row);
        }

        for log in rows {
            let message: &Message =
                &Message::strand_deserialize(&general_purpose::STANDARD_NO_PAD.decode(&log.data)?)
                    .map_err(|err| anyhow!("Failed to deserialize message: {:?}", err))?;
            let electoral_log_message: ElectoralLogMessage = message.try_into()?;

            client
                .insert_electoral_log_messages(
                    self.elog_database.as_str(),
                    &vec![electoral_log_message],
                )
                .await
                .map_err(|err| anyhow!("Failed to insert log message: {:?}", err))?;
        }

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
}

#[derive(Deserialize, Debug)]
pub struct GetElectoralLogBody {
    pub tenant_id: String,
    pub election_event_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filter: Option<HashMap<OrderField, String>>,
    pub order_by: Option<HashMap<OrderField, OrderDirection>>,
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
                    OrderField::Id => {
                        let int_value: i64 = value.parse()?;
                        where_clauses.push(format!("id = @{}", param_name));
                        params.push(create_named_param(param_name, Value::N(int_value)));
                    }
                    OrderField::StatementTimestamp | OrderField::Created | OrderField::Message => {}
                    _ => {
                        where_clauses.push(format!("{field} LIKE @{}", param_name));
                        params.push(create_named_param(param_name, Value::S(value.to_string())));
                    }
                }
            }

            if !where_clauses.is_empty() {
                clauses.push(format!("WHERE {}", where_clauses.join(" AND ")));
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectoralLogRow {
    pub id: i64,
    pub created: i64,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: String,
    pub data: String,
    pub user_id: Option<String>,
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
        })
    }
}

pub const IMMUDB_ROWS_LIMIT: usize = 2500;

#[instrument(err)]
pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
    let mut client = get_immudb_client().await?;
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
            user_id
        FROM electoral_log_messages
        {clauses}
        "#,
    );
    let sql_query_response = client.sql_query(&sql, params).await?;
    let items = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(ElectoralLogRow::try_from)
        .collect::<Result<Vec<ElectoralLogRow>>>()?;

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
        items: items,
        total: TotalAggregate {
            aggregate: aggregate,
        },
    })
}
