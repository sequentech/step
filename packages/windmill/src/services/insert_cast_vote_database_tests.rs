// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real PostgreSQL contracts. The devenv runner owns the disposable schema;
//! each Rust test creates private fixtures and rolls its transaction back.

use super::super::*;
use serde_json::json;

pub(in crate::services::insert_cast_vote) struct ElectionFixture {
    pub tenant: Uuid,
    pub event: Uuid,
    pub election: Uuid,
    pub area: Uuid,
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

    async fn insert(
        &self,
        transaction: &Transaction<'_>,
        content: &str,
        ballot_id: &str,
    ) -> Result<CastVote> {
        postgres::cast_vote::insert_cast_vote(
            transaction,
            &self.tenant,
            &self.event,
            &self.election,
            &self.area,
            content,
            "voter",
            ballot_id,
            &[0; 64],
            &None,
            &None,
            VotingStatusChannel::ONLINE,
            CastVoteStatus::Valid,
        )
        .await
    }
}

pub(in crate::services::insert_cast_vote) async fn test_client() -> DbClient {
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

    let inserted = fixture
        .insert(&transaction, &content, "ballot")
        .await
        .unwrap();
    assert_eq!(inserted.content.as_deref(), Some(content.as_str()));
    assert_eq!(inserted.status, CastVoteStatus::Valid);
    assert_eq!(inserted.area_id, Some(fixture.area.to_string()));
    assert_eq!(inserted.ballot_id.as_deref(), Some("ballot"));

    let error = fixture
        .insert(&transaction, &content, "another-ballot")
        .await
        .unwrap_err();
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

pub(in crate::services::insert_cast_vote) mod electoral_log_outbox {
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
    pub(in crate::services::insert_cast_vote) async fn open_election(
        client: &mut DbClient,
    ) -> ElectionFixture {
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

    pub(in crate::services::insert_cast_vote) async fn remove(
        client: &DbClient,
        fixture: &ElectionFixture,
    ) {
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

    pub(in crate::services::insert_cast_vote) async fn count(
        client: &DbClient,
        table: &str,
        fixture: &ElectionFixture,
    ) -> i64 {
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
    pub(in crate::services::insert_cast_vote) async fn cast(
        client: &mut DbClient,
        fixture: &ElectionFixture,
        signing_key: &StrandSignatureSk,
    ) -> Result<(CastVote, VotingStatusChannel, PendingLogEvent), CastVoteError> {
        cast_ballot(client, fixture, signing_key, "synthetic-ballot").await
    }

    /// Casts the synthetic ballot `ballot_id` through the production insert
    /// and commit.
    pub(in crate::services::insert_cast_vote) async fn cast_ballot(
        client: &mut DbClient,
        fixture: &ElectionFixture,
        signing_key: &StrandSignatureSk,
        ballot_id: &str,
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
                ballot_id: ballot_id.to_string(),
                election_id: fixture.election,
                content: format!("encrypted-{ballot_id}"),
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

        // One vote is allowed: the database refuses a second ballot and rolls
        // back the event stored before it.
        let refused = cast_ballot(&mut client, &fixture, &signing_key, "second-ballot").await;
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

    async fn set_allowed_revotes(client: &DbClient, fixture: &ElectionFixture, revotes: i32) {
        client
            .execute(
                "UPDATE sequent_backend.election SET num_allowed_revotes = $1 WHERE id = $2",
                &[&revotes, &fixture.election],
            )
            .await
            .unwrap();
    }

    async fn stored_ballot(
        client: &mut DbClient,
        fixture: &ElectionFixture,
        voter: &str,
        election: &Uuid,
        ballot_id: &str,
    ) -> Option<CastVote> {
        let transaction = client.transaction().await.unwrap();
        let stored = postgres::cast_vote::get_voter_cast_vote_by_ballot_id(
            &transaction,
            &fixture.tenant.to_string(),
            &fixture.event.to_string(),
            &election.to_string(),
            voter,
            ballot_id,
        )
        .await
        .unwrap();
        transaction.rollback().await.unwrap();
        stored
    }

    /// A resent ballot is refused as already cast, whatever the revote limit,
    /// so it neither stores a second vote and event nor uses up a revote.
    #[tokio::test]
    #[ignore = "requires the disposable devenv database fixture"]
    async fn resent_ballot_is_already_cast_and_uses_no_revote() {
        for revotes in [1, 2, 0] {
            let mut client = test_client().await;
            let fixture = open_election(&mut client).await;
            set_allowed_revotes(&client, &fixture, revotes).await;
            let signing_key = StrandSignatureSk::gen().unwrap();

            let (accepted, _, _) = cast_ballot(&mut client, &fixture, &signing_key, "first")
                .await
                .unwrap();
            let resent = cast_ballot(&mut client, &fixture, &signing_key, "first").await;
            assert!(
                matches!(resent, Err(CastVoteError::BallotAlreadyCast)),
                "revotes {revotes}: {:?}",
                resent.err()
            );
            assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
            assert_eq!(count(&client, "electoral_log_outbox", &fixture).await, 1);
            assert_eq!(
                stored_ballot(&mut client, &fixture, VOTER, &fixture.election, "first").await,
                Some(accepted)
            );

            let revote = cast_ballot(&mut client, &fixture, &signing_key, "second").await;
            if revotes == 1 {
                assert!(matches!(
                    revote,
                    Err(CastVoteError::InsertFailedExceedsAllowedRevotes)
                ));
                assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
            } else {
                revote.unwrap();
                assert_eq!(count(&client, "cast_vote", &fixture).await, 2);
            }
            remove(&client, &fixture).await;
        }
    }

    /// The stored vote answers a resend whatever its status, including one
    /// Datafix has not resolved yet and one it discarded.
    #[tokio::test]
    #[ignore = "requires the disposable devenv database fixture"]
    async fn resent_ballot_is_already_cast_in_any_status() {
        for status in [CastVoteStatus::InProgress, CastVoteStatus::Discarded] {
            let mut client = test_client().await;
            let fixture = open_election(&mut client).await;
            let signing_key = StrandSignatureSk::gen().unwrap();
            cast_ballot(&mut client, &fixture, &signing_key, "first")
                .await
                .unwrap();
            client
                .execute(
                    "UPDATE sequent_backend.cast_vote SET status = $1 WHERE election_event_id = $2",
                    &[&status.to_string(), &fixture.event],
                )
                .await
                .unwrap();

            let resent = cast_ballot(&mut client, &fixture, &signing_key, "first").await;
            assert!(
                matches!(resent, Err(CastVoteError::BallotAlreadyCast)),
                "{status}: {:?}",
                resent.err()
            );
            assert_eq!(count(&client, "cast_vote", &fixture).await, 1);
            let stored = stored_ballot(&mut client, &fixture, VOTER, &fixture.election, "first")
                .await
                .unwrap();
            assert_eq!(stored.status, status);
            remove(&client, &fixture).await;
        }
    }

    /// The stored ballot is found only for the voter and election it was cast
    /// in, with the content it was stored with.
    #[tokio::test]
    #[ignore = "requires the disposable devenv database fixture"]
    async fn stored_ballot_is_found_only_for_its_voter_and_election() {
        let mut client = test_client().await;
        let fixture = open_election(&mut client).await;
        let signing_key = StrandSignatureSk::gen().unwrap();
        cast_ballot(&mut client, &fixture, &signing_key, "first")
            .await
            .unwrap();

        let stored = stored_ballot(&mut client, &fixture, VOTER, &fixture.election, "first")
            .await
            .unwrap();
        assert_eq!(stored.ballot_id.as_deref(), Some("first"));
        assert_eq!(stored.voter_id_string.as_deref(), Some(VOTER));
        assert_eq!(stored.content.as_deref(), Some("encrypted-first"));
        assert_eq!(
            stored_ballot(
                &mut client,
                &fixture,
                "another-voter",
                &fixture.election,
                "first"
            )
            .await,
            None
        );
        assert_eq!(
            stored_ballot(&mut client, &fixture, VOTER, &Uuid::new_v4(), "first").await,
            None
        );
        assert_eq!(
            stored_ballot(&mut client, &fixture, VOTER, &fixture.election, "second").await,
            None
        );
        remove(&client, &fixture).await;
    }
}

/// The cast check and the tally read only the ballot styles of published
/// publications. A generated draft keeps its styles live until it is
/// published, so filtering on the style alone would also return the draft's.
#[tokio::test]
#[ignore = "requires the disposable devenv database fixture"]
async fn only_published_ballot_styles_are_read() {
    use sequent_core::ballot::Contest;
    use sequent_core::fixtures::ballot_codec::{get_test_contest, get_writein_ballot_style};

    let mut client = test_client().await;
    let transaction = client.transaction().await.unwrap();
    let fixture = ElectionFixture::create(&transaction).await;
    let other_area = Uuid::new_v4();
    // Each style has a single contest named after its case:
    // (contest, area, published, publication deleted, style deleted).
    let cases = [
        ("published", fixture.area, true, false, false),
        ("draft", fixture.area, false, false, false),
        ("deleted-publication", fixture.area, true, true, false),
        ("deleted-style", fixture.area, true, false, true),
        ("other-area", other_area, true, false, false),
    ];
    for (contest_id, area, published, publication_deleted, style_deleted) in cases {
        let publication = Uuid::new_v4();
        transaction
            .execute(
                "INSERT INTO sequent_backend.ballot_publication
                    (id, tenant_id, election_event_id, is_generated, published_at, deleted_at)
                VALUES ($1, $2, $3, true, CASE WHEN $4 THEN now() END, CASE WHEN $5 THEN now() END)",
                &[
                    &publication,
                    &fixture.tenant,
                    &fixture.event,
                    &published,
                    &publication_deleted,
                ],
            )
            .await
            .unwrap();
        let ballot_style = BallotStyle {
            contests: vec![Contest {
                id: contest_id.to_string(),
                ..get_test_contest()
            }],
            ..get_writein_ballot_style()
        };
        transaction
            .execute(
                "INSERT INTO sequent_backend.ballot_style
                    (id, tenant_id, election_event_id, election_id, area_id, ballot_eml,
                     deleted_at, ballot_publication_id)
                VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7 THEN now() END, $8)",
                &[
                    &Uuid::new_v4(),
                    &fixture.tenant,
                    &fixture.event,
                    &fixture.election,
                    &area,
                    &serde_json::to_string(&ballot_style).unwrap(),
                    &style_deleted,
                    &publication,
                ],
            )
            .await
            .unwrap();
    }

    let cast_styles = get_published_style_contest_ids(
        &transaction,
        &fixture.tenant.to_string(),
        &fixture.event.to_string(),
        &fixture.election.to_string(),
        &fixture.area.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(cast_styles, vec![HashSet::from(["published".to_string()])]);

    let mut tally_contests: Vec<String> = get_ballot_styles_by_elections(
        &transaction,
        &fixture.tenant.to_string(),
        &fixture.event.to_string(),
        &vec![fixture.election.to_string()],
        None,
    )
    .await
    .unwrap()
    .iter()
    .flat_map(|style| read_style_contest_ids(style).unwrap())
    .collect();
    tally_contests.sort();
    assert_eq!(tally_contests, ["other-area", "published"]);
    transaction.rollback().await.unwrap();
}
