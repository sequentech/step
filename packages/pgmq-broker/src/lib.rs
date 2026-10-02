// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! PGMQ transport for the existing Celery task protocol.
//! Connections (including TLS) belong to the application; this crate owns no credentials.

use async_trait::async_trait;
use celery::broker::{Broker, BrokerBuilder, ConsumerHealth, Delivery, DeliveryStream};
use celery::error::{BrokerError, ProtocolError};
use celery::protocol::{Message, TryDeserializeMessage};
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use futures::Stream;
use serde_json::Value;
use sha2::{Digest, Sha256};
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

fn db_error(_: impl std::fmt::Display) -> BrokerError {
    // Database errors can contain message payloads. Keep voter data out of broker logs.
    BrokerError::IoError(std::io::Error::other("PGMQ database operation failed"))
}

fn lost_lease() -> BrokerError {
    BrokerError::IoError(std::io::Error::other(
        "PGMQ delivery lease is no longer owned",
    ))
}

/// Map logical queue names to safe, case-sensitive identities within PGMQ's 47-character limit.
pub fn queue_name(logical_name: &str) -> String {
    format!(
        "step_{}",
        &format!("{:x}", Sha256::digest(logical_name.as_bytes()))[..40]
    )
}

/// Decode the same Celery JSON envelope used by Rust and the Keycloak publisher.
pub fn decode(value: Value) -> Result<Message, ProtocolError> {
    serde_json::from_value::<celery::protocol::Delivery>(value)?.try_deserialize_message()
}

/// Enqueue on an existing transaction when the producer needs atomic business-state changes.
pub async fn send<C: GenericClient + Sync>(
    client: &C,
    logical_queue: &str,
    message: &Message,
) -> Result<(), BrokerError> {
    let payload: Value = serde_json::from_slice(&message.json_serialized(None)?)?;
    client
        .query_one(
            "SELECT pgmq.send($1, $2, COALESCE($3::timestamptz, clock_timestamp()))",
            &[&queue_name(logical_queue), &payload, &message.headers.eta],
        )
        .await
        .map_err(db_error)?;
    Ok(())
}

pub struct PgmqBrokerBuilder {
    pool: Option<Arc<Pool>>,
    publisher: Option<Arc<deadpool_postgres::Client>>,
    queues: HashSet<String>,
    prefetch: u16,
}

impl PgmqBrokerBuilder {
    pub fn from_pool(pool: Arc<Pool>) -> Self {
        Self {
            pool: Some(pool),
            publisher: None,
            queues: HashSet::new(),
            prefetch: 100,
        }
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
        let client = pool.get().await.map_err(db_error)?;
        // Installation is a deployment operation; workers only declare their logged queues.
        for queue in &self.queues {
            client
                .query_one("SELECT pgmq.create($1)", &[&queue_name(queue)])
                .await
                .map_err(db_error)?;
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
        let physical_queue = queue_name(queue);
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
                                &[&physical_queue, &LEASE_SECONDS],
                            )
                            .await
                            .map_err(db_error)
                    })
                    .await
                    .map_err(db_error)??;
                    if let Some(row) = row {
                        let delivery = PgDelivery::new(
                            pool.clone(),
                            physical_queue.clone(),
                            row.get("msg_id"),
                            row.get("read_ct"),
                            row.get::<_, Option<Value>>("message")
                                .unwrap_or(Value::Null),
                            permit,
                        );
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
        if let Some(client) = &self.publisher {
            return send(&****client, queue, message).await;
        }
        let client = self.pool.get().await.map_err(db_error)?;
        send(&**client, queue, message).await
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
        let client = self.pool.get().await.map_err(db_error)?;
        let consumers = self.consumers.lock().await;
        let mut result = Vec::new();
        for queue in queues {
            let count: i64 = client
                .query_one(
                    "SELECT queue_length FROM pgmq.metrics($1)",
                    &[&queue_name(queue)],
                )
                .await
                .map_err(db_error)?
                .get(0);
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
        tokio::time::timeout(QUERY_TIMEOUT, async {
            let mut finished = self.finished.lock().await;
            if *finished {
                return Ok(());
            }
            let mut client = self.pool.get().await.map_err(db_error)?;
            let tx = client.transaction().await.map_err(db_error)?;
            let owned = tx
                .query_opt(
                    &format!(
                        "SELECT 1 FROM pgmq.q_{} WHERE msg_id = $1 AND read_ct = $2 \
             AND vt > clock_timestamp() FOR UPDATE",
                        self.queue
                    ),
                    &[&self.id, &self.read_count],
                )
                .await
                .map_err(db_error)?;
            if owned.is_none() {
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
