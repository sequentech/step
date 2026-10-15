// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Authoritative readiness census and ballot counts, exercising PostgreSQL
//! grouping and the event ballot store instead of portal/statistics totals.

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use electoral_log::adapters::ballot_box::{AcceptOutcome, BallotStatus};
use event_database::{run, EventDatabase, Vote};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use uuid::Uuid;
use windmill::services::readiness::{ordinary_ballot_count, registry_counts};

#[tokio::test]
async fn current_census_counts_unique_users_across_the_whole_realm_and_retains_invalid_assignments()
{
    let pool = schema::pool().await;
    let client = pool.get().await.unwrap();
    // Temporary Keycloak-shaped tables belong only to this fixture connection.
    client.batch_execute("CREATE TEMP TABLE realm (id text, name text);
        CREATE TEMP TABLE user_entity (id text, realm_id text, service_account_client_link text);
        CREATE TEMP TABLE user_attribute (user_id text, name text, value text);
        INSERT INTO realm VALUES ('realm', 'official'), ('other', 'practice');
        INSERT INTO user_entity VALUES ('ada', 'realm', NULL), ('bob', 'realm', NULL),
            ('charles', 'realm', NULL), ('service', 'realm', 'client'), ('outsider', 'other', NULL);").await.unwrap();
    for (user, area) in [
        ("ada", "area-a-id"),
        ("bob", "area-b-id"),
        ("charles", "area-a-id"),
        ("charles", "area-a-id"),
        ("service", "area-a-id"),
        ("outsider", "area-a-id"),
    ] {
        client
            .execute(
                "INSERT INTO user_attribute VALUES ($1, $2, $3)",
                &[&user, &AREA_ID_ATTR_NAME, &area],
            )
            .await
            .unwrap();
    }
    let names = [
        ("area-a-id".into(), " Area A ".into()),
        ("area-b-id".into(), "Area B".into()),
    ]
    .into();
    assert_eq!(
        registry_counts(&client, "official", &names).await.unwrap(),
        [("Area A".into(), 2), ("Area B".into(), 1)].into()
    );
    // A later additive import cannot masquerade as the whole certified file.
    client
        .batch_execute("INSERT INTO user_entity VALUES ('delta', 'realm', NULL)")
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO user_attribute VALUES ('delta', $1, 'area-b-id')",
            &[&AREA_ID_ATTR_NAME],
        )
        .await
        .unwrap();
    assert_eq!(
        registry_counts(&client, "official", &names).await.unwrap(),
        [("Area A".into(), 2), ("Area B".into(), 2)].into()
    );
    client.batch_execute("INSERT INTO user_entity VALUES ('unassigned', 'realm', NULL), ('unknown', 'realm', NULL), ('ambiguous', 'realm', NULL)").await.unwrap();
    for (user, area) in [
        ("unknown", "deleted-area"),
        ("ambiguous", "area-a-id"),
        ("ambiguous", "area-b-id"),
    ] {
        client
            .execute(
                "INSERT INTO user_attribute VALUES ($1, $2, $3)",
                &[&user, &AREA_ID_ATTR_NAME, &area],
            )
            .await
            .unwrap();
    }
    assert_eq!(
        registry_counts(&client, "official", &names).await.unwrap(),
        [("".into(), 3), ("Area A".into(), 2), ("Area B".into(), 2)].into()
    );
}

#[test]
fn ordinary_ballot_count_includes_unsequenced_pending_and_rejected_records() {
    run(async {
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        let db = EventDatabase::create(tenant, event).await;
        let election = Uuid::new_v4();
        let area = Uuid::new_v4();
        assert_eq!(
            ordinary_ballot_count(&db.store, &event.to_string())
                .await
                .unwrap(),
            0
        );
        for (voter, status) in [
            ("valid-voter", BallotStatus::Valid),
            ("pending-voter", BallotStatus::Pending),
        ] {
            let vote = Vote {
                election,
                area,
                voter,
                content: voter,
                ballot_id: voter,
                channel: "ONLINE",
            };
            assert!(matches!(
                db.try_cast(&vote, status).await,
                AcceptOutcome::Accepted { .. }
            ));
        }
        db.cast_rejected(&Vote {
            election,
            area,
            voter: "rejected-voter",
            content: "rejected",
            ballot_id: "rejected",
            channel: "ONLINE",
        })
        .await;
        // None has been sequenced into electoral-log records.
        assert_eq!(
            ordinary_ballot_count(&db.store, &event.to_string())
                .await
                .unwrap(),
            3
        );
        assert_eq!(
            ordinary_ballot_count(&db.store, &Uuid::new_v4().to_string())
                .await
                .unwrap(),
            0
        );
        db.drop().await;
    });
}
