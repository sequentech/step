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
