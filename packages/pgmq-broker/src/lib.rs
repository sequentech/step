// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! PGMQ transport for the existing Celery task protocol.
//! Connections (including TLS) belong to the application; this crate owns no credentials.
//! Each environment has its own task-queue database, so queues keep their plain names.

use async_trait::async_trait;
use celery::broker::{
    Broker, BrokerBuilder, ConsumerHealth, Delivery, DeliveryOutcome, DeliveryStream,
};
use celery::error::{BrokerError, ProtocolError};
use celery::protocol::{Message, TryDeserializeMessage};
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use futures::Stream;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::task::AbortHandle;
use tokio_postgres::GenericClient;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;

const LEASE_SECONDS: i32 = 60;
const RENEW_SECONDS: u64 = 10;
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(200);
/// After this many deliveries ended without an outcome (a worker that crashed or lost its
/// lease), the message is archived as failed instead of being delivered again.
const MAX_ABANDONED_DELIVERIES: i64 = 5;
/// PGMQ's limit, which keeps its table and index names within PostgreSQL's.
pub const MAX_QUEUE_NAME_LEN: usize = 47;
/// Message header recording how the processing of an archived message ended.
pub const OUTCOME_HEADER: &str = "x-step-outcome";

pub mod setup;

fn db_error(error: impl std::error::Error + 'static) -> BrokerError {
    // Database messages/details can contain voter payloads. Keep only safe categories/codes.
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    let mut category = "database operation failed".to_string();
    while let Some(error) = cause {
        if let Some(error) = error.downcast_ref::<tokio_postgres::Error>() {
            if let Some(db) = error.as_db_error() {
                category = format!("PostgreSQL SQLSTATE {}", db.code().code());
                break;
            }
            if error.is_closed() {
                category = "connection closed".into();
                break;
            }
        }
        if error.is::<tokio::time::error::Elapsed>() {
            category = "operation timed out".into();
            break;
        }
        if let Some(error) = error.downcast_ref::<deadpool_postgres::PoolError>() {
            if let deadpool_postgres::PoolError::Timeout(_) = error {
                category = "connection pool timed out".into();
                break;
            }
        }
        if let Some(error) = error.downcast_ref::<std::io::Error>() {
            category = format!("I/O {:?}", error.kind());
        }
        cause = error.source();
    }
    BrokerError::IoError(std::io::Error::other(format!("PGMQ {category}")))
}

fn lost_lease() -> BrokerError {
    BrokerError::IoError(std::io::Error::other(
        "PGMQ delivery lease is no longer owned",
    ))
}

/// Queue names are table-name suffixes: lowercase ASCII letters, digits and underscores,
/// at most [`MAX_QUEUE_NAME_LEN`] characters.
pub fn validate_queue_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_QUEUE_NAME_LEN {
        return Err(format!(
            "queue name {name:?} must have 1 to {MAX_QUEUE_NAME_LEN} characters"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!(
            "queue name {name:?} may only contain lowercase ASCII letters, digits and underscores"
        ));
    }
    Ok(())
}

/// Refuse a task-queue database that is not set up or belongs to another environment.
pub async fn verify_environment<C: GenericClient + Sync>(
    client: &C,
    environment: &str,
) -> Result<(), BrokerError> {
    let installed: Option<String> = client
        .query_opt("SELECT environment FROM step_queue.installation", &[])
        .await
        .map(|row| row.map(|row| row.get(0)))
        .or_else(|error| match error.code() {
            Some(code) if *code == tokio_postgres::error::SqlState::UNDEFINED_TABLE => Ok(None),
            _ => Err(db_error(error)),
        })?;
    match installed {
        Some(installed) if installed == environment => Ok(()),
        Some(installed) => Err(BrokerError::InvalidBrokerUrl(format!(
            "the task-queue database belongs to environment {installed}, not {environment}"
        ))),
        None => Err(BrokerError::InvalidBrokerUrl(
            "the task-queue database is not set up".into(),
        )),
    }
}

/// Delete the messages of `queue`'s archive that were archived more than `retention` ago,
/// at most `batch` rows per statement. Returns how many were deleted; the queue itself is
/// not touched.
pub async fn purge_archive<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    retention: Duration,
    batch: i64,
) -> Result<u64, BrokerError> {
    validate_queue_name(queue).map_err(BrokerError::UnknownQueue)?;
    let retention_secs = retention.as_secs_f64();
    let mut deleted = 0;
    loop {
        let count = client
            .execute(
                &format!(
                    "DELETE FROM pgmq.a_{queue} WHERE msg_id IN (\
                     SELECT msg_id FROM pgmq.a_{queue} \
                     WHERE archived_at < clock_timestamp() - make_interval(secs => $1) \
                     ORDER BY archived_at LIMIT $2)"
                ),
                &[&retention_secs, &batch],
            )
            .await
            .map_err(db_error)?;
        deleted += count;
        if count < u64::try_from(batch).unwrap_or(u64::MAX) {
            return Ok(deleted);
        }
    }
}

/// Decode the same Celery JSON envelope used by Rust and the Keycloak publisher.
/// Deliveries of a message that ended neither in a retry nor an outcome: each read counts
/// one delivery, and each retry keeps the row and counts one in the message's headers.
fn abandoned_deliveries(read_count: i32, payload: &Value) -> i64 {
    let retries = payload["headers"]["retries"].as_i64().unwrap_or(0);
    i64::from(read_count) - 1 - retries
}

pub fn decode(value: Value) -> Result<Message, ProtocolError> {
    serde_json::from_value::<celery::protocol::Delivery>(value)?.try_deserialize_message()
}

/// Enqueue on an existing transaction when the producer needs atomic business-state changes.
pub async fn send<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    message: &Message,
) -> Result<(), BrokerError> {
    let payload: Value = serde_json::from_slice(&message.json_serialized(None)?)?;
    client
        .query_one(
            "SELECT pgmq.send($1, $2, COALESCE($3::timestamptz, clock_timestamp()))",
            &[&queue, &payload, &message.headers.eta],
        )
        .await
        .map_err(db_error)?;
    Ok(())
}

pub struct PgmqBrokerBuilder {
    pool: Option<Arc<Pool>>,
    environment: Option<String>,
    publisher: Option<Arc<deadpool_postgres::Client>>,
    queues: HashSet<String>,
    prefetch: u16,
}

impl PgmqBrokerBuilder {
    pub fn from_pool(pool: Arc<Pool>) -> Self {
        Self {
            pool: Some(pool),
            environment: None,
            publisher: None,
            queues: HashSet::new(),
            prefetch: 100,
        }
    }
    /// The environment (ENV_SLUG) the task-queue database must belong to.
    pub fn environment(mut self, environment: &str) -> Self {
        self.environment = Some(environment.into());
        self
    }
    /// Keep Beat publication on the session that owns its advisory leadership lock.
    pub fn publisher_connection(mut self, client: Arc<deadpool_postgres::Client>) -> Self {
        self.publisher = Some(client);
        self
    }
}

#[async_trait]
impl BrokerBuilder for PgmqBrokerBuilder {
    fn new(_url: &str) -> Self {
        Self {
            pool: None,
            environment: None,
            publisher: None,
            queues: HashSet::new(),
            prefetch: 100,
        }
    }

    fn prefetch_count(mut self: Box<Self>, count: u16) -> Box<dyn BrokerBuilder> {
        self.prefetch = count;
        self
    }

    fn declare_queue(mut self: Box<Self>, name: &str) -> Box<dyn BrokerBuilder> {
        self.queues.insert(name.into());
        self
    }

    fn heartbeat(self: Box<Self>, _heartbeat: Option<u16>) -> Box<dyn BrokerBuilder> {
        self
    }

    async fn build(&self, _connection_timeout: u32) -> Result<Box<dyn Broker>, BrokerError> {
        let pool = self.pool.as_ref().ok_or_else(|| BrokerError::InvalidBrokerUrl(
            "PGMQ requires PgmqBrokerBuilder::from_pool with the application's TLS configuration".into()
        ))?.clone();
        if self.prefetch == 0 {
            return Err(BrokerError::IoError(std::io::Error::other(
                "PGMQ prefetch must be positive",
            )));
        }
        let environment = self.environment.as_deref().ok_or_else(|| {
            BrokerError::InvalidBrokerUrl("PGMQ requires PgmqBrokerBuilder::environment".into())
        })?;
        for queue in &self.queues {
            validate_queue_name(queue).map_err(BrokerError::UnknownQueue)?;
        }
        let existing: HashSet<String> = tokio::time::timeout(QUERY_TIMEOUT, async {
            let client = pool.get().await.map_err(db_error)?;
            verify_environment(&**client, environment).await?;
            // Queues are created when the environment is provisioned, never by its services.
            let declared: Vec<&str> = self.queues.iter().map(String::as_str).collect();
            Ok::<_, BrokerError>(
                client
                    .query(
                        "SELECT queue_name FROM pgmq.meta WHERE queue_name = ANY($1)",
                        &[&declared],
                    )
                    .await
                    .map_err(db_error)?
                    .iter()
                    .map(|row| row.get(0))
                    .collect(),
            )
        })
        .await
        .map_err(db_error)??;
        if let Some(missing) = self.queues.iter().find(|queue| !existing.contains(*queue)) {
            tracing::error!(
                queue = missing.as_str(),
                "PGMQ queue does not exist; set up the task-queue database"
            );
            return Err(BrokerError::UnknownQueue(missing.clone()));
        }
        Ok(Box::new(PgmqBroker {
            publisher: self.publisher.clone(),
            pool,
            queues: self.queues.clone(),
            permits: self
                .queues
                .iter()
                .map(|queue| {
                    (
                        queue.clone(),
                        Arc::new(Semaphore::new(usize::from(self.prefetch))),
                    )
                })
                .collect(),
            consumers: Mutex::new(HashMap::new()),
        }))
    }
}

struct PgmqBroker {
    pool: Arc<Pool>,
    publisher: Option<Arc<deadpool_postgres::Client>>,
    queues: HashSet<String>,
    permits: HashMap<String, Arc<Semaphore>>,
    consumers: Mutex<HashMap<String, (String, CancellationToken)>>,
}

type DeliveryResult = Result<Box<dyn Delivery>, Box<dyn celery::broker::DeliveryError>>;

struct Deliveries {
    receiver: ReceiverStream<DeliveryResult>,
    cancel: CancellationToken,
}

impl Drop for Deliveries {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl Stream for Deliveries {
    type Item = DeliveryResult;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_next(cx)
    }
}
impl DeliveryStream for Deliveries {}

#[async_trait]
impl Broker for PgmqBroker {
    fn safe_url(&self) -> String {
        "pgmq://configured-postgresql-pool".into()
    }

    async fn consume(
        &self,
        queue: &str,
        error_handler: Box<dyn Fn(BrokerError) + Send + Sync + 'static>,
    ) -> Result<(String, Box<dyn DeliveryStream>), BrokerError> {
        if !self.queues.contains(queue) {
            return Err(BrokerError::UnknownQueue(queue.into()));
        }
        let tag = uuid::Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        {
            let mut consumers = self.consumers.lock().await;
            consumers.retain(|_, (_, stop)| !stop.is_cancelled());
            consumers.insert(tag.clone(), (queue.into(), cancel.clone()));
        }
        let pool = self.pool.clone();
        let queue_name = queue.to_string();
        let permits = self
            .permits
            .get(queue)
            .ok_or_else(|| BrokerError::UnknownQueue(queue.into()))?
            .clone();
        let (sender, receiver) = mpsc::channel(1);
        let stop = cancel.clone();
        tokio::spawn(async move {
            let polling = async {
                loop {
                    let permit = permits.clone().acquire_owned().await.map_err(db_error)?;
                    let row = tokio::time::timeout(QUERY_TIMEOUT, async {
                        let client = pool.get().await.map_err(db_error)?;
                        client
                            .query_opt(
                                "SELECT * FROM pgmq.read($1, $2, 1)",
                                &[&queue_name, &LEASE_SECONDS],
                            )
                            .await
                            .map_err(db_error)
                    })
                    .await
                    .map_err(db_error)??;
                    if let Some(row) = row {
                        let delivery = PgDelivery::new(
                            pool.clone(),
                            queue_name.clone(),
                            row.get("msg_id"),
                            row.get("read_ct"),
                            row.get::<_, Option<Value>>("message")
                                .unwrap_or(Value::Null),
                            permit,
                        );
                        if abandoned_deliveries(delivery.read_count, &delivery.payload)
                            >= MAX_ABANDONED_DELIVERIES
                        {
                            tracing::error!(
                                queue = queue_name.as_str(),
                                message_id = delivery.id,
                                "PGMQ message was abandoned by {MAX_ABANDONED_DELIVERIES} \
                                 deliveries; archiving it as failed"
                            );
                            if let Err(error) =
                                delivery.ack_with_outcome(DeliveryOutcome::Failed).await
                            {
                                tracing::warn!(%error, "Could not archive an abandoned PGMQ message");
                            }
                            continue;
                        }
                        if sender
                            .send(Ok(Box::new(delivery) as Box<dyn Delivery>))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    } else {
                        drop(permit);
                        tokio::time::sleep(POLL_INTERVAL).await;
                    }
                }
                Ok::<(), BrokerError>(())
            };
            tokio::select! {
                _ = stop.cancelled() => {},
                result = polling => { if let Err(error) = result { error_handler(error); } }
            }
            stop.cancel();
        });
        Ok((
            tag,
            Box::new(Deliveries {
                receiver: ReceiverStream::new(receiver),
                cancel,
            }),
        ))
    }

    async fn cancel(&self, tag: &str) -> Result<(), BrokerError> {
        if let Some((_, cancel)) = self.consumers.lock().await.remove(tag) {
            cancel.cancel();
        }
        Ok(())
    }
    async fn ack(&self, delivery: &dyn Delivery) -> Result<(), BrokerError> {
        delivery.ack().await
    }
    async fn ack_with_outcome(
        &self,
        delivery: &dyn Delivery,
        outcome: DeliveryOutcome,
    ) -> Result<(), BrokerError> {
        delivery.ack_with_outcome(outcome).await
    }
    async fn retry(
        &self,
        delivery: &dyn Delivery,
        eta: Option<DateTime<Utc>>,
    ) -> Result<(), BrokerError> {
        delivery.resend(self, eta).await
    }
    async fn send(&self, message: &Message, queue: &str) -> Result<(), BrokerError> {
        if !self.queues.contains(queue) {
            return Err(BrokerError::UnknownQueue(queue.into()));
        }
        // Producers enqueue inside requests, such as a voter's, so a stalled database must not
        // hold them.
        tokio::time::timeout(QUERY_TIMEOUT, async {
            if let Some(client) = &self.publisher {
                return send(&****client, queue, message).await;
            }
            let client = self.pool.get().await.map_err(db_error)?;
            send(&**client, queue, message).await
        })
        .await
        .map_err(db_error)?
    }
    // ETAs are stored in PostgreSQL and do not occupy a worker permit until due.
    async fn increase_prefetch_count(&self) -> Result<(), BrokerError> {
        Ok(())
    }
    async fn decrease_prefetch_count(&self) -> Result<(), BrokerError> {
        Ok(())
    }
    async fn close(&self) -> Result<(), BrokerError> {
        for (_, (_, cancel)) in self.consumers.lock().await.drain() {
            cancel.cancel();
        }
        Ok(())
    }
    async fn reconnect(&self, _connection_timeout: u32) -> Result<(), BrokerError> {
        tokio::time::timeout(QUERY_TIMEOUT, async {
            self.pool
                .get()
                .await
                .map_err(db_error)?
                .simple_query("SELECT 1")
                .await
                .map_err(db_error)?;
            Ok(())
        })
        .await
        .map_err(db_error)?
    }
    async fn check_consumer_health(
        &self,
        queues: &Vec<String>,
    ) -> Result<Vec<ConsumerHealth>, BrokerError> {
        let counts: Vec<i64> = tokio::time::timeout(QUERY_TIMEOUT, async {
            let client = self.pool.get().await.map_err(db_error)?;
            let mut counts = Vec::new();
            for queue in queues {
                counts.push(
                    client
                        .query_one("SELECT queue_length FROM pgmq.metrics($1)", &[queue])
                        .await
                        .map_err(db_error)?
                        .get(0),
                );
            }
            Ok::<_, BrokerError>(counts)
        })
        .await
        .map_err(db_error)??;
        let consumers = self.consumers.lock().await;
        let mut result = Vec::new();
        for (queue, count) in queues.iter().zip(counts) {
            let active = consumers
                .values()
                .filter(|(name, stop)| name == queue && !stop.is_cancelled())
                .count();
            result.push(ConsumerHealth {
                queue_name: queue.clone(),
                consumer_count: active as u32,
                message_count: u32::try_from(count).unwrap_or(u32::MAX),
                is_consuming: active > 0,
            });
        }
        Ok(result)
    }
}

struct PgDelivery {
    pool: Arc<Pool>,
    queue: String,
    id: i64,
    read_count: i32,
    payload: Value,
    finished: Arc<Mutex<bool>>,
    renewal: AbortHandle,
    _permit: OwnedSemaphorePermit,
}

impl std::fmt::Debug for PgDelivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgDelivery")
            .field("id", &self.id)
            .field("queue", &self.queue)
            .finish()
    }
}

impl PgDelivery {
    fn new(
        pool: Arc<Pool>,
        queue: String,
        id: i64,
        read_count: i32,
        payload: Value,
        permit: OwnedSemaphorePermit,
    ) -> Self {
        let finished = Arc::new(Mutex::new(false));
        let state = finished.clone();
        let renew_pool = pool.clone();
        let renew_queue = queue.clone();
        let renewal = tokio::spawn(async move {
            let mut last_success = tokio::time::Instant::now();
            loop {
                tokio::time::sleep(Duration::from_secs(RENEW_SECONDS)).await;
                let done = state.lock().await;
                if *done { return; }
                let result = tokio::time::timeout(QUERY_TIMEOUT, async {
                    let client = renew_pool.get().await.map_err(db_error)?;
                    client.execute(&format!(
                        "UPDATE pgmq.q_{renew_queue} SET vt = clock_timestamp() + interval '60 seconds' \
                         WHERE msg_id = $1 AND read_ct = $2 AND vt > clock_timestamp()"),
                        &[&id, &read_count]).await.map_err(db_error)
                }).await;
                match result {
                    Ok(Ok(1)) => last_success = tokio::time::Instant::now(),
                    Ok(Ok(_)) => {
                        tracing::error!(message_id = id, "PGMQ lease lost; terminating worker to stop stale execution");
                        std::process::exit(1);
                    }
                    _ if last_success.elapsed() >= Duration::from_secs(40) => {
                        tracing::error!(message_id = id, "PGMQ lease cannot be renewed; terminating worker before expiry");
                        std::process::exit(1);
                    }
                    _ => tracing::warn!(message_id = id, "PGMQ lease renewal failed; retrying"),
                }
            }
        }).abort_handle();
        Self {
            pool,
            queue,
            id,
            read_count,
            payload,
            finished,
            renewal,
            _permit: permit,
        }
    }
}

impl Drop for PgDelivery {
    fn drop(&mut self) {
        self.renewal.abort();
    }
}
impl TryDeserializeMessage for PgDelivery {
    fn try_deserialize_message(&self) -> Result<Message, ProtocolError> {
        decode(self.payload.clone())
    }
}

#[async_trait]
impl Delivery for PgDelivery {
    async fn resend(
        &self,
        _broker: &dyn Broker,
        eta: Option<DateTime<Utc>>,
    ) -> Result<(), BrokerError> {
        tokio::time::timeout(QUERY_TIMEOUT, async {
            let mut finished = self.finished.lock().await;
            if *finished {
                return Err(lost_lease());
            }
            let mut message = self.try_deserialize_message()?;
            message.headers.retries = Some(message.headers.retries.unwrap_or(0).saturating_add(1));
            message.headers.eta = eta;
            let payload: Value = serde_json::from_slice(&message.json_serialized(None)?)?;
            let client = self.pool.get().await.map_err(db_error)?;
            // Rescheduling the same row is atomic; Celery's following ack must become a no-op.
            let changed = client
                .execute(
                    &format!(
            "UPDATE pgmq.q_{} SET message = $3, vt = COALESCE($4::timestamptz, clock_timestamp()) \
             WHERE msg_id = $1 AND read_ct = $2 AND vt > clock_timestamp()", self.queue),
                    &[&self.id, &self.read_count, &payload, &eta],
                )
                .await
                .map_err(db_error)?;
            if changed != 1 {
                return Err(lost_lease());
            }
            *finished = true;
            Ok(())
        })
        .await
        .map_err(db_error)?
    }

    async fn remove(&self) -> Result<(), BrokerError> {
        self.ack().await
    }

    async fn ack(&self) -> Result<(), BrokerError> {
        self.archive(None).await
    }

    async fn ack_with_outcome(&self, outcome: DeliveryOutcome) -> Result<(), BrokerError> {
        self.archive(Some(outcome)).await
    }
}

impl PgDelivery {
    async fn archive(&self, outcome: Option<DeliveryOutcome>) -> Result<(), BrokerError> {
        tokio::time::timeout(QUERY_TIMEOUT, async {
            let mut finished = self.finished.lock().await;
            if *finished {
                return Ok(());
            }
            let mut client = self.pool.get().await.map_err(db_error)?;
            let tx = client.transaction().await.map_err(db_error)?;
            // Only the lease owner archives; the outcome stays with the archived message.
            let owned = tx
                .execute(
                    &format!(
                        "UPDATE pgmq.q_{} SET headers = CASE WHEN $3::text IS NULL THEN headers \
                         ELSE COALESCE(headers, '{{}}'::jsonb) \
                         || jsonb_build_object('{OUTCOME_HEADER}', $3::text) END \
                         WHERE msg_id = $1 AND read_ct = $2 AND vt > clock_timestamp()",
                        self.queue
                    ),
                    &[
                        &self.id,
                        &self.read_count,
                        &outcome.map(|outcome| outcome.as_str()),
                    ],
                )
                .await
                .map_err(db_error)?;
            if owned != 1 {
                return Err(lost_lease());
            }
            // Celery also acks malformed/failed/expired tasks. Archive every terminal delivery
            // so operators can inspect/replay failures without changing task handlers.
            tx.query_one(
                "SELECT pgmq.archive($1, $2::bigint)",
                &[&self.queue, &self.id],
            )
            .await
            .map_err(db_error)?;
            tx.commit().await.map_err(db_error)?;
            *finished = true;
            Ok(())
        })
        .await
        .map_err(db_error)?
    }
}

#[cfg(test)]
mod tests;
