// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Privacy restrictions composed with the real lifecycle and voter filters.
#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use electoral_log::adapters::privacy::BallotIdentity;
use event_database::{EventDatabase, Vote};
use serde_json::json;
use uuid::Uuid;
use windmill::services::users::{
    count_keycloak_users, list_users, list_users_has_voted, FilterOption, ListUsersFilter,
};

async fn event(tx: &Transaction<'_>, privacy: bool) -> (Uuid, Uuid) {
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1,$2)",
        &[&tenant, &tenant.to_string()],
    )
    .await
    .unwrap();
    let presentation = if privacy {
        json!({"ballot_identity":"pseudonym","voter_signing_policy":"with-signature","receipts":{"policy":"signed-by-ballot-box"}})
    } else {
        json!({"ballot_identity":"voter-id"})
    };
    tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation,voting_channels) VALUES($1,$2,'RSA256',$3,$4)", &[&event,&tenant,&presentation,&json!({"online":true,"telephone":false})]).await.unwrap();
    (tenant, event)
}

#[tokio::test]
async fn pseudonym_lifecycle_rejects_telephone_at_event_and_child_scope() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    for privacy in [false, true] {
        let (tenant, event) = event(&tx, privacy).await;
        let election = Uuid::new_v4();
        tx.batch_execute("SAVEPOINT channel_insert").await.unwrap();
        let inserted=tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,voting_channels) VALUES($1,$2,$3,$4)", &[&election,&tenant,&event,&json!({"telephone":true})]).await;
        if privacy {
            assert!(
                inserted.is_err(),
                "pseudonym child telephone voting must be refused before publication"
            );
            assert!(inserted
                .unwrap_err()
                .as_db_error()
                .unwrap()
                .message()
                .contains("telephone"));
        } else {
            assert_eq!(inserted.unwrap(), 1);
        }
        tx.batch_execute("ROLLBACK TO SAVEPOINT channel_insert")
            .await
            .unwrap();
        tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,voting_channels) VALUES($1,$2,$3,$4)", &[&election,&tenant,&event,&json!({"telephone":false})]).await.unwrap();
        for (sql, id) in [
            (
                "UPDATE sequent_backend.election SET voting_channels=$2 WHERE id=$1",
                election,
            ),
            (
                "UPDATE sequent_backend.election_event SET voting_channels=$2 WHERE id=$1",
                event,
            ),
        ] {
            tx.batch_execute("SAVEPOINT channel_update").await.unwrap();
            let update = tx.execute(sql, &[&id, &json!({"telephone":true})]).await;
            assert_eq!(
                update.is_err(),
                privacy,
                "channel compatibility is enforced for every edit"
            );
            tx.batch_execute("ROLLBACK TO SAVEPOINT channel_update")
                .await
                .unwrap();
        }
        if privacy {
            // Simulate a child configuration saved before the invariant was
            // introduced. Publication must refuse it before opening a board.
            tx.batch_execute("ALTER TABLE sequent_backend.election DISABLE TRIGGER check_election_privacy_channels").await.unwrap();
            tx.execute(
                "UPDATE sequent_backend.election SET voting_channels=$2 WHERE id=$1",
                &[&election, &json!({"telephone":true})],
            )
            .await
            .unwrap();
            tx.batch_execute("ALTER TABLE sequent_backend.election ENABLE TRIGGER check_election_privacy_channels").await.unwrap();
            tx.batch_execute("SAVEPOINT preexisting_channel_open")
                .await
                .unwrap();
            let opened = tx
                .execute(
                    "UPDATE sequent_backend.election SET status=$2 WHERE id=$1",
                    &[&election, &json!({"voting_status":"OPEN"})],
                )
                .await;
            assert!(opened
                .unwrap_err()
                .as_db_error()
                .unwrap()
                .message()
                .contains("telephone"));
            tx.batch_execute("ROLLBACK TO SAVEPOINT preexisting_channel_open")
                .await
                .unwrap();
            let opened = tx
                .execute(
                    "UPDATE sequent_backend.election_event SET status=$2 WHERE id=$1",
                    &[&event, &json!({"voting_status":"OPEN"})],
                )
                .await;
            assert!(opened
                .unwrap_err()
                .as_db_error()
                .unwrap()
                .message()
                .contains("telephone"));
            tx.batch_execute("ROLLBACK TO SAVEPOINT preexisting_channel_open")
                .await
                .unwrap();
            let error = windmill::services::vote_privacy::allocate_registry(
                &tx,
                &tenant.to_string(),
                &event.to_string(),
            )
            .await
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("telephone voting on every child election"),
                "{error}"
            );
        }
    }
}

#[test]
fn participation_filters_count_only_the_matching_population() {
    event_database::run(async {
        for privacy in [true, false] {
            let pool = schema::pool().await;
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let (tenant, event) = event(&tx, privacy).await;
            let database = EventDatabase::create(tenant, event).await;
            if privacy {
                database
                    .store
                    .set_ballot_identity(&event.to_string(), BallotIdentity::Pseudonym)
                    .await
                    .unwrap();
            }
            tx.execute(
                "UPDATE sequent_backend.election_event SET bulletin_board_reference=$2 WHERE id=$1",
                &[
                    &event,
                    &json!({"id":1,"database_name":database.board,"is_archived":false}),
                ],
            )
            .await
            .unwrap();
            let election = Uuid::new_v4();
            tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id) VALUES($1,$2,$3)",&[&election,&tenant,&event]).await.unwrap();
            let hidden_election = Uuid::new_v4();
            tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id) VALUES($1,$2,$3)",&[&hidden_election,&tenant,&event]).await.unwrap();
            let voted_area = Uuid::new_v4();
            let unvoted_area = Uuid::new_v4();
            for area in [voted_area, unvoted_area] {
                tx.execute("INSERT INTO sequent_backend.area(id,tenant_id,election_event_id,name) VALUES($1,$2,$3,'Fixture area')",&[&area,&tenant,&event]).await.unwrap();
            }
            tx.batch_execute("CREATE SCHEMA voter_filter_fixture; SET LOCAL search_path TO voter_filter_fixture,public;
          CREATE TABLE realm(id text PRIMARY KEY,name text);
          CREATE TABLE user_entity(id text PRIMARY KEY,realm_id text,username text,email text,email_verified boolean DEFAULT false,enabled boolean,first_name text,last_name text,created_timestamp bigint,service_account_client_link text);
          CREATE TABLE user_attribute(id text,name text,value text,user_id text);
          CREATE INDEX ON user_attribute(user_id);
          INSERT INTO realm VALUES('realm','privacy-filter');").await.unwrap();
            let ids: Vec<String> = (0..4).map(|_| Uuid::new_v4().to_string()).collect();
            for (i, id) in ids.iter().enumerate() {
                tx.execute("INSERT INTO user_entity(id,realm_id,username,enabled) VALUES($1,'realm',$2,$3)", &[id,&format!("voter-{i}"),&(i!=3)]).await.unwrap();
                tx.execute(
                    "INSERT INTO user_attribute(id,name,value,user_id) VALUES($1,'selected',$2,$1)",
                    &[id, &if i < 2 { "yes" } else { "no" }],
                )
                .await
                .unwrap();
            }
            for (index, id) in ids.iter().enumerate() {
                tx.execute(
                    "INSERT INTO user_attribute(id,name,value,user_id) VALUES($1,'area-id',$2,$1)",
                    &[
                        id,
                        &if index < 2 {
                            voted_area.to_string()
                        } else {
                            unvoted_area.to_string()
                        },
                    ],
                )
                .await
                .unwrap();
            }
            if privacy {
                database
                    .store
                    .allocate_voter_pseudonyms(&event.to_string(), &ids)
                    .await
                    .unwrap();
            }
            let store = database.store.client().await.unwrap();
            for (index, id) in ids[..2].iter().enumerate() {
                let vote_election = if index == 0 {
                    election
                } else {
                    hidden_election
                };
                if privacy {
                    store.execute("INSERT INTO sequent_backend.voter_participation(election_event_id,election_id,pseudonym) SELECT election_event_id,$2::text::uuid,pseudonym FROM sequent_backend.voter_pseudonym WHERE voter_id=$1",&[id,&vote_election.to_string()]).await.unwrap();
                } else {
                    database
                        .cast(&Vote {
                            election: vote_election,
                            area: voted_area,
                            voter: id,
                            content: "fixture ballot",
                            ballot_id: &format!("ballot-{index}"),
                            channel: "online",
                        })
                        .await;
                }
            }
            let base = ListUsersFilter {
                election_event_id: Some(event.to_string()),
                limit: Some(1),
                has_voted: Some(false),
                ..ListUsersFilter::new(&tenant.to_string(), "privacy-filter")
            };
            let mut selected = base.clone();
            selected.attributes = Some(std::collections::HashMap::from([(
                "selected".into(),
                "yes".into(),
            )]));
            let (users, count) = list_users_has_voted(&tx, &tx, selected, &tenant.to_string())
                .await
                .unwrap();
            assert!(users.is_empty());
            assert_eq!(
                count, 0,
                "the unvoted total cannot use a global participation count"
            );
            let mut disabled = base.clone();
            disabled.enabled = Some(false);
            let (users, count) = list_users_has_voted(&tx, &tx, disabled, &tenant.to_string())
                .await
                .unwrap();
            assert_eq!(count, 1);
            assert_eq!(users[0].id.as_deref(), Some(ids[3].as_str()));
            let mut username = base.clone();
            username.username = Some(FilterOption::IsEqual("voter-0".into()));
            let (users, count) = list_users_has_voted(&tx, &tx, username, &tenant.to_string())
                .await
                .unwrap();
            assert!(users.is_empty());
            assert_eq!(count, 0);
            let (first, count) = list_users_has_voted(&tx, &tx, base.clone(), &tenant.to_string())
                .await
                .unwrap();
            assert_eq!(count, 2);
            let mut second_filter = base.clone();
            second_filter.offset = Some(1);
            let (second, count) =
                list_users_has_voted(&tx, &tx, second_filter, &tenant.to_string())
                    .await
                    .unwrap();
            assert_eq!(count, 2);
            assert_ne!(first[0].id, second[0].id);
            let mut search = base.clone();
            search.search = Some("voter-0".into());
            let (users, count) =
                list_users_has_voted(&tx, &tx, search.clone(), &tenant.to_string())
                    .await
                    .unwrap();
            assert!(users.is_empty());
            assert_eq!(count, 0);
            search.has_voted = None;
            let (users, count) = list_users(&tx, &tx, search.clone()).await.unwrap();
            assert_eq!(users[0].id.as_deref(), Some(ids[0].as_str()));
            assert_eq!(count, 1);
            assert_eq!(count_keycloak_users(&tx, &tx, search).await.unwrap(), 1);
            let mut area = base.clone();
            area.area_id = Some(voted_area.to_string());
            let (users, count) = list_users_has_voted(&tx, &tx, area, &tenant.to_string())
                .await
                .unwrap();
            assert!(users.is_empty());
            assert_eq!(count, 0);
            // Multiple matching area values must not duplicate a participation
            // page or consume its limit; the user remains one eligible voter.
            tx.execute(
                "INSERT INTO user_attribute(id,name,value,user_id) VALUES($1,'area-id',$2,$1)",
                &[&ids[0], &unvoted_area.to_string()],
            )
            .await
            .unwrap();
            for wanted in [true, false] {
                let mut restricted = base.clone();
                restricted.limit = Some(10);
                restricted.has_voted = Some(wanted);
                restricted.sort = Some(std::collections::HashMap::from([
                    ("'field'".into(), "selected".into()),
                    ("'order'".into(), "ASC".into()),
                ]));
                restricted.permitted_election_ids = Some(vec![election.to_string()]);
                let (users, count) =
                    list_users_has_voted(&tx, &tx, restricted.clone(), &tenant.to_string())
                        .await
                        .unwrap();
                assert_eq!(count, if wanted { 1 } else { 3 });
                assert_eq!(users.len(), count as usize);
                assert_eq!(
                    users
                        .iter()
                        .any(|user| user.id.as_deref() == Some(ids[0].as_str())),
                    wanted
                );
                assert!(users
                    .iter()
                    .flat_map(|user| user.votes_info.as_deref().unwrap_or_default())
                    .all(|vote| vote.election_id == election.to_string()
                        && vote.last_voted_at.is_empty()));
                restricted.limit = Some(0);
                assert_eq!(
                    list_users_has_voted(&tx, &tx, restricted.clone(), &tenant.to_string())
                        .await
                        .unwrap()
                        .1,
                    count
                );
                restricted.permitted_election_ids = Some(vec![]);
                assert_eq!(
                    list_users_has_voted(&tx, &tx, restricted, &tenant.to_string())
                        .await
                        .unwrap()
                        .1,
                    if wanted { 0 } else { 4 }
                );
            }
            if !privacy {
                let annotations = json!({"datafix:id":"fixture","datafix:password_policy":r#"{"base":"password-only","size":6,"characters":"numeric"}"#,"datafix:voterview_request":r#"{"url":"https://example.invalid","usr":"fixture","psw":"fixture","county_mun":"fixture"}"#});
                tx.execute(
                    "UPDATE sequent_backend.election_event SET annotations=$2 WHERE id=$1",
                    &[&event, &annotations],
                )
                .await
                .unwrap();
                tx.execute("INSERT INTO user_attribute(id,name,value,user_id) VALUES($1,'voted-channel','PAPER',$1)",&[&ids[2]]).await.unwrap();
                let mut datafix = base.clone();
                datafix.has_voted = Some(true);
                datafix.limit = Some(10);
                assert_eq!(
                    list_users_has_voted(&tx, &tx, datafix.clone(), &tenant.to_string())
                        .await
                        .unwrap()
                        .1,
                    3
                );
                datafix.permitted_election_ids = Some(vec![election.to_string()]);
                assert_eq!(
                    list_users_has_voted(&tx, &tx, datafix.clone(), &tenant.to_string())
                        .await
                        .unwrap()
                        .1,
                    2
                );
                datafix.permitted_election_ids = Some(vec![]);
                assert_eq!(
                    list_users_has_voted(&tx, &tx, datafix, &tenant.to_string())
                        .await
                        .unwrap()
                        .1,
                    0
                );
            }
            drop(store);
            database.drop().await;
        }
    });
}
