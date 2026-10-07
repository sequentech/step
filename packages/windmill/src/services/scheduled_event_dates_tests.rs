// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::ballot::VotingStatusChannel::{KIOSK, ONLINE, TELEPHONE};
use serde_json::json;
use uuid::Uuid;

fn event(
    id: &str,
    election: Option<&str>,
    channels: Option<Vec<VotingStatusChannel>>,
) -> ScheduledEvent {
    serde_json::from_value(json!({
        "id": id,
        "tenant_id": "tenant",
        "election_event_id": "event",
        "event_processor": "END_VOTING_PERIOD",
        "event_payload": {"election_id": election, "voting_channels": channels},
        "task_id": format!("custom-{id}"),
    }))
    .unwrap()
}

#[test]
fn separate_channels_are_not_selected_for_overwrite() {
    let kiosk = event("kiosk", None, Some(vec![KIOSK]));
    assert!(select_schedule(
        &[&kiosk],
        None,
        &EventProcessors::END_VOTING_PERIOD,
        Some(&[ONLINE])
    )
    .unwrap()
    .is_none());
    assert_eq!(
        select_schedule(
            &[&kiosk],
            Some("kiosk"),
            &EventProcessors::END_VOTING_PERIOD,
            Some(&[TELEPHONE])
        )
        .unwrap()
        .unwrap()
        .id,
        "kiosk"
    );
}

#[test]
fn selected_ids_must_match_tenant_event_election_and_processor() {
    let original = event("selected", Some("election"), Some(vec![KIOSK]));
    for (tenant, event, election, processor) in [
        (
            "foreign",
            "event",
            Some("election"),
            EventProcessors::END_VOTING_PERIOD,
        ),
        (
            "tenant",
            "foreign",
            Some("election"),
            EventProcessors::END_VOTING_PERIOD,
        ),
        ("tenant", "event", None, EventProcessors::END_VOTING_PERIOD),
        (
            "tenant",
            "event",
            Some("other"),
            EventProcessors::END_VOTING_PERIOD,
        ),
        (
            "tenant",
            "event",
            Some("election"),
            EventProcessors::START_VOTING_PERIOD,
        ),
    ] {
        let events = [original.clone()];
        let scoped = scoped_events(&events, tenant, event, election, &processor).unwrap();
        assert!(select_schedule(&scoped, Some("selected"), &processor, None).is_err());
    }
    let mut archived = original;
    archived.archived_at = Some(chrono::Utc::now());
    assert!(scoped_events(
        &[archived],
        "tenant",
        "event",
        Some("election"),
        &EventProcessors::END_VOTING_PERIOD
    )
    .unwrap()
    .is_empty());
}

#[test]
fn ambiguous_legacy_requests_fail_instead_of_picking_a_row() {
    let kiosk = event("kiosk", None, Some(vec![KIOSK]));
    let online = event("online", None, Some(vec![ONLINE]));
    assert!(select_schedule(
        &[&kiosk, &online],
        None,
        &EventProcessors::END_VOTING_PERIOD,
        None
    )
    .is_err());
    assert_eq!(
        select_schedule(
            &[&kiosk, &online],
            Some("kiosk"),
            &EventProcessors::END_VOTING_PERIOD,
            None
        )
        .unwrap()
        .unwrap()
        .id,
        "kiosk"
    );
}

#[test]
fn channel_keys_preserve_online_deadlines_and_normalize_channel_order() {
    let processor = EventProcessors::END_VOTING_PERIOD;
    let base = generate_manage_date_task_name("tenant", "event", Some("election"), &processor);
    for channels in [
        None,
        Some(vec![]),
        Some(vec![ONLINE]),
        Some(vec![KIOSK, ONLINE]),
    ] {
        assert_eq!(
            generate_channel_date_task_name(
                "tenant",
                "event",
                Some("election"),
                &processor,
                channels.as_deref()
            ),
            base
        );
    }
    let kiosk = generate_channel_date_task_name(
        "tenant",
        "event",
        Some("election"),
        &processor,
        Some(&[KIOSK]),
    );
    assert_ne!(kiosk, base);
    assert_eq!(
        generate_channel_date_task_name(
            "tenant",
            "event",
            Some("election"),
            &processor,
            Some(&[TELEPHONE, KIOSK, KIOSK])
        ),
        generate_channel_date_task_name(
            "tenant",
            "event",
            Some("election"),
            &processor,
            Some(&[KIOSK, TELEPHONE])
        )
    );
}

async fn client() -> deadpool_postgres::Client {
    let mut config = deadpool_postgres::Config::new();
    config.url = Some(
        std::env::var("SCHEDULE_TEST_DATABASE_URL")
            .expect("requires the disposable schedule test database"),
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

async fn save(
    transaction: &Transaction<'_>,
    tenant: &str,
    event: &str,
    election: Option<&str>,
    date: Option<&str>,
    channels: Option<Vec<VotingStatusChannel>>,
    id: Option<&str>,
) -> Result<()> {
    manage_dates(
        transaction,
        tenant,
        event,
        election,
        date.map(|date| CronConfig {
            cron: None,
            local: None,
            timezone: None,
            scheduled_date: Some(date.into()),
        }),
        &EventProcessors::END_VOTING_PERIOD,
        channels,
        id,
    )
    .await
}

#[tokio::test]
#[ignore = "requires the disposable schedule test database"]
async fn independent_channel_schedules_edit_and_archive_only_the_selected_row() {
    let mut client = client().await;
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4().to_string();
    let event = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES ($1, $2)",
        &[
            &Uuid::parse_str(&event).unwrap(),
            &Uuid::parse_str(&tenant).unwrap(),
        ],
    )
    .await
    .unwrap();
    let election = Uuid::new_v4().to_string();
    for scope in [None, Some(election.as_str())] {
        save(
            &tx,
            &tenant,
            &event,
            scope,
            Some("2027-01-01T17:00:00Z"),
            Some(vec![KIOSK]),
            None,
        )
        .await
        .unwrap();
        save(
            &tx,
            &tenant,
            &event,
            scope,
            Some("2027-01-01T20:00:00Z"),
            Some(vec![ONLINE]),
            None,
        )
        .await
        .unwrap();
        let events = find_scheduled_event_by_election_event_id(&tx, &tenant, &event)
            .await
            .unwrap();
        let scoped = scoped_events(
            &events,
            &tenant,
            &event,
            scope,
            &EventProcessors::END_VOTING_PERIOD,
        )
        .unwrap();
        assert_eq!(scoped.len(), 2);
        let kiosk = scoped
            .iter()
            .find(|event| payload(event).unwrap().channels() == [KIOSK])
            .unwrap();
        let online = scoped
            .iter()
            .find(|event| payload(event).unwrap().channels() == [ONLINE])
            .unwrap();
        assert_eq!(
            online.task_id.as_deref(),
            Some(
                generate_manage_date_task_name(
                    &tenant,
                    &event,
                    scope,
                    &EventProcessors::END_VOTING_PERIOD
                )
                .as_str()
            )
        );
        rename_scheduled_event_task(&tx, &tenant, &kiosk.id, "manually-created-kiosk")
            .await
            .unwrap();
        save(
            &tx,
            &tenant,
            &event,
            scope,
            Some("2027-01-01T18:00:00Z"),
            Some(vec![TELEPHONE]),
            Some(&kiosk.id),
        )
        .await
        .unwrap();
        let changed =
            find_scheduled_event_by_id(&tx, Some(tenant.clone()), Some(event.clone()), &kiosk.id)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(payload(&changed).unwrap().channels(), [TELEPHONE]);
        assert_eq!(
            changed.cron_config.unwrap().scheduled_date.as_deref(),
            Some("2027-01-01T18:00:00Z")
        );
        let unchanged =
            find_scheduled_event_by_id(&tx, Some(tenant.clone()), Some(event.clone()), &online.id)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(
            unchanged.cron_config.unwrap().scheduled_date.as_deref(),
            Some("2027-01-01T20:00:00Z")
        );
        assert!(save(
            &tx,
            &tenant,
            &event,
            scope,
            Some("2027-01-01T19:00:00Z"),
            Some(vec![ONLINE]),
            Some(&kiosk.id)
        )
        .await
        .is_err());
        save(&tx, &tenant, &event, scope, None, None, Some(&kiosk.id))
            .await
            .unwrap();
        assert!(find_scheduled_event_by_id(
            &tx,
            Some(tenant.clone()),
            Some(event.clone()),
            &kiosk.id
        )
        .await
        .unwrap()
        .is_none());
        assert!(find_scheduled_event_by_id(
            &tx,
            Some(tenant.clone()),
            Some(event.clone()),
            &online.id
        )
        .await
        .unwrap()
        .is_some());
        assert!(save(
            &tx,
            &tenant,
            &event,
            scope,
            Some("2027-01-01T19:00:00Z"),
            Some(vec![KIOSK]),
            Some(&kiosk.id)
        )
        .await
        .is_err());
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the disposable schedule test database"]
async fn legacy_kiosk_task_keeps_its_row_and_date_when_online_is_added() {
    let mut client = client().await;
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4().to_string();
    let event = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES ($1, $2)",
        &[
            &Uuid::parse_str(&event).unwrap(),
            &Uuid::parse_str(&tenant).unwrap(),
        ],
    )
    .await
    .unwrap();
    let base =
        generate_manage_date_task_name(&tenant, &event, None, &EventProcessors::END_VOTING_PERIOD);
    let old = insert_scheduled_event(
        &tx,
        &tenant,
        &event,
        EventProcessors::END_VOTING_PERIOD,
        &base,
        CronConfig {
            cron: None,
            local: None,
            timezone: None,
            scheduled_date: Some("2027-01-01T17:00:00Z".into()),
        },
        json!({"voting_channels": ["KIOSK"]}),
    )
    .await
    .unwrap();
    save(
        &tx,
        &tenant,
        &event,
        None,
        Some("2027-01-01T20:00:00Z"),
        Some(vec![ONLINE]),
        None,
    )
    .await
    .unwrap();
    let old_after =
        find_scheduled_event_by_id(&tx, Some(tenant.clone()), Some(event.clone()), &old.id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(old_after.event_payload, old.event_payload);
    assert_eq!(old_after.cron_config, old.cron_config);
    assert_ne!(old_after.task_id, old.task_id);
    assert!(save(
        &tx,
        &tenant,
        &event,
        None,
        None,
        None,
        Some(&Uuid::new_v4().to_string())
    )
    .await
    .is_err());
    assert_eq!(
        find_scheduled_event_by_election_event_id(&tx, &tenant, &event)
            .await
            .unwrap()
            .len(),
        2
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the disposable schedule test database"]
async fn editing_a_stopped_channel_schedule_rearms_it_and_preserves_wall_time() {
    let mut client = client().await;
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES ($1, $2)",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    let tenant = tenant.to_string();
    let event = event.to_string();
    save(
        &tx,
        &tenant,
        &event,
        None,
        Some("2027-01-01T17:00:00Z"),
        Some(vec![KIOSK]),
        None,
    )
    .await
    .unwrap();
    let rows = find_scheduled_event_by_election_event_id(&tx, &tenant, &event)
        .await
        .unwrap();
    let id = &rows[0].id;
    tx.execute("UPDATE sequent_backend.scheduled_event SET stopped_at = now(), annotations = '{\"schedule_recompute\":{},\"retained\":true}'::jsonb WHERE id = $1", &[&Uuid::parse_str(id).unwrap()]).await.unwrap();
    let cron = CronConfig {
        cron: None,
        scheduled_date: Some("2099-01-01T17:00:00Z".into()),
        local: Some("2099-01-01T12:00".into()),
        timezone: Some("America/Bogota".into()),
    };
    manage_dates(
        &tx,
        &tenant,
        &event,
        None,
        Some(cron.clone()),
        &EventProcessors::END_VOTING_PERIOD,
        Some(vec![KIOSK]),
        Some(id),
    )
    .await
    .unwrap();
    let rows = find_scheduled_event_by_election_event_id(&tx, &tenant, &event)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, *id);
    assert!(rows[0].stopped_at.is_none());
    assert_eq!(
        serde_json::to_value(&rows[0].cron_config).unwrap(),
        serde_json::to_value(cron).unwrap()
    );
    assert_eq!(rows[0].annotations, Some(json!({"retained":true})));
}
