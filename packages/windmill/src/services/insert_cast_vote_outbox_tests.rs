// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signed CastVote of an accepted vote, stored with the vote, against a
//! disposable PostgreSQL database with this branch's schema (and RabbitMQ and
//! immudb for the delivery test), with synthetic data only. Run with
//! `CAST_VOTE_TEST_DATABASE_URL=… MASTER_SECRET=<64 hex digits>
//! [ELECTORAL_LOG_TEST_AMQP_URL=… ELECTORAL_LOG_TEST_IMMUDB_URL=…]
//! cargo test -p windmill --lib services::insert_cast_vote::outbox_tests -- --include-ignored`.

use super::*;
use crate::services::electoral_log_outbox::{
    publish_and_remove, publish_log_event, relay_log_events,
};
use crate::services::electoral_log_queue::{
    drain_log_event_queue, AmqpLogEventQueue, BoardDelivery, LogEventPreparation, LogEventStore,
};
use crate::services::protocol_manager::create_protocol_manager_keys;
use crate::tasks::electoral_log::persist_board_deliveries;
use async_trait::async_trait;
use electoral_log::client::board_client::{BoardClient, DeliveryReceipt, ElectoralLogMessage};
use lapin::{Connection, ConnectionProperties};
use serde_json::json;

const VOTER: &str = "synthetic-voter";

async fn test_client() -> DbClient {
    let mut config = deadpool_postgres::Config::new();
    config.url =
        Some(std::env::var("CAST_VOTE_TEST_DATABASE_URL").expect("CAST_VOTE_TEST_DATABASE_URL"));
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
        let transaction = client.transaction().await.unwrap();
        let electoral_log = ElectoralLog::for_voter(
            &transaction,
            &self.board,
            &self.tenant,
            &self.event,
            VOTER,
            voter_signing_key,
        )
        .await
        .unwrap();
        let signing_key = self.system_key(&transaction).await;
        let election = self.election.to_string();
        let pseudonym_h = PseudonymHash(HashWrapper::new(hash_voter_id(VOTER).unwrap()));
        let vote_h = CastVoteHash(HashWrapper::new([7u8; 64]));
        let username = Some(VOTER.to_string());
        let audit = CastVoteAudit {
            electoral_log: &electoral_log,
            tenant_id: &self.tenant,
            election_event_id: &self.event,
            election_id: &election,
            pseudonym_h: &pseudonym_h,
            vote_h: &vote_h,
            ip: "ip: ",
            country: "country: ",
            voter_id: VOTER,
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
                voter_id: VOTER,
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
}

#[tokio::test]
#[ignore = "requires the disposable database fixture"]
async fn the_event_is_signed_with_the_voter_key_when_there_is_one() {
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
    println!(
        "sender with a voter key is the voter: {}; without one, the system: {}",
        sender_of(&signed_by_voter) == voter_pk,
        sender_of(&signed_by_system) == system_pk
    );
    assert_eq!(sender_of(&signed_by_voter), voter_pk);
    assert_eq!(sender_of(&signed_by_system), system_pk);
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
    let relayed = relay_log_events(
        &mut client,
        &connection,
        &queue_name,
        std::time::Duration::ZERO,
        500,
    )
    .await
    .unwrap();
    println!(
        "publish confirmed, process stopped before removal: outbox events {pending_after_publish}; relayed {relayed}; outbox events {}",
        fixture.count(&client, "electoral_log_outbox").await
    );
    assert_eq!(pending_after_publish, 1);
    assert!(relayed >= 1);
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
}
