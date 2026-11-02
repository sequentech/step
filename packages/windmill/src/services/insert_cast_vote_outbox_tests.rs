// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signed CastVote of an accepted vote, stored with the vote, against a
//! disposable PostgreSQL database with this branch's schema (and RabbitMQ and
//! immudb for the delivery test), with synthetic data only. Run with
//! `CAST_VOTE_TEST_DATABASE_URL=… MASTER_SECRET=<64 hex digits>
//! [ELECTORAL_LOG_TEST_AMQP_URL=… ELECTORAL_LOG_TEST_IMMUDB_URL=…]
//! cargo test -p windmill --lib services::insert_cast_vote::outbox_tests -- --include-ignored
//! --test-threads=1`, on a database used by nothing else: the relay tests take
//! every outbox row. `HASURA_DB__*` must name the same database: some tests
//! drive the production callers, which read their configuration from the
//! environment.
//!
//! Not exercised here: the Datafix SetVoted step after the commit, which needs
//! Keycloak and VoterView. The event is committed with the vote before that step
//! runs, as the first test shows, so a failure there leaves it in the outbox.

use super::*;
use crate::services::celery_app::Queue;
use crate::services::electoral_log_outbox::{
    celery_body, deliver_log_event, publish_and_remove, publish_log_event,
    relay_configured_log_events, relay_log_events, store_log_event, OUTBOX_RELAY_PUBLISH_BUDGET,
};
use crate::services::electoral_log_queue::{
    drain_log_event_queue, AmqpLogEventQueue, BoardDelivery, LogEventPreparation, LogEventStore,
};
use crate::services::protocol_manager::create_protocol_manager_keys;
use crate::tasks::electoral_log::persist_board_deliveries;
use async_trait::async_trait;
use electoral_log::client::board_client::{BoardClient, DeliveryReceipt, ElectoralLogMessage};
use lapin::{
    options::{BasicGetOptions, QueueDeclareOptions},
    types::FieldTable,
    uri::AMQPUri,
    Connection, ConnectionProperties,
};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use strand::serialization::StrandDeserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const VOTER: &str = "synthetic-voter";

async fn test_client() -> DbClient {
    let config: tokio_postgres::Config = std::env::var("CAST_VOTE_TEST_DATABASE_URL")
        .expect("CAST_VOTE_TEST_DATABASE_URL")
        .parse()
        .unwrap();
    deadpool_postgres::Pool::builder(deadpool_postgres::Manager::new(
        config,
        tokio_postgres::NoTls,
    ))
    .max_size(2)
    .build()
    .unwrap()
    .get()
    .await
    .unwrap()
}

/// An open election allowing one vote, its event, area and protocol manager
/// key, committed so that the vote's own transaction sees them.
struct Fixture {
    tenant: String,
    event: String,
    election: uuid::Uuid,
    area: String,
    board: String,
}

impl Fixture {
    async fn create(client: &mut DbClient) -> Self {
        let (tenant, event, election, area) = (
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        );
        let board = format!("synthetic{}", event.simple());
        let transaction = client.transaction().await.unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                &[&tenant, &tenant.to_string()],
            )
            .await
            .unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election_event \
                 (id, tenant_id, name, encryption_protocol) VALUES ($1, $2, 'synthetic', 'synthetic')",
                &[&event, &tenant],
            )
            .await
            .unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id) \
                 VALUES ($1, $2, $3)",
                &[&area, &tenant, &event],
            )
            .await
            .unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election \
                 (id, tenant_id, election_event_id, name, status, voting_channels, \
                  num_allowed_revotes) VALUES ($1, $2, $3, 'synthetic', $4, $5, 1)",
                &[
                    &election,
                    &tenant,
                    &event,
                    &json!({"voting_status": "OPEN"}),
                    &json!({"online": true}),
                ],
            )
            .await
            .unwrap();
        create_protocol_manager_keys(
            &transaction,
            &tenant.to_string(),
            &event.to_string(),
            &board,
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        Self {
            tenant: tenant.to_string(),
            event: event.to_string(),
            election,
            area: area.to_string(),
            board,
        }
    }

    fn election_event(&self) -> ElectionEvent {
        ElectionEvent {
            id: self.event.clone(),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            tenant_id: self.tenant.clone(),
            name: "synthetic".to_string(),
            description: None,
            presentation: None,
            bulletin_board_reference: None,
            is_archived: false,
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: "synthetic".to_string(),
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            alias: None,
            statistics: None,
        }
    }

    /// Removes everything the fixture and its votes stored.
    async fn remove(&self, client: &DbClient) {
        for table in [
            "cast_vote",
            "electoral_log_outbox",
            "secret",
            "area",
            "election",
        ] {
            client
                .execute(
                    &format!(
                        "DELETE FROM sequent_backend.{table} \
                         WHERE election_event_id = $1::text::uuid"
                    ),
                    &[&self.event],
                )
                .await
                .unwrap();
        }
        client
            .execute(
                "DELETE FROM sequent_backend.election_event WHERE id = $1::text::uuid",
                &[&self.event],
            )
            .await
            .unwrap();
        client
            .execute(
                "DELETE FROM sequent_backend.tenant WHERE id = $1::text::uuid",
                &[&self.tenant],
            )
            .await
            .unwrap();
    }

    async fn count(&self, client: &DbClient, table: &str) -> i64 {
        client
            .query_one(
                &format!(
                    "SELECT count(*) FROM sequent_backend.{table} \
                     WHERE election_event_id = $1::text::uuid"
                ),
                &[&self.event],
            )
            .await
            .unwrap()
            .get(0)
    }

    async fn system_key(&self, transaction: &Transaction<'_>) -> StrandSignatureSk {
        get_protocol_manager::<RistrettoCtx>(
            transaction,
            &self.tenant,
            Some(&self.event),
            &self.board,
        )
        .await
        .unwrap()
        .get_signing_key()
        .clone()
    }

    /// Casts a synthetic vote through the production insert and commit, with
    /// the electoral log prepared as `try_insert_cast_vote` prepares it.
    async fn cast(
        &self,
        client: &mut DbClient,
        voter_signing_key: &Option<StrandSignatureSk>,
    ) -> Result<(CastVote, PendingLogEvent), CastVoteError> {
        self.cast_as(client, voter_signing_key, VOTER).await
    }

    async fn cast_as(
        &self,
        client: &mut DbClient,
        voter_signing_key: &Option<StrandSignatureSk>,
        voter: &str,
    ) -> Result<(CastVote, PendingLogEvent), CastVoteError> {
        let transaction = client.transaction().await.unwrap();
        let electoral_log = ElectoralLog::for_voter(
            &transaction,
            &self.board,
            &self.tenant,
            &self.event,
            voter,
            voter_signing_key,
        )
        .await
        .unwrap();
        let signing_key = self.system_key(&transaction).await;
        let election = self.election.to_string();
        let pseudonym_h = PseudonymHash(HashWrapper::new(hash_voter_id(voter).unwrap()));
        let vote_h = CastVoteHash(HashWrapper::new([7u8; 64]));
        let username = Some(voter.to_string());
        let audit = CastVoteAudit {
            electoral_log: &electoral_log,
            tenant_id: &self.tenant,
            election_event_id: &self.event,
            election_id: &election,
            pseudonym_h: &pseudonym_h,
            vote_h: &vote_h,
            ip: "ip: ",
            country: "country: ",
            voter_id: voter,
            username: &username,
            area_id: &self.area,
        };
        insert_cast_vote_and_commit(
            InsertCastVoteInput {
                ballot_id: "synthetic-ballot".to_string(),
                election_id: self.election,
                content: "synthetic-encrypted-ballot".to_string(),
            },
            transaction,
            self.election_event(),
            VotingStatusChannel::ONLINE,
            CastVoteIds {
                election_event_id: &self.event,
                tenant_id: &self.tenant,
                voter_id: voter,
                area_id: &self.area,
            },
            signing_key,
            &Some(ISO8601::now().timestamp_millis()),
            &None,
            &None,
            voter_signing_key,
            &audit,
        )
        .await
    }
}

fn sender_of(pending: &PendingLogEvent) -> String {
    serde_json::from_str::<ElectoralLogMessage>(&pending.input.body.as_raw())
        .unwrap()
        .sender_pk
}

#[tokio::test]
#[ignore = "requires the disposable database fixture"]
async fn accepted_vote_commits_with_its_signed_event_and_a_refused_vote_with_none() {
    let mut client = test_client().await;
    let fixture = Fixture::create(&mut client).await;

    let (_, pending) = fixture.cast(&mut client, &None).await.unwrap();
    let stored: serde_json::Value = client
        .query_one(
            "SELECT input FROM sequent_backend.electoral_log_outbox WHERE id = $1",
            &[&pending.id],
        )
        .await
        .unwrap()
        .get(0);
    let message: ElectoralLogMessage = serde_json::from_str(&pending.input.body.as_raw()).unwrap();
    println!(
        "accepted: votes {}, outbox events {}, kind {}",
        fixture.count(&client, "cast_vote").await,
        fixture.count(&client, "electoral_log_outbox").await,
        message.statement_kind
    );
    assert_eq!(stored, serde_json::to_value(&pending.input).unwrap());
    assert_eq!(message.statement_kind, "CastVote");
    assert_eq!(fixture.count(&client, "cast_vote").await, 1);
    assert_eq!(fixture.count(&client, "electoral_log_outbox").await, 1);

    // One vote is allowed: the second is refused and stores no event.
    let refused = fixture.cast(&mut client, &None).await;
    println!(
        "refused: {:?}, votes {}, outbox events {}",
        refused.as_ref().err(),
        fixture.count(&client, "cast_vote").await,
        fixture.count(&client, "electoral_log_outbox").await
    );
    assert!(refused.is_err());
    assert_eq!(fixture.count(&client, "cast_vote").await, 1);
    assert_eq!(fixture.count(&client, "electoral_log_outbox").await, 1);
    fixture.remove(&client).await;
}

#[tokio::test]
#[ignore = "requires the disposable database fixture"]
async fn the_event_is_signed_by_the_voter_key_when_there_is_one() {
    let mut client = test_client().await;
    let voter_key = StrandSignatureSk::gen().unwrap();
    let with_voter_key = Fixture::create(&mut client).await;
    let (_, signed_by_voter) = with_voter_key
        .cast(&mut client, &Some(voter_key.clone()))
        .await
        .unwrap();
    let without = Fixture::create(&mut client).await;
    let (_, signed_by_system) = without.cast(&mut client, &None).await.unwrap();

    let transaction = client.transaction().await.unwrap();
    let system_key = without.system_key(&transaction).await;
    let voter_pk = strand::signature::StrandSignaturePk::from_sk(&voter_key)
        .unwrap()
        .to_der_b64_string()
        .unwrap();
    let system_pk = strand::signature::StrandSignaturePk::from_sk(&system_key)
        .unwrap()
        .to_der_b64_string()
        .unwrap();
    let system_pk_of =
        |key: &StrandSignatureSk| strand::signature::StrandSignaturePk::from_sk(key).unwrap();
    let verified = |pending: &PendingLogEvent, system: &StrandSignatureSk| {
        let stored: ElectoralLogMessage =
            serde_json::from_str(&pending.input.body.as_raw()).unwrap();
        electoral_log::messages::message::Message::strand_deserialize(&stored.message)
            .unwrap()
            .verify(&system_pk_of(system))
    };
    let with_voter_system_key = with_voter_key.system_key(&transaction).await;
    let voter_signed = verified(&signed_by_voter, &with_voter_system_key);
    let system_signed = verified(&signed_by_system, &system_key);
    println!(
        "sender with a voter key is the voter: {}; without one, the system: {}; \
         signatures verify: {:?} {:?}",
        sender_of(&signed_by_voter) == voter_pk,
        sender_of(&signed_by_system) == system_pk,
        voter_signed.as_ref().map_err(|err| err.to_string()),
        system_signed.as_ref().map_err(|err| err.to_string())
    );
    transaction.rollback().await.unwrap();
    with_voter_key.remove(&client).await;
    without.remove(&client).await;
    assert!(voter_signed.is_ok());
    assert!(system_signed.is_ok());
    assert_eq!(sender_of(&signed_by_voter), voter_pk);
    assert_eq!(sender_of(&signed_by_system), system_pk);
}

/// A vote whose INSERT fails after its signed event was stored leaves neither.
#[tokio::test]
#[ignore = "requires the disposable database fixture"]
async fn a_vote_insert_failing_after_its_event_is_stored_leaves_neither() {
    const REFUSED_VOTER: &str = "synthetic-voter-refused-by-the-database";
    const REFUSAL: &str = "synthetic refusal of the vote INSERT";
    // Not rolled back with the vote: counts the INSERTs the trigger refused.
    const REFUSALS: &str = "sequent_backend.synthetic_vote_refusals";
    let mut client = test_client().await;
    let fixture = Fixture::create(&mut client).await;
    // Only this test's synthetic voter is refused, and only while it runs.
    client
        .batch_execute(&format!(
            "DROP SEQUENCE IF EXISTS {REFUSALS}; CREATE SEQUENCE {REFUSALS}; \
             CREATE OR REPLACE FUNCTION sequent_backend.synthetic_refuse_vote() RETURNS trigger \
             LANGUAGE plpgsql AS $$ BEGIN \
             IF NEW.voter_id_string = '{REFUSED_VOTER}' THEN \
             PERFORM nextval('{REFUSALS}'); \
             RAISE EXCEPTION '{REFUSAL}'; END IF; \
             RETURN NEW; END $$; \
             DROP TRIGGER IF EXISTS synthetic_refuse_vote ON sequent_backend.cast_vote; \
             CREATE TRIGGER synthetic_refuse_vote BEFORE INSERT ON sequent_backend.cast_vote \
             FOR EACH ROW EXECUTE FUNCTION sequent_backend.synthetic_refuse_vote();"
        ))
        .await
        .unwrap();

    let refused = fixture.cast_as(&mut client, &None, REFUSED_VOTER).await;
    let (votes, events) = (
        fixture.count(&client, "cast_vote").await,
        fixture.count(&client, "electoral_log_outbox").await,
    );
    // Only the injected refusal, raised by the vote INSERT after the event was
    // stored, shows its rollback; an earlier failure would not reach the trigger.
    let trigger_refusals: i64 = client
        .query_one(
            &format!("SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM {REFUSALS}"),
            &[],
        )
        .await
        .unwrap()
        .get(0);
    let insert_failed = matches!(&refused, Err(CastVoteError::InsertFailed(_)));
    println!(
        "vote INSERT failed: {insert_failed}; INSERTs refused by the injected trigger \
         {trigger_refusals}; votes {votes}, outbox events {events}"
    );

    client
        .batch_execute(&format!(
            "DROP TRIGGER synthetic_refuse_vote ON sequent_backend.cast_vote; \
                 DROP FUNCTION sequent_backend.synthetic_refuse_vote(); DROP SEQUENCE {REFUSALS};"
        ))
        .await
        .unwrap();
    fixture.remove(&client).await;
    assert!(insert_failed, "{refused:?}");
    assert_eq!(trigger_refusals, 1);
    assert_eq!(votes, 0);
    assert_eq!(events, 0);
}

struct SyntheticBoard(String);

#[async_trait]
impl LogEventStore for SyntheticBoard {
    type Message = ElectoralLogMessage;

    async fn prepare(
        &mut self,
        events: &[LogEventInput],
    ) -> anyhow::Result<Vec<LogEventPreparation<ElectoralLogMessage>>> {
        events
            .iter()
            .map(|event| {
                Ok(LogEventPreparation::Ready {
                    board: self.0.clone(),
                    messages: vec![serde_json::from_str(&event.body.as_raw())?],
                })
            })
            .collect()
    }

    async fn persist(
        &mut self,
        board: &str,
        deliveries: Vec<BoardDelivery<ElectoralLogMessage>>,
    ) -> anyhow::Result<Vec<DeliveryReceipt>> {
        persist_board_deliveries(&mut immudb().await, board, &deliveries).await
    }
}

async fn immudb() -> BoardClient {
    let url =
        std::env::var("ELECTORAL_LOG_TEST_IMMUDB_URL").expect("ELECTORAL_LOG_TEST_IMMUDB_URL");
    BoardClient::new(&url, "immudb", "immudb").await.unwrap()
}

async fn rabbitmq() -> Connection {
    let url = std::env::var("ELECTORAL_LOG_TEST_AMQP_URL").expect("ELECTORAL_LOG_TEST_AMQP_URL");
    Connection::connect(&url, ConnectionProperties::default())
        .await
        .unwrap()
}

async fn queue_ready(connection: &Connection, queue: &str) -> u32 {
    connection
        .create_channel()
        .await
        .unwrap()
        .queue_declare(
            queue,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap()
        .message_count()
}

#[tokio::test]
#[ignore = "requires the disposable database fixture, RabbitMQ and immudb"]
async fn vote_accepted_while_rabbitmq_is_down_reaches_its_board_once() {
    let mut client = test_client().await;
    let fixture = Fixture::create(&mut client).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let board = format!("synthetic{suffix}");
    let queue_name = format!("synthetic-{suffix}_electoral_log_event_queue");
    let dead_letter = format!("{queue_name}_dead_letter");
    immudb()
        .await
        .upsert_electoral_log_db(&board)
        .await
        .unwrap();

    let (_, pending) = fixture.cast(&mut client, &None).await.unwrap();
    // The broker closes the connection before the event is published. No
    // database client is taken for a publish RabbitMQ did not confirm.
    let lost = rabbitmq().await;
    lost.close(320, "CONNECTION_FORCED").await.unwrap();
    let mut client_taken = false;
    let delivered = publish_and_remove(&lost, &queue_name, &pending, || {
        client_taken = true;
        async { Ok(test_client().await) }
    })
    .await;
    println!(
        "after the vote, broker connection closed: delivery {:?}; database client taken {client_taken}; votes {}, outbox events {}",
        delivered.as_ref().map_err(|err| err.to_string()),
        fixture.count(&client, "cast_vote").await,
        fixture.count(&client, "electoral_log_outbox").await
    );
    assert!(delivered.is_err());
    assert!(!client_taken);
    assert_eq!(fixture.count(&client, "cast_vote").await, 1);
    assert_eq!(fixture.count(&client, "electoral_log_outbox").await, 1);

    // A publish is confirmed but its process stops before removing the row;
    // the relay publishes it again under the same ID and removes it.
    let connection = rabbitmq().await;
    publish_log_event(&connection, &queue_name, &pending)
        .await
        .unwrap();
    let pending_after_publish = fixture.count(&client, "electoral_log_outbox").await;
    // The relay takes every row of the database: this test's must be the only one.
    let all_rows: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.electoral_log_outbox",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(all_rows, 1, "another test left rows in the outbox");
    let relayed = relay_log_events(
        &mut client,
        &connection,
        &queue_name,
        Duration::ZERO,
        500,
        OUTBOX_RELAY_PUBLISH_BUDGET,
    )
    .await
    .unwrap();
    println!(
        "publish confirmed, process stopped before removal: outbox events {pending_after_publish}; relayed {relayed}; outbox events {}",
        fixture.count(&client, "electoral_log_outbox").await
    );
    assert_eq!(pending_after_publish, 1);
    assert_eq!(relayed, 1);
    assert_eq!(fixture.count(&client, "electoral_log_outbox").await, 0);

    let mut queue = AmqpLogEventQueue::new(
        connection.create_channel().await.unwrap(),
        queue_name.clone(),
        dead_letter.clone(),
    )
    .await
    .unwrap();
    drain_log_event_queue(&mut queue, &mut SyntheticBoard(board.clone()), 10)
        .await
        .unwrap();
    let rows = immudb()
        .await
        .get_electoral_log_messages(&board)
        .await
        .unwrap();
    let signed: ElectoralLogMessage = serde_json::from_str(&pending.input.body.as_raw()).unwrap();
    println!(
        "board rows {}: kinds {:?}; signed statement preserved {}",
        rows.len(),
        rows.iter()
            .map(|row| row.statement_kind.as_str())
            .collect::<Vec<_>>(),
        rows.first()
            .map(|row| row.message == signed.message)
            .unwrap_or(false)
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].statement_kind, "CastVote");
    assert_eq!(rows[0].message, signed.message);

    immudb().await.delete_database(&board).await.unwrap();
    let channel = connection.create_channel().await.unwrap();
    for queue in [queue_name, dead_letter] {
        channel
            .queue_delete(&queue, Default::default())
            .await
            .unwrap();
    }
    fixture.remove(&client).await;
}

/// What the proxy does with the broker's publisher confirms.
#[derive(Clone, Copy)]
enum Confirms {
    Forward,
    Delay(Duration),
    Withhold,
}

/// A local proxy to RabbitMQ that forwards every frame both ways, except the
/// broker's publisher confirms (`basic.ack`), which it handles as `confirms`
/// says.
struct ConfirmProxy {
    url: String,
    confirms: Arc<Mutex<Confirms>>,
    withheld: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl ConfirmProxy {
    fn set(&self, confirms: Confirms) {
        *self.confirms.lock().unwrap() = confirms;
    }

    fn withheld(&self) -> usize {
        self.withheld.load(Ordering::SeqCst)
    }
}

impl Drop for ConfirmProxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Forwards AMQP frames from the broker. From the broker, `basic.ack` (class
/// 60, method 80) is only sent to confirm a publish. Delayed frames hold back
/// the ones behind them, so the order is kept.
async fn forward_frames(
    mut broker: tokio::net::tcp::OwnedReadHalf,
    mut client: tokio::net::tcp::OwnedWriteHalf,
    confirms: Arc<Mutex<Confirms>>,
    withheld: Arc<AtomicUsize>,
) -> std::io::Result<()> {
    loop {
        let mut header = [0; 7];
        broker.read_exact(&mut header).await?;
        let size = u32::from_be_bytes([header[3], header[4], header[5], header[6]]) as usize;
        // The payload and the frame-end octet.
        let mut rest = vec![0; size + 1];
        broker.read_exact(&mut rest).await?;
        if header[0] == 1 && size >= 4 && rest[..4] == [0, 60, 0, 80] {
            let confirms = *confirms.lock().unwrap();
            match confirms {
                Confirms::Forward => {}
                Confirms::Delay(delay) => tokio::time::sleep(delay).await,
                Confirms::Withhold => {
                    withheld.fetch_add(1, Ordering::SeqCst);
                    continue;
                }
            }
        }
        client.write_all(&header).await?;
        client.write_all(&rest).await?;
    }
}

async fn confirm_proxy(confirms: Confirms) -> ConfirmProxy {
    let url = std::env::var("ELECTORAL_LOG_TEST_AMQP_URL").expect("ELECTORAL_LOG_TEST_AMQP_URL");
    let uri: AMQPUri = url.parse().unwrap();
    let broker = format!("{}:{}", uri.authority.host, uri.authority.port);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let proxied = url.replacen(&format!("@{broker}"), &format!("@{address}"), 1);
    let confirms = Arc::new(Mutex::new(confirms));
    let withheld = Arc::new(AtomicUsize::new(0));
    let task = tokio::spawn({
        let (confirms, withheld) = (confirms.clone(), withheld.clone());
        async move {
            let mut connections = tokio::task::JoinSet::new();
            while let Ok((client, _)) = listener.accept().await {
                let upstream = tokio::net::TcpStream::connect(&broker).await.unwrap();
                let (mut from_client, to_client) = client.into_split();
                let (from_broker, mut to_broker) = upstream.into_split();
                connections.spawn(async move {
                    tokio::io::copy(&mut from_client, &mut to_broker)
                        .await
                        .map(|_| ())
                });
                connections.spawn(forward_frames(
                    from_broker,
                    to_client,
                    confirms.clone(),
                    withheld.clone(),
                ));
            }
        }
    });
    ConfirmProxy {
        url: proxied,
        confirms,
        withheld,
        task,
    }
}

/// Whether another connection can lock the outbox row within a second.
async fn lockable(client: &mut DbClient, id: &Uuid) -> bool {
    let transaction = client.transaction().await.unwrap();
    transaction
        .batch_execute("SET LOCAL lock_timeout = '1s'")
        .await
        .unwrap();
    let locked = transaction
        .query(
            "SELECT id FROM sequent_backend.electoral_log_outbox WHERE id = $1 FOR UPDATE",
            &[id],
        )
        .await
        .map(|rows| rows.len() == 1);
    transaction.rollback().await.unwrap();
    locked.unwrap_or(false)
}

/// Makes outbox rows old enough for any relay, the first one oldest.
async fn age(client: &DbClient, ids: &[Uuid]) {
    for (position, id) in ids.iter().enumerate() {
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox \
                 SET created_at = now() - interval '10 years' + make_interval(secs => $2) \
                 WHERE id = $1",
                &[id, &(position as f64)],
            )
            .await
            .unwrap();
    }
}

/// The correlation IDs and bodies of the messages ready in a queue, which are
/// put back once read.
async fn ready_messages(connection: &Connection, queue: &str) -> Vec<(String, Vec<u8>)> {
    let channel = connection.create_channel().await.unwrap();
    let mut messages = Vec::new();
    while let Some(message) = channel
        .basic_get(queue, BasicGetOptions::default())
        .await
        .unwrap()
    {
        let id = message
            .delivery
            .properties
            .correlation_id()
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default();
        messages.push((id, message.delivery.data));
    }
    // Closing the channel puts the unacknowledged messages back.
    channel.close(200, "OK").await.unwrap();
    messages
}

/// The production bounds with a second's margin: `OUTBOX_DELIVERY_TIMEOUT`,
/// and for a relay run the sum of its connections (5 s), row selection
/// (5 s), publishing (30 s) and removal (5 s) limits.
const DELIVERY_BOUND: Duration = Duration::from_secs(6);
const RELAY_BOUND: Duration = Duration::from_secs(46);
/// How long the test waits for a delivery or relay that is not bounded.
const HARNESS: Duration = Duration::from_secs(50);

/// Runs `work` for at most `HARNESS`, returning how long it took, or `None`
/// if it was still waiting.
async fn timed<T>(work: impl std::future::Future<Output = T>) -> (Option<T>, Duration) {
    let started = Instant::now();
    let result = tokio::time::timeout(HARNESS, work).await.ok();
    (result, started.elapsed())
}

/// Drives the production callers, `deliver_log_event` and
/// `relay_configured_log_events`, configured through the environment as in a
/// worker, with RabbitMQ withholding its confirmations.
#[tokio::test]
#[ignore = "requires the disposable database fixture, HASURA_DB__*, RabbitMQ and immudb"]
async fn vote_accepted_while_rabbitmq_withholds_confirms_reaches_its_board_once() {
    let mut client = test_client().await;
    let fixture = Fixture::create(&mut client).await;
    let suffix = Uuid::new_v4().simple().to_string();
    let board = format!("synthetic{suffix}");
    let slug = format!("synthetic-{suffix}");
    let queue_name = Queue::ElectoralLogEvent.queue_name(&slug);
    let dead_letter = format!("{queue_name}_dead_letter");
    immudb()
        .await
        .upsert_electoral_log_db(&board)
        .await
        .unwrap();
    let direct = rabbitmq().await;
    let proxy = confirm_proxy(Confirms::Withhold).await;
    // The workers' connection goes through the proxy.
    std::env::set_var("AMQP_ADDR", &proxy.url);
    std::env::set_var("ENV_SLUG", &slug);

    let (_, pending) = fixture.cast(&mut client, &None).await.unwrap();

    // Without a confirmation, publishing takes no database client.
    let proxied = Connection::connect(&proxy.url, ConnectionProperties::default())
        .await
        .unwrap();
    let mut client_taken = false;
    let unconfirmed = tokio::time::timeout(
        Duration::from_secs(2),
        publish_and_remove(&proxied, &queue_name, &pending, || {
            client_taken = true;
            async { Ok(test_client().await) }
        }),
    )
    .await;
    println!(
        "confirms withheld, publish and remove: still waiting {}; database client taken {client_taken}",
        unconfirmed.is_err()
    );

    // RabbitMQ stores the event but its confirmation does not arrive. The
    // vote's delivery returns in bounded time and leaves the event in the
    // outbox.
    let (delivered, delivery_elapsed) = timed(deliver_log_event(&pending)).await;
    let withheld_after_delivery = proxy.withheld();
    let queued_after_delivery = queue_ready(&direct, &queue_name).await;
    let outbox_after_delivery = fixture.count(&client, "electoral_log_outbox").await;
    println!(
        "confirms withheld, vote delivery: returned {} after {delivery_elapsed:?}; confirms withheld {withheld_after_delivery}; \
         queued {queued_after_delivery}; votes {}, outbox events {outbox_after_delivery}",
        delivered.is_some(),
        fixture.count(&client, "cast_vote").await,
    );

    // The relay takes the row under its lock and publishes it again. It also
    // returns in bounded time, and its lock is released by then.
    age(&client, &[pending.id]).await;
    let mut probe_client = test_client().await;
    let relay_done = tokio::sync::Notify::new();
    let ((relayed, relay_elapsed), lockable_after_relay) = tokio::join!(
        async {
            let relayed = timed(relay_configured_log_events()).await;
            relay_done.notify_one();
            relayed
        },
        async {
            let _ = tokio::time::timeout(RELAY_BOUND, relay_done.notified()).await;
            lockable(&mut probe_client, &pending.id).await
        }
    );
    let withheld_after_relay = proxy.withheld();
    let outbox_after_relay = fixture.count(&client, "electoral_log_outbox").await;
    println!(
        "confirms withheld, relay: returned {:?} after {relay_elapsed:?}; row lockable by another connection {lockable_after_relay}; \
         confirms withheld {withheld_after_relay}; outbox events {outbox_after_relay}",
        relayed.as_ref().map(|relayed| relayed.as_ref().map_err(|err| err.to_string())),
    );

    // Confirmations are back. The vote's event is published again, under
    // the same ID and with the same payload, and removed.
    proxy.set(Confirms::Forward);
    let (redelivered, redelivery_elapsed) = timed(deliver_log_event(&pending)).await;
    let outbox_after_redelivery = fixture.count(&client, "electoral_log_outbox").await;
    let copies = ready_messages(&direct, &queue_name).await;
    let body = celery_body(&pending.input).unwrap();
    let same_event = copies
        .iter()
        .all(|(id, data)| id == &pending.id.to_string() && data == &body);
    println!(
        "confirms restored, vote delivery: returned {} after {redelivery_elapsed:?}; outbox events {outbox_after_redelivery}; \
         queued copies {}, all with the outbox ID and payload {same_event}",
        redelivered.is_some(),
        copies.len(),
    );

    // The receipt-backed consumer writes the event once.
    let mut queue = AmqpLogEventQueue::new(
        direct.create_channel().await.unwrap(),
        queue_name.clone(),
        dead_letter.clone(),
    )
    .await
    .unwrap();
    drain_log_event_queue(&mut queue, &mut SyntheticBoard(board.clone()), 10)
        .await
        .unwrap();
    let queued_after_drain = queue_ready(&direct, &queue_name).await;
    let rows = immudb()
        .await
        .get_electoral_log_messages(&board)
        .await
        .unwrap();
    let signed: ElectoralLogMessage = serde_json::from_str(&pending.input.body.as_raw()).unwrap();
    println!(
        "board rows {}: kinds {:?}; signed statement preserved {}; queued {queued_after_drain}",
        rows.len(),
        rows.iter()
            .map(|row| row.statement_kind.as_str())
            .collect::<Vec<_>>(),
        rows.first()
            .map(|row| row.message == signed.message)
            .unwrap_or(false)
    );

    immudb().await.delete_database(&board).await.unwrap();
    let channel = direct.create_channel().await.unwrap();
    for queue in [queue_name, dead_letter] {
        channel
            .queue_delete(&queue, Default::default())
            .await
            .unwrap();
    }
    let votes = fixture.count(&client, "cast_vote").await;
    fixture.remove(&client).await;

    assert!(unconfirmed.is_err());
    assert!(!client_taken);
    assert!(delivered.is_some());
    assert!(delivery_elapsed < DELIVERY_BOUND);
    assert!(withheld_after_delivery >= 2);
    assert_eq!(queued_after_delivery, 2);
    assert_eq!(outbox_after_delivery, 1);
    assert!(matches!(relayed, Some(Err(_))));
    assert!(relay_elapsed < RELAY_BOUND);
    assert!(lockable_after_relay);
    assert!(withheld_after_relay > withheld_after_delivery);
    assert_eq!(outbox_after_relay, 1);
    assert!(redelivered.is_some());
    assert_eq!(outbox_after_redelivery, 0);
    assert_eq!(copies.len(), 4);
    assert!(same_event);
    assert_eq!(queued_after_drain, 0);
    assert_eq!(votes, 1);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].statement_kind, "CastVote");
    assert_eq!(rows[0].message, signed.message);
}

/// RabbitMQ confirms, but more slowly than one relay run's budget allows for
/// all its rows. Each run removes the events confirmed within it, so the
/// outbox empties across runs.
#[tokio::test]
#[ignore = "requires the disposable database fixture and RabbitMQ"]
async fn relay_removes_the_events_confirmed_within_its_budget() {
    let mut client = test_client().await;
    let fixture = Fixture::create(&mut client).await;
    let queue_name = format!(
        "synthetic-{}_electoral_log_event_queue",
        Uuid::new_v4().simple()
    );
    let (_, pending) = fixture.cast(&mut client, &None).await.unwrap();
    let mut ids = vec![pending.id];
    let transaction = client.transaction().await.unwrap();
    for _ in 0..2 {
        let stored = store_log_event(
            &transaction,
            &fixture.tenant.parse().unwrap(),
            &fixture.event.parse().unwrap(),
            pending.input.clone(),
        )
        .await
        .unwrap();
        ids.push(stored.id);
    }
    transaction.commit().await.unwrap();
    age(&client, &ids).await;
    let direct = rabbitmq().await;
    let proxy = confirm_proxy(Confirms::Delay(Duration::from_secs(1))).await;
    let proxied = Connection::connect(&proxy.url, ConnectionProperties::default())
        .await
        .unwrap();

    let budget = Duration::from_millis(2500);
    let mut runs = Vec::new();
    for _ in 0..3 {
        let relayed = relay_log_events(
            &mut client,
            &proxied,
            &queue_name,
            Duration::ZERO,
            ids.len() as i64,
            budget,
        )
        .await;
        let remaining = fixture.count(&client, "electoral_log_outbox").await;
        println!(
            "confirms delayed 1s, relay budget {budget:?}: relayed {:?}; outbox events {remaining}",
            relayed.as_ref().map_err(|err| err.to_string())
        );
        runs.push((relayed.ok(), remaining));
        if remaining == 0 {
            break;
        }
    }
    proxy.set(Confirms::Forward);
    let copies = ready_messages(&direct, &queue_name).await;
    let mut published: Vec<String> = copies.iter().map(|(id, _)| id.clone()).collect();
    published.sort();
    published.dedup();
    let mut expected: Vec<String> = ids.iter().map(Uuid::to_string).collect();
    expected.sort();
    println!(
        "queued copies {}, distinct IDs {} of the {} stored",
        copies.len(),
        published.len(),
        expected.len()
    );

    direct
        .create_channel()
        .await
        .unwrap()
        .queue_delete(&queue_name, Default::default())
        .await
        .unwrap();
    fixture.remove(&client).await;

    // The first run publishes two events within its budget and removes
    // them; the third, maybe stored but unconfirmed, stays for the next run.
    assert_eq!(runs[0], (None, 1));
    assert_eq!(runs[1], (Some(1), 0));
    assert_eq!(runs.len(), 2);
    assert_eq!(published, expected);
}
