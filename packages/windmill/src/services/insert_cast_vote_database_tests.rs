// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real PostgreSQL contracts. The devenv runner owns the disposable schema;
//! each Rust test creates private fixtures and rolls its transaction back.

use super::super::*;
use serde_json::json;

struct ElectionFixture {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
}

impl ElectionFixture {
    async fn create(transaction: &Transaction<'_>) -> Self {
        let fixture = Self {
            tenant: Uuid::new_v4(),
            event: Uuid::new_v4(),
            election: Uuid::new_v4(),
            area: Uuid::new_v4(),
        };
        transaction
            .execute(
                r#"
            INSERT INTO sequent_backend.election
                (id, tenant_id, election_event_id, num_allowed_revotes, status)
            VALUES ($1, $2, $3, 1, '{"voting_status":"OPEN"}')
            "#,
                &[&fixture.election, &fixture.tenant, &fixture.event],
            )
            .await
            .unwrap();
        fixture
    }

    async fn load(&self, transaction: &Transaction<'_>) -> CastVoteConfiguration {
        get_cast_vote_configuration(
            transaction,
            &self.tenant.to_string(),
            &self.event.to_string(),
            &self.election.to_string(),
        )
        .await
        .unwrap()
    }

    async fn insert(&self, transaction: &Transaction<'_>, content: &str) -> Result<CastVote> {
        postgres::cast_vote::insert_cast_vote(
            transaction,
            &self.tenant,
            &self.event,
            &self.election,
            &self.area,
            content,
            "voter",
            "ballot",
            &[0; 64],
            &None,
            &None,
            VotingStatusChannel::ONLINE,
            CastVoteStatus::Valid,
        )
        .await
    }
}

async fn test_client() -> DbClient {
    let mut config = deadpool_postgres::Config::new();
    config.url = Some(
        std::env::var("CAST_VOTE_TEST_DATABASE_URL")
            .expect("run scripts/test_cast_vote_scalability.py --rust-tests inside devenv"),
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

#[tokio::test]
#[ignore = "requires the disposable devenv database fixture"]
async fn materialized_dates_match_schedule_contract_and_remain_tenant_scoped() {
    let mut client = test_client().await;
    let transaction = client.transaction().await.unwrap();
    let fixture = ElectionFixture::create(&transaction).await;
    let task = generate_manage_date_task_name(
        &fixture.tenant.to_string(),
        &fixture.event.to_string(),
        Some(&fixture.election.to_string()),
        &EventProcessors::END_VOTING_PERIOD,
    );
    transaction
        .execute(
            r#"
        INSERT INTO sequent_backend.scheduled_event
            (tenant_id, election_event_id, task_id, event_payload, cron_config)
        VALUES ($1, $2, $3, $4, $5)
        "#,
            &[
                &fixture.tenant,
                &fixture.event,
                &task,
                &json!({"election_id": fixture.election.to_string()}),
                &json!({"scheduled_date": "2026-10-01T12:00:00Z"}),
            ],
        )
        .await
        .unwrap();

    // The migration projects the exact task/payload matching used by imports.
    let schedules = postgres::scheduled_event::find_scheduled_event_by_election_event_id(
        &transaction,
        &fixture.tenant.to_string(),
        &fixture.event.to_string(),
    )
    .await
    .unwrap();
    let expected = generate_voting_period_dates(
        schedules,
        &fixture.tenant.to_string(),
        &fixture.event.to_string(),
        Some(&fixture.election.to_string()),
    )
    .unwrap();
    let loaded = fixture.load(&transaction).await;
    assert_eq!(loaded.dates.start_date, expected.start_date);
    assert_eq!(loaded.dates.end_date, expected.end_date);

    transaction
        .execute(
            "UPDATE sequent_backend.election SET status = $1 WHERE id = $2",
            &[&json!({"voting_status": "PAUSED"}), &fixture.election],
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.load(&transaction).await.status.unwrap()["voting_status"],
        "PAUSED"
    );

    transaction
        .execute(
            "UPDATE sequent_backend.scheduled_event SET archived_at = now() WHERE task_id = $1",
            &[&task],
        )
        .await
        .unwrap();
    assert!(fixture.load(&transaction).await.dates.end_date.is_none());
    assert!(get_cast_vote_configuration(
        &transaction,
        &Uuid::new_v4().to_string(),
        &fixture.event.to_string(),
        &fixture.election.to_string(),
    )
    .await
    .is_err());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the disposable devenv database fixture"]
async fn insert_preserves_response_and_maps_trigger_error_without_retrying() {
    let mut client = test_client().await;
    let transaction = client.transaction().await.unwrap();
    let fixture = ElectionFixture::create(&transaction).await;
    let content = "encrypted-ballot".repeat(1000);

    let inserted = fixture.insert(&transaction, &content).await.unwrap();
    assert_eq!(inserted.content.as_deref(), Some(content.as_str()));
    assert_eq!(inserted.status, CastVoteStatus::Valid);
    assert_eq!(inserted.area_id, Some(fixture.area.to_string()));
    assert_eq!(inserted.ballot_id.as_deref(), Some("ballot"));

    let error = fixture.insert(&transaction, &content).await.unwrap_err();
    let mapped = map_insert_error(error);
    assert_eq!(
        serde_json::to_value(&mapped).unwrap(),
        "insert_failed_exceeds_allowed_revotes"
    );
    assert!(matches!(
        skip_or_propagate(mapped),
        Ok(InsertCastVoteResult::SkipRetryFailure(
            CastVoteError::InsertFailedExceedsAllowedRevotes
        ))
    ));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the disposable devenv database fixture"]
async fn channel_enablement_and_pause_are_rechecked_from_the_writer() {
    let mut client = test_client().await;
    let transaction = client.transaction().await.unwrap();
    let fixture = ElectionFixture::create(&transaction).await;
    let event = super::election_event(None);
    let auth_time = Some(ISO8601::now().timestamp());
    for (channel, key, status_key) in [
        (VotingStatusChannel::ONLINE, "online", "voting_status"),
        (VotingStatusChannel::KIOSK, "kiosk", "kiosk_voting_status"),
        (
            VotingStatusChannel::TELEPHONE,
            "telephone",
            "telephone_voting_status",
        ),
    ] {
        for (enabled, paused) in [(true, false), (false, false), (true, true), (true, false)] {
            let mut channels = json!({"online": true, "kiosk": true, "telephone": true});
            channels[key] = json!(enabled);
            let mut status = json!({"voting_status": "OPEN", "kiosk_voting_status": "OPEN", "telephone_voting_status": "OPEN"});
            if paused {
                status[status_key] = json!("PAUSED");
            }
            transaction
                .execute(
                    "UPDATE sequent_backend.election SET voting_channels=$1, status=$2 WHERE id=$3",
                    &[&channels, &status, &fixture.election],
                )
                .await
                .unwrap();
            let result = check_status(
                &fixture.tenant.to_string(),
                &fixture.event.to_string(),
                &fixture.election.to_string(),
                &transaction,
                &event,
                &auth_time,
                channel,
                false,
            )
            .await;
            if !enabled {
                assert!(matches!(
                    result,
                    Err(CastVoteError::VotingChannelNotEnabled(_))
                ));
            } else if paused {
                assert!(matches!(result, Err(CastVoteError::CheckStatusFailed(_))));
            } else {
                assert_eq!(result.unwrap(), channel);
            }
        }
    }
    transaction.rollback().await.unwrap();
}

mod electoral_log_outbox {
    use super::*;
    use crate::services::electoral_log_outbox::{
        publish_log_event, relay_log_events, PendingLogEvent,
    };
    use crate::services::electoral_log_queue::{
        drain_log_event_queue, AmqpLogEventQueue, BoardDelivery, LogEventPreparation, LogEventStore,
    };
    use crate::tasks::electoral_log::persist_board_deliveries;
    use async_trait::async_trait;
    use electoral_log::client::board_client::{BoardClient, DeliveryReceipt, ElectoralLogMessage};
    use lapin::{Connection, ConnectionProperties};

    const VOTER: &str = "synthetic-voter";

    /// An open election with the online channel, committed so that the vote's
    /// own transaction sees it.
    async fn open_election(client: &mut DbClient) -> ElectionFixture {
        let transaction = client.transaction().await.unwrap();
        let fixture = ElectionFixture::create(&transaction).await;
        transaction
            .execute(
                "UPDATE sequent_backend.election SET voting_channels = $1 WHERE id = $2",
                &[&json!({"online": true}), &fixture.election],
            )
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        fixture
    }

    async fn remove(client: &DbClient, fixture: &ElectionFixture) {
        for table in ["cast_vote", "electoral_log_outbox", "election"] {
            let column = if table == "election" {
                "id"
            } else {
                "election_event_id"
            };
            let value = if table == "election" {
                fixture.election
            } else {
                fixture.event
            };
            client
                .execute(
                    &format!("DELETE FROM sequent_backend.{table} WHERE {column} = $1"),
                    &[&value],
                )
                .await
                .unwrap();
        }
    }

    async fn count(client: &DbClient, table: &str, fixture: &ElectionFixture) -> i64 {
        client
            .query_one(
                &format!(
                    "SELECT count(*) FROM sequent_backend.{table} WHERE election_event_id = $1"
                ),
                &[&fixture.event],
            )
            .await
            .unwrap()
            .get(0)
    }

    /// Casts a synthetic vote through the production insert and commit.
    async fn cast(
        client: &mut DbClient,
        fixture: &ElectionFixture,
        signing_key: &StrandSignatureSk,
    ) -> Result<(CastVote, VotingStatusChannel, PendingLogEvent), CastVoteError> {
        let tenant = fixture.tenant.to_string();
        let event = fixture.event.to_string();
        let election = fixture.election.to_string();
        let area = fixture.area.to_string();
        let electoral_log =
            ElectoralLog::for_voter_with_signing_key("synthetic", VOTER, signing_key);
        let pseudonym_h = PseudonymHash(HashWrapper::new(hash_voter_id(VOTER).unwrap()));
        let vote_h = CastVoteHash(HashWrapper::new([7u8; 64]));
        let audit = CastVoteAudit {
            electoral_log: &electoral_log,
            tenant_id: &tenant,
            election_event_id: &event,
            election_id: &election,
            pseudonym_h: &pseudonym_h,
            vote_h: &vote_h,
            ip: "ip: ",
            country: "country: ",
            voter_id: VOTER,
            username: &None,
            area_id: &area,
        };
        let transaction = client.transaction().await.unwrap();
        insert_cast_vote_and_commit(
            InsertCastVoteInput {
                ballot_id: "synthetic-ballot".to_string(),
                election_id: fixture.election,
                content: "synthetic-encrypted-ballot".to_string(),
            },
            transaction,
            super::super::election_event(None),
            VotingStatusChannel::ONLINE,
            CastVoteIds {
                election_event_id: &event,
                tenant_id: &tenant,
                voter_id: VOTER,
                area_id: &area,
            },
            signing_key.clone(),
            &Some(ISO8601::now().timestamp()),
            &None,
            &None,
            &None,
            false,
            CastVoteStatus::Valid,
            &audit,
        )
        .await
    }

    #[tokio::test]
    #[ignore = "requires the disposable devenv database fixture"]
    async fn accepted_vote_commits_with_its_signed_event_and_a_refused_vote_with_none() {
        let mut client = test_client().await;
        let fixture = open_election(&mut client).await;
        let signing_key = StrandSignatureSk::gen().unwrap();

        let (_, channel, pending) = cast(&mut client, &fixture, &signing_key).await.unwrap();
        let stored: serde_json::Value = client
            .query_one(
                "SELECT input FROM sequent_backend.electoral_log_outbox WHERE id = $1",
                &[&pending.id],
            )
            .await
            .unwrap()
            .get(0);
        let message: ElectoralLogMessage =
            serde_json::from_str(&pending.input.body.as_raw()).unwrap();
        println!(
            "accepted: channel {channel}, votes {}, outbox events {}, kind {}",
            count(&client, "cast_vote", &fixture).await,
            count(&client, "electoral_log_outbox", &fixture).await,
            message.statement_kind
        );
        assert_eq!(stored, serde_json::to_value(&pending.input).unwrap());
        assert_eq!(message.statement_kind, "CastVote");
        assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
        assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 1);

        // One vote is allowed: the database refuses the second and rolls back
        // the event stored before it.
        let refused = cast(&mut client, &fixture, &signing_key).await;
        println!(
            "refused: {:?}, votes {}, outbox events {}",
            refused.as_ref().err(),
            count(&client, "cast_vote", &fixture).await,
            count(&client, "electoral_log_outbox", &fixture).await
        );
        assert!(matches!(
            refused,
            Err(CastVoteError::InsertFailedExceedsAllowedRevotes)
        ));
        assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
        assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 1);
        remove(&client, &fixture).await;
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

    #[tokio::test]
    #[ignore = "requires the disposable devenv database fixture, RabbitMQ and immudb"]
    async fn vote_accepted_while_rabbitmq_is_down_reaches_its_board_once() {
        let mut client = test_client().await;
        let fixture = open_election(&mut client).await;
        let signing_key = StrandSignatureSk::gen().unwrap();
        let suffix = Uuid::new_v4().simple().to_string();
        let board = format!("synthetic{suffix}");
        let queue_name = format!("synthetic-{suffix}_electoral_log_event_queue");
        immudb()
            .await
            .upsert_electoral_log_db(&board)
            .await
            .unwrap();

        // RabbitMQ is unreachable when the vote is committed.
        std::env::set_var("AMQP_ADDR", "amqp://synthetic:synthetic@127.0.0.1:1/%2f");
        std::env::set_var("ENV_SLUG", format!("synthetic-{suffix}"));
        let (_, _, pending) = cast(&mut client, &fixture, &signing_key).await.unwrap();
        deliver_log_event(&pending).await;
        println!(
            "after the vote, broker down: votes {}, outbox events {}",
            count(&client, "cast_vote", &fixture).await,
            count(&client, "electoral_log_outbox", &fixture).await
        );
        assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
        assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 1);

        // RabbitMQ is back: the relay publishes the event under its outbox ID.
        let amqp =
            std::env::var("ELECTORAL_LOG_TEST_AMQP_URL").expect("ELECTORAL_LOG_TEST_AMQP_URL");
        let connection = Connection::connect(&amqp, ConnectionProperties::default())
            .await
            .unwrap();
        let relayed = relay_log_events(
            &mut client,
            &connection,
            &queue_name,
            std::time::Duration::ZERO,
            500,
        )
        .await
        .unwrap();
        // The relay stopped after publishing and before removing the row: the
        // event is published again under the same ID.
        publish_log_event(&connection, &queue_name, &pending)
            .await
            .unwrap();
        println!(
            "relayed {relayed}; outbox events {}",
            count(&client, "electoral_log_outbox", &fixture).await
        );
        assert_eq!(relayed, 1);
        assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 0);

        let mut queue = AmqpLogEventQueue::new(
            connection.create_channel().await.unwrap(),
            queue_name.clone(),
            format!("{queue_name}_dead_letter"),
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
        let signed: ElectoralLogMessage =
            serde_json::from_str(&pending.input.body.as_raw()).unwrap();
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
        for queue in [queue_name.clone(), format!("{queue_name}_dead_letter")] {
            channel
                .queue_delete(&queue, Default::default())
                .await
                .unwrap();
        }
        remove(&client, &fixture).await;
    }
}
