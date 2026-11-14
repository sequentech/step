// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::services::celery_app::get_celery_connection;
use crate::services::celery_app::Queue;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::database::PgConfig;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_outbox::relay_configured_log_events;
use crate::services::electoral_log_queue::{
    drain_log_event_queue, identify_log_events, store_log_events, AmqpLogEventQueue, BoardDelivery,
    EventOutcome, LogEventPreparation, LogEventStore,
};
use crate::services::protocol_manager::{get_board_client, get_protocol_manager};
use crate::services::users::get_user_area_id;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use b3::messages::message::Signer;
use celery::error::TaskError;
use dashmap::DashMap;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::client::board_client::{BoardClient, DeliveryReceipt, ElectoralLogMessage};
use immudb_rs::TxMode;
use once_cell::sync::Lazy;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use std::future::Future;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignaturePk;
use tracing::{info, instrument, warn};

/// Classifies the type of an incoming log event.
///
/// Serializes as a plain string for wire compatibility:
/// - `Internal`            → `"internal"`
/// - `KeycloakEvent(s)`   → `s` (the raw Keycloak event type, e.g. `"LOGIN"`)
#[derive(Clone, Debug, PartialEq)]
pub enum LogMessageType {
    Internal,
    KeycloakEvent(String),
}

impl Serialize for LogMessageType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LogMessageType::Internal => serializer.serialize_str("internal"),
            LogMessageType::KeycloakEvent(event_type) => serializer.serialize_str(event_type),
        }
    }
}

impl<'de> Deserialize<'de> for LogMessageType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "internal" => LogMessageType::Internal,
            _ => LogMessageType::KeycloakEvent(s),
        })
    }
}

/// Represents the typed content of a log event body.
///
/// Serializes as a plain string for wire compatibility:
/// - `Communications(msg)` → `"communications <msg>"`
/// - `Plain(s)`            → `s`
#[derive(Clone, Debug, PartialEq)]
pub enum LogEventBody {
    /// Body from a send-template action; contains the template message.
    Communications(String),
    /// Body from a standard Keycloak event; typically the error field or "null".
    Plain(String),
}

impl LogEventBody {
    pub fn as_raw(&self) -> String {
        match self {
            LogEventBody::Communications(msg) => format!("communications {}", msg),
            LogEventBody::Plain(s) => s.clone(),
        }
    }
}

impl Serialize for LogEventBody {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_raw())
    }
}

impl<'de> Deserialize<'de> for LogEventBody {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(if let Some(msg) = s.strip_prefix("communications ") {
            LogEventBody::Communications(msg.trim().to_string())
        } else {
            LogEventBody::Plain(s)
        })
    }
}

/// Represents an incoming log event.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEventInput {
    pub election_event_id: String,
    pub message_type: LogMessageType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub tenant_id: String,
    pub body: LogEventBody,
}

/// Enqueue the electoral log event.
/// This task is routed to the durable electoral_log_batch_queue.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn enqueue_electoral_log_event(input: LogEventInput) -> Result<()> {
    // By calling this task, the event is enqueued into the electoral_log_batch_queue.
    Ok(())
}

/// Legacy wire signature retained for batches queued by an earlier version.
/// The dispatcher drains its queue and acknowledges a batch once all of its
/// events are stored; workers are redirected away from that queue, which is
/// the only one this task is routed to. Should a worker still consume it, the
/// events are written under the same identities, but Celery acknowledges the
/// message even when this task fails, so the dispatcher is what keeps them.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(bind = true, max_retries = 0)]
pub async fn process_electoral_log_events_batch(
    task: &Self,
    events: Vec<LogEventInput>,
) -> Result<()> {
    let events = identify_log_events(&task.request.correlation_id, true, events)?;
    let outcomes = store_log_events(&mut ElectoralLogBoards, events).await?;
    let failed = outcomes
        .iter()
        .filter(|outcome| !matches!(outcome, EventOutcome::Stored))
        .count();
    if failed > 0 {
        return Err(anyhow!("{failed} electoral log events of the batch were not stored").into());
    }
    Ok(())
}

/// Parses the row an internal producer queued and returns it rebuilt from its
/// signed message, checked with the election event's system key.
fn verified_internal_message(
    input: &LogEventInput,
    system_pk: &StrandSignaturePk,
) -> anyhow::Result<ElectoralLogMessage> {
    let row: ElectoralLogMessage = deserialize_str(&input.body.as_raw())
        .with_context(|| "Error parsing input.body into a ElectoralLogMessage")?;
    row.rebuild_verified(system_pk, &input.election_event_id)
}

/// The public key of each board's protocol manager (by tenant, election event
/// and board), which signs the board's internal messages, as last read. A message that does not verify against
/// the cached key is checked again with the key read anew, which replaces the
/// cached one, so a board recreated under the same name with a new protocol
/// manager is picked up without a restart.
static BOARD_SYSTEM_PKS: Lazy<DashMap<(String, String, String), StrandSignaturePk>> =
    Lazy::new(DashMap::new);

fn board_key(tenant_id: &str, election_event_id: &str, board: &str) -> (String, String, String) {
    (
        tenant_id.to_string(),
        election_event_id.to_string(),
        board.to_string(),
    )
}

/// Reads the public key of the protocol manager that signs the internal
/// messages of a board and caches it in place of the board's cached key.
async fn board_system_pk(
    hasura_tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &str,
) -> anyhow::Result<StrandSignaturePk> {
    let protocol_manager =
        get_protocol_manager::<RistrettoCtx>(hasura_tx, tenant_id, Some(election_event_id), board)
            .await
            .with_context(|| "Error getting protocol manager")?;
    let system_pk = StrandSignaturePk::from_sk(protocol_manager.get_signing_key())
        .with_context(|| "Error getting protocol manager public key")?;
    BOARD_SYSTEM_PKS.insert(
        board_key(tenant_id, election_event_id, board),
        system_pk.clone(),
    );
    Ok(system_pk)
}

/// Checks a message with the cached key and, when there is no cached key or
/// the message does not verify against it, with the current key that
/// `read_current_key` reads. The outer error is that read's, so the event is
/// retried; the inner result is the check's.
async fn verify_with_cached_or_current_key<K, T, Fut>(
    cached_key: Option<K>,
    read_current_key: impl FnOnce() -> Fut,
    verify: impl Fn(&K) -> anyhow::Result<T>,
) -> anyhow::Result<anyhow::Result<T>>
where
    Fut: Future<Output = anyhow::Result<K>>,
{
    if let Some(cached_key) = cached_key {
        if let Ok(verified) = verify(&cached_key) {
            return Ok(Ok(verified));
        }
    }
    let current_key = read_current_key().await?;
    Ok(verify(&current_key))
}

/// Builds the entries of one event. Errors from the databases or the vault are
/// returned as errors so the event is retried; an event that can never be
/// written is rejected, including an internal message that does not verify
/// against its election event's system key.
async fn prepare_log_event(
    hasura_tx: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    input: &LogEventInput,
) -> anyhow::Result<LogEventPreparation<ElectoralLogMessage>> {
    if let Err(err) =
        parse_uuid_v4(&input.tenant_id).and_then(|_| parse_uuid_v4(&input.election_event_id))
    {
        return Ok(LogEventPreparation::Rejected(err));
    }
    let Some(election_event) =
        get_election_event_by_id_if_exist(hasura_tx, &input.tenant_id, &input.election_event_id)
            .await
            .with_context(|| "Error getting election event")?
    else {
        return Ok(LogEventPreparation::Rejected(anyhow!(
            "Election event {} not found",
            input.election_event_id
        )));
    };
    let Some(board) = get_election_event_board(election_event.bulletin_board_reference.clone())
    else {
        return Ok(LogEventPreparation::Rejected(anyhow!(
            "Election event {} has no board",
            input.election_event_id
        )));
    };

    let messages = match &input.message_type {
        LogMessageType::Internal => {
            let cached_pk = BOARD_SYSTEM_PKS
                .get(&board_key(&input.tenant_id, &election_event.id, &board))
                .map(|system_pk| system_pk.value().clone());
            verify_with_cached_or_current_key(
                cached_pk,
                || board_system_pk(hasura_tx, &input.tenant_id, &election_event.id, &board),
                |system_pk: &StrandSignaturePk| verified_internal_message(input, system_pk),
            )
            .await?
            .map(|message| vec![message])
        }
        LogMessageType::KeycloakEvent(event_type) => {
            let user_id = input
                .user_id
                .clone()
                .unwrap_or_else(|| "unknown_user".into());
            let realm = get_event_realm(&input.tenant_id, &input.election_event_id);
            let user_area_id = get_user_area_id(keycloak_transaction, &realm, &user_id)
                .await
                .with_context(|| "Error getting user area id")?;
            let electoral_log = ElectoralLog::new(
                hasura_tx,
                &input.tenant_id,
                Some(&election_event.id),
                &board,
            )
            .await
            .with_context(|| "Error initializing electoral log")?;
            build_keycloak_event_messages(&electoral_log, input, event_type, user_id, user_area_id)
        }
    };

    let messages = messages.and_then(|messages| {
        messages
            .iter()
            .try_for_each(ElectoralLogMessage::check_column_limits)?;
        Ok(messages)
    });
    Ok(match messages {
        Ok(messages) => LogEventPreparation::Ready { board, messages },
        Err(err) => LogEventPreparation::Rejected(err),
    })
}

fn build_keycloak_event_messages(
    electoral_log: &ElectoralLog,
    input: &LogEventInput,
    event_type: &str,
    user_id: String,
    user_area_id: Option<String>,
) -> anyhow::Result<Vec<ElectoralLogMessage>> {
    let mut messages = Vec::new();
    if let LogEventBody::Communications(ref template_body) = input.body {
        messages.push(
            electoral_log
                .build_send_template_message(
                    Some(template_body.clone()),
                    input.election_event_id.clone(),
                    Some(user_id.clone()),
                    input.username.clone(),
                    None,
                    user_area_id.clone(),
                )
                .with_context(|| "Error building send template message")?,
        );
    }
    messages.push(
        electoral_log
            .build_keycloak_event_message(
                input.election_event_id.clone(),
                event_type.to_string(),
                input.body.as_raw(),
                Some(user_id),
                input.username.clone(),
                user_area_id,
            )
            .with_context(|| "Error building keycloak event message")?,
    );
    Ok(messages)
}

// A communications delivery writes two audit rows plus its receipt. Keeping
// sixteen whole deliveries per transaction leaves room for immudb's SQL indexes.
const ELECTORAL_LOG_DELIVERIES_PER_TRANSACTION: usize = 16;

/// Savepoint under which one event of a batch is prepared.
const EVENT_SAVEPOINT: &str = "electoral_log_event";

/// Prepares each event under its own savepoints of both transactions: a failed
/// query rolls back only its event's savepoint, which is retained, and the
/// events after it are still prepared.
pub(crate) async fn prepare_each<M>(
    hasura_tx: &mut Transaction<'_>,
    keycloak_transaction: &mut Transaction<'_>,
    events: &[LogEventInput],
    mut prepare: impl AsyncFnMut(
        &Transaction<'_>,
        &Transaction<'_>,
        &LogEventInput,
    ) -> anyhow::Result<LogEventPreparation<M>>,
) -> anyhow::Result<Vec<LogEventPreparation<M>>> {
    let mut preparations = Vec::with_capacity(events.len());
    for input in events {
        let hasura_event = hasura_tx
            .savepoint(EVENT_SAVEPOINT)
            .await
            .with_context(|| "Error starting a Hasura savepoint")?;
        let keycloak_event = keycloak_transaction
            .savepoint(EVENT_SAVEPOINT)
            .await
            .with_context(|| "Error starting a keycloak savepoint")?;
        match prepare(&hasura_event, &keycloak_event, input).await {
            Ok(preparation) => {
                keycloak_event.commit().await?;
                hasura_event.commit().await?;
                preparations.push(preparation);
            }
            Err(err) => {
                keycloak_event.rollback().await?;
                hasura_event.rollback().await?;
                preparations.push(LogEventPreparation::Retained(err));
            }
        }
    }
    Ok(preparations)
}

/// Writes electoral log events to the board of their election event.
struct ElectoralLogBoards;

#[async_trait]
impl LogEventStore for ElectoralLogBoards {
    type Message = ElectoralLogMessage;

    /// Uses a single Hasura transaction to fetch the event details of the
    /// whole batch.
    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> anyhow::Result<Vec<LogEventPreparation<ElectoralLogMessage>>> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting DB pool for batch processing")?;
        let mut hasura_tx = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting Hasura transaction")?;

        let mut keycloak_db_client: DbClient = get_keycloak_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting keycloak DB pool for batch processing")?;
        let mut keycloak_transaction = keycloak_db_client
            .transaction()
            .await
            .with_context(|| "Error starting keycloak transaction")?;

        let preparations = prepare_each(
            &mut hasura_tx,
            &mut keycloak_transaction,
            events,
            async |hasura, keycloak, input| prepare_log_event(hasura, keycloak, input).await,
        )
        .await?;

        hasura_tx
            .commit()
            .await
            .with_context(|| "Error committing Hasura transaction")?;
        Ok(preparations)
    }

    async fn persist(
        &mut self,
        board: &str,
        deliveries: Vec<BoardDelivery<ElectoralLogMessage>>,
    ) -> anyhow::Result<Vec<DeliveryReceipt>> {
        let mut board_client = get_board_client().await?;
        persist_board_deliveries(&mut board_client, board, &deliveries).await
    }
}

/// Commits the deliveries of one board a few at a time. A failure after some
/// transactions are committed leaves every delivery of the board in the
/// queue; their receipts make writing the committed ones again a no-op.
pub async fn persist_board_deliveries(
    board_client: &mut BoardClient,
    board: &str,
    deliveries: &[BoardDelivery<ElectoralLogMessage>],
) -> anyhow::Result<Vec<DeliveryReceipt>> {
    board_client.open_session(board).await?;
    let result = async {
        board_client
            .ensure_electoral_log_delivery_receipts()
            .await?;
        let mut receipts = Vec::with_capacity(deliveries.len());
        for chunk in deliveries.chunks(ELECTORAL_LOG_DELIVERIES_PER_TRANSACTION) {
            let immudb_tx = board_client.new_tx(TxMode::ReadWrite).await?;
            for delivery in chunk {
                receipts.push(
                    board_client
                        .insert_electoral_log_delivery(
                            &immudb_tx,
                            &delivery.delivery_id,
                            &delivery.payload_hash,
                            &delivery.messages,
                        )
                        .await
                        .with_context(|| {
                            format!("Error inserting electoral log delivery for board {board}")
                        })?,
                );
            }
            board_client.commit(&immudb_tx).await.with_context(|| {
                format!("Error committing immudb transaction for board {board}")
            })?;
        }
        anyhow::Ok(receipts)
    }
    .await;
    // A failed close must neither hide a failed write nor put committed
    // entries back in the queue.
    if let Err(err) = board_client.close_session().await {
        warn!(%board, error = ?err, "Error closing immudb session");
    }
    result
}

/// Publishes the signed CastVotes that accepted votes left in the electoral
/// log outbox, for example because RabbitMQ was unavailable after the vote
/// was committed.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
// No expiry: a worker may take this task late, behind a long dispatcher run,
// and an expired task is acknowledged without running. A late or overlapping
// relay finds nothing to do or takes other rows under their locks.
#[celery::task(max_retries = 0)]
pub async fn relay_electoral_log_outbox() -> Result<()> {
    relay_configured_log_events().await?;
    Ok(())
}

/// Dispatcher: reads batches of events from the electoral_log_event_queue, and
/// batches queued by earlier versions from the electoral_log_batch_queue,
/// writes them to their boards and acknowledges each delivery only once its
/// entries are committed. Events that can never be written are moved to the
/// electoral_log_dead_letter_queue.
#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
// No time_limit: a batch stays in the queue until it is committed, so a
// deadline shorter than a large batch would retry it forever.
#[celery::task(max_retries = 0, expires = 1)]
pub async fn electoral_log_batch_dispatcher() -> Result<()> {
    info!("starting electoral_log_batch_dispatcher");

    // Reuse the global AMQP connection.
    let connection_arc = get_celery_connection().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;

    // Get the batch size from PgConfig.
    let batch_size: usize = PgConfig::from_env()?.default_sql_batch_size.try_into()?;

    let mut failed_queues = Vec::new();
    for source in [Queue::ElectoralLogBatch, Queue::ElectoralLogEvent] {
        let queue_name = source.queue_name(&slug);
        // Each queue has its own channel: dropping it returns the deliveries
        // left unacknowledged to their queue.
        let channel = connection_arc
            .create_channel()
            .await
            .with_context(|| "Error creating RabbitMQ channel")?;
        let mut queue = AmqpLogEventQueue::new(
            channel,
            queue_name.clone(),
            Queue::ElectoralLogDeadLetter.queue_name(&slug),
        )
        .await?;
        if let Err(err) =
            drain_log_event_queue(&mut queue, &mut ElectoralLogBoards, batch_size).await
        {
            warn!(queue = %queue_name, error = ?err, "Electoral log events left in the queue");
            failed_queues.push(queue_name);
        }
    }
    if !failed_queues.is_empty() {
        return Err(anyhow!("Electoral log events were left in {failed_queues:?}").into());
    }
    info!("finishing electoral_log_batch_dispatcher");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn database() -> DbClient {
        let mut config = deadpool_postgres::Config::new();
        config.url = Some(
            std::env::var("CAST_VOTE_TEST_DATABASE_URL").expect("CAST_VOTE_TEST_DATABASE_URL"),
        );
        config
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .unwrap()
            .get()
            .await
            .unwrap()
    }

    fn input(election_event_id: &str) -> LogEventInput {
        LogEventInput {
            election_event_id: election_event_id.to_string(),
            message_type: LogMessageType::Internal,
            user_id: None,
            username: None,
            tenant_id: "tenant".to_string(),
            body: LogEventBody::Plain(String::new()),
        }
    }

    /// One event whose Keycloak query fails, then a healthy event that queries
    /// both databases.
    async fn failing_then_healthy(
        hasura: &Transaction<'_>,
        keycloak: &Transaction<'_>,
        input: &LogEventInput,
    ) -> anyhow::Result<LogEventPreparation<i32>> {
        if input.election_event_id == "failing" {
            keycloak.query_one("SELECT 1 / 0", &[]).await?;
        }
        let row = hasura.query_one("SELECT 2", &[]).await?;
        keycloak.query_one("SELECT 3", &[]).await?;
        Ok(LogEventPreparation::Ready {
            board: input.election_event_id.clone(),
            messages: vec![row.get(0)],
        })
    }

    #[tokio::test]
    #[ignore = "requires the disposable devenv database fixture"]
    async fn a_failed_query_holds_back_only_its_event() {
        let (mut hasura_client, mut keycloak_client) = (database().await, database().await);
        let mut hasura_tx = hasura_client.transaction().await.unwrap();
        let mut keycloak_tx = keycloak_client.transaction().await.unwrap();
        let events = [input("failing"), input("healthy")];

        let preparations = prepare_each(
            &mut hasura_tx,
            &mut keycloak_tx,
            &events,
            async |hasura, keycloak, input| failing_then_healthy(hasura, keycloak, input).await,
        )
        .await
        .unwrap();
        println!(
            "with savepoints: {:?}",
            preparations
                .iter()
                .map(|preparation| match preparation {
                    LogEventPreparation::Ready { board, .. } => format!("ready {board}"),
                    LogEventPreparation::Retained(err) => format!("retained: {err:#}"),
                    LogEventPreparation::Rejected(err) => format!("rejected: {err:#}"),
                })
                .collect::<Vec<_>>()
        );
        assert!(matches!(preparations[0], LogEventPreparation::Retained(_)));
        assert!(matches!(
            &preparations[1],
            LogEventPreparation::Ready { board, messages } if board == "healthy" && messages == &[2]
        ));
        hasura_tx.rollback().await.unwrap();
        keycloak_tx.rollback().await.unwrap();

        // Control: without savepoints the failed query aborts the transaction,
        // and the healthy event's queries fail too.
        let keycloak_tx = keycloak_client.transaction().await.unwrap();
        let hasura_tx = hasura_client.transaction().await.unwrap();
        assert!(failing_then_healthy(&hasura_tx, &keycloak_tx, &events[0])
            .await
            .is_err());
        let healthy = failing_then_healthy(&hasura_tx, &keycloak_tx, &events[1]).await;
        println!(
            "without savepoints, the healthy event: {:?}",
            healthy
                .as_ref()
                .map(|_| ())
                .map_err(|err| format!("{err:#}"))
        );
        assert!(healthy.is_err());
    }
}

#[cfg(test)]
mod internal_message_tests {
    use super::*;
    use crate::services::insert_cast_vote::hash_voter_id;
    use electoral_log::messages::message::{Message, SigningData};
    use electoral_log::messages::newtypes::{
        CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
        VoterIpString, VotingChannelString,
    };
    use strand::hash::HashWrapper;
    use strand::serialization::{StrandDeserialize, StrandSerialize};
    use strand::signature::StrandSignatureSk;

    const EVENT_ID: &str = "event-id";

    fn queued_cast_vote(system_sk: &StrandSignatureSk) -> anyhow::Result<LogEventInput> {
        let signing_data = SigningData::new(system_sk.clone(), "voter-id", system_sk.clone());
        let message = Message::cast_vote_with_channel_message(
            EventIdString(EVENT_ID.to_string()),
            ElectionIdString(Some("election-id".to_string())),
            PseudonymHash(HashWrapper::new(hash_voter_id("voter-id")?)),
            CastVoteHash::new([2; 64]),
            &signing_data,
            VoterIpString("ip".to_string()),
            VoterCountryString("country".to_string()),
            VotingChannelString("ONLINE".to_string()),
            Some("voter-id".to_string()),
            Some("voter".to_string()),
            "area-id".to_string(),
        )?;
        let board_message: ElectoralLogMessage = (&message).try_into()?;
        Ok(LogEventInput {
            election_event_id: EVENT_ID.to_string(),
            message_type: LogMessageType::Internal,
            user_id: Some("voter-id".to_string()),
            username: Some("voter".to_string()),
            tenant_id: "tenant-id".to_string(),
            body: LogEventBody::Plain(serde_json::to_string(&board_message)?),
        })
    }

    #[test]
    fn internal_message_signed_with_the_event_key_is_accepted() -> anyhow::Result<()> {
        let system_sk = StrandSignatureSk::r#gen()?;
        let system_pk = StrandSignaturePk::from_sk(&system_sk)?;
        let input = queued_cast_vote(&system_sk)?;

        let message = verified_internal_message(&input, &system_pk)?;

        assert_eq!(message.statement_kind, "CastVote");
        assert_eq!(message.user_id.as_deref(), Some("voter-id"));
        Ok(())
    }

    #[test]
    fn internal_message_signed_with_another_key_is_rejected() -> anyhow::Result<()> {
        let input = queued_cast_vote(&StrandSignatureSk::r#gen()?)?;
        let event_pk = StrandSignaturePk::from_sk(&StrandSignatureSk::r#gen()?)?;

        assert!(verified_internal_message(&input, &event_pk).is_err());
        Ok(())
    }

    #[test]
    fn internal_message_for_another_election_event_is_rejected() -> anyhow::Result<()> {
        let system_sk = StrandSignatureSk::r#gen()?;
        let system_pk = StrandSignaturePk::from_sk(&system_sk)?;
        let input = LogEventInput {
            election_event_id: "other-event-id".to_string(),
            ..queued_cast_vote(&system_sk)?
        };

        assert!(verified_internal_message(&input, &system_pk).is_err());
        Ok(())
    }

    #[test]
    fn internal_message_with_a_forged_voter_is_rejected() -> anyhow::Result<()> {
        let system_sk = StrandSignatureSk::r#gen()?;
        let system_pk = StrandSignaturePk::from_sk(&system_sk)?;
        let mut input = queued_cast_vote(&system_sk)?;
        let mut row: ElectoralLogMessage = deserialize_str(&input.body.as_raw())?;
        let mut message = Message::strand_deserialize(&row.message)?;
        message.user_id = Some("other-voter-id".to_string());
        row.message = message.strand_serialize()?;
        input.body = LogEventBody::Plain(serde_json::to_string(&row)?);

        assert!(verified_internal_message(&input, &system_pk).is_err());
        Ok(())
    }

    #[test]
    fn hash_voter_id_is_the_pseudonym_of_the_electoral_log() -> anyhow::Result<()> {
        assert_eq!(
            hash_voter_id("voter-id")?,
            electoral_log::messages::message::hash_voter_id("voter-id")?
        );
        Ok(())
    }

    #[test]
    fn the_key_cache_tells_tenants_with_the_same_board_name_apart() {
        assert_ne!(
            board_key("tenant-a", EVENT_ID, "board"),
            board_key("tenant-b", EVENT_ID, "board")
        );
    }

    #[tokio::test]
    async fn a_message_signed_with_a_new_board_key_is_accepted_after_reading_it(
    ) -> anyhow::Result<()> {
        let cached_pk = StrandSignaturePk::from_sk(&StrandSignatureSk::r#gen()?)?;
        let new_sk = StrandSignatureSk::r#gen()?;
        let new_pk = StrandSignaturePk::from_sk(&new_sk)?;
        let input = queued_cast_vote(&new_sk)?;
        let mut read = false;

        let message = verify_with_cached_or_current_key(
            Some(cached_pk),
            || {
                read = true;
                async move { Ok::<_, anyhow::Error>(new_pk) }
            },
            |system_pk: &StrandSignaturePk| verified_internal_message(&input, system_pk),
        )
        .await??;

        assert!(read);
        assert_eq!(message.statement_kind, "CastVote");
        Ok(())
    }

    #[tokio::test]
    async fn a_message_signed_with_an_unknown_key_is_rejected_after_reading_the_board_key(
    ) -> anyhow::Result<()> {
        let input = queued_cast_vote(&StrandSignatureSk::r#gen()?)?;
        let cached_pk = StrandSignaturePk::from_sk(&StrandSignatureSk::r#gen()?)?;
        let new_pk = StrandSignaturePk::from_sk(&StrandSignatureSk::r#gen()?)?;
        let mut read = false;

        let verified = verify_with_cached_or_current_key(
            Some(cached_pk),
            || {
                read = true;
                async move { Ok::<_, anyhow::Error>(new_pk) }
            },
            |system_pk: &StrandSignaturePk| verified_internal_message(&input, system_pk),
        )
        .await?;

        assert!(read);
        assert!(verified.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn a_message_signed_with_the_cached_key_is_accepted_without_reading_the_board_key(
    ) -> anyhow::Result<()> {
        let system_sk = StrandSignatureSk::r#gen()?;
        let cached_pk = StrandSignaturePk::from_sk(&system_sk)?;
        let input = queued_cast_vote(&system_sk)?;
        let mut read = false;

        let message = verify_with_cached_or_current_key(
            Some(cached_pk),
            || {
                read = true;
                async { Err::<StrandSignaturePk, _>(anyhow!("the board key is read")) }
            },
            |system_pk: &StrandSignaturePk| verified_internal_message(&input, system_pk),
        )
        .await??;

        assert!(!read);
        assert_eq!(message.statement_kind, "CastVote");
        Ok(())
    }

    #[tokio::test]
    async fn an_error_reading_the_board_key_is_returned_as_an_error() -> anyhow::Result<()> {
        let input = queued_cast_vote(&StrandSignatureSk::r#gen()?)?;
        let cached_pk = StrandSignaturePk::from_sk(&StrandSignatureSk::r#gen()?)?;

        let verified = verify_with_cached_or_current_key(
            Some(cached_pk),
            || async { Err::<StrandSignaturePk, _>(anyhow!("vault unavailable")) },
            |system_pk: &StrandSignaturePk| verified_internal_message(&input, system_pk),
        )
        .await;

        assert!(verified.is_err());
        Ok(())
    }
}
