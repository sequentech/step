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

#[tokio::test]
async fn privacy_queued_participation_grants_are_bound_immutable_and_fail_closed() {
    use sequent_core::types::messaging::MessageReason;
    use windmill::postgres::messaging::{record_bulk_send, record_bulk_send_with_participation};
    use windmill::services::messaging::dispatch::retry_refusal;
    use windmill::services::participation_authorization::*;
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = event(&tx, false).await;
    let allowed = Uuid::new_v4().to_string();
    let forged = json!({"tenant_id":tenant,"election_event_id":event,"actor_id":"actor",
        "command":"bulk-send","command_id":Uuid::new_v4(),"election_ids":[allowed]});
    let payload = json!({"audience_selection":"VOTED","participation_authorization":forged});
    // A public scheduled-event payload is writable input, including grant-like keys.
    let legacy = record_bulk_send(&tx, &tenant, Some(&event), Some("actor"), &payload)
        .await
        .unwrap();
    assert!(authorized_bulk_send(&tx, &tenant, &legacy)
        .await
        .err()
        .unwrap()
        .downcast_ref::<ParticipationReauthorization>()
        .is_some());
    assert_eq!(
        retry_refusal(
            &tx,
            &tenant,
            Some(&format!("send-template:{legacy}:voter")),
            None
        )
        .await
        .unwrap(),
        Some(MessageReason::PARTICIPATION_REAUTHORIZATION_REQUIRED)
    );
    let sent = record_bulk_send_with_participation(
        &tx,
        &tenant,
        Some(&event),
        Some("actor"),
        &payload,
        Some(vec![allowed.clone()]),
    )
    .await
    .unwrap();
    let command = authorized_bulk_send(&tx, &tenant, &sent)
        .await
        .unwrap()
        .unwrap();
    let grant = command.participation.unwrap();
    assert_eq!(grant.election_ids, vec![allowed.clone()]);
    grant
        .validate(
            &tenant.to_string(),
            &event.to_string(),
            "actor",
            ParticipationCommand::BulkSend,
            &sent.to_string(),
        )
        .unwrap();
    for (t, e, a, id) in [
        ("other", event.to_string(), "actor", sent.to_string()),
        (
            &tenant.to_string(),
            "other".into(),
            "actor",
            sent.to_string(),
        ),
        (
            &tenant.to_string(),
            event.to_string(),
            "other",
            sent.to_string(),
        ),
        (
            &tenant.to_string(),
            event.to_string(),
            "actor",
            "other".into(),
        ),
    ] {
        assert!(grant
            .validate(t, &e, a, ParticipationCommand::BulkSend, &id)
            .is_err());
    }
    tx.execute(
        "INSERT INTO sequent_backend.election(id,tenant_id,election_event_id) VALUES($1,$2,$3)",
        &[&Uuid::new_v4(), &tenant, &event],
    )
    .await
    .unwrap();
    assert_eq!(
        authorized_bulk_send(&tx, &tenant, &sent)
            .await
            .unwrap()
            .unwrap()
            .participation
            .unwrap()
            .election_ids,
        vec![allowed],
        "a later election never widens the stored command"
    );
    for sql in [
        "UPDATE sequent_backend.scheduled_event SET event_payload='{}'::jsonb WHERE id=$1",
        "UPDATE sequent_backend.scheduled_event SET participation_authorization=NULL WHERE id=$1",
        "UPDATE sequent_backend.scheduled_event SET created_by='other' WHERE id=$1",
    ] {
        tx.batch_execute("SAVEPOINT immutable_send").await.unwrap();
        let error = tx.execute(sql, &[&sent]).await.unwrap_err();
        assert!(error.as_db_error().unwrap().message().contains("immutable"));
        tx.batch_execute("ROLLBACK TO SAVEPOINT immutable_send")
            .await
            .unwrap();
    }
    let ordinary = record_bulk_send(
        &tx,
        &tenant,
        Some(&event),
        Some("actor"),
        &json!({"audience_selection":"ALL_USERS"}),
    )
    .await
    .unwrap();
    assert!(authorized_bulk_send(&tx, &tenant, &ordinary)
        .await
        .unwrap()
        .is_some());
    assert_eq!(
        retry_refusal(
            &tx,
            &tenant,
            Some(&format!("send-template:{ordinary}:voter")),
            None
        )
        .await
        .unwrap(),
        None
    );

    let doc = Uuid::new_v4();
    let task = Uuid::new_v4();
    let mut export_grant = grant.clone();
    export_grant.command = ParticipationCommand::VoterExport;
    export_grant.command_id = doc.to_string();
    tx.execute("INSERT INTO sequent_backend.tasks_execution(id,tenant_id,election_event_id,name,type,execution_status,executed_by_user,annotations,participation_authorization) VALUES($1,$2,$3,'Export','EXPORT_VOTERS','IN_PROGRESS','actor',$4,$5)",
        &[&task,&tenant,&event,&json!({"participation_authorization":{"election_ids":["forged"]}}),&serde_json::to_value(&export_grant).unwrap()]).await.unwrap();
    assert_eq!(
        export_authorization(&tx, &tenant.to_string(), &task.to_string())
            .await
            .unwrap(),
        export_grant
    );
    let forged_task = Uuid::new_v4();
    tx.execute("INSERT INTO sequent_backend.tasks_execution(id,tenant_id,election_event_id,name,type,execution_status,executed_by_user,annotations) VALUES($1,$2,$3,'Export','EXPORT_VOTERS','IN_PROGRESS','actor',$4)",
        &[&forged_task,&tenant,&event,&json!({"participation_authorization":export_grant})]).await.unwrap();
    assert!(
        export_authorization(&tx, &tenant.to_string(), &forged_task.to_string())
            .await
            .err()
            .unwrap()
            .downcast_ref::<ParticipationReauthorization>()
            .is_some()
    );
    tx.batch_execute("SAVEPOINT immutable_task").await.unwrap();
    assert!(tx.execute("UPDATE sequent_backend.tasks_execution SET participation_authorization=NULL WHERE id=$1",&[&task]).await.unwrap_err().as_db_error().unwrap().message().contains("immutable"));
    tx.batch_execute("ROLLBACK TO SAVEPOINT immutable_task")
        .await
        .unwrap();
    // Metadata edits that preserve scope remain possible; stripping scope,
    // renaming the object or making it public must not bypass download checks.
    for (name, annotations) in [
        (
            "scoped.csv",
            json!({"participation_scope":{"election_event_id":event,"election_ids":[]}}),
        ),
        ("users-export-old.csv", json!({})),
        ("election-event-old-export-sha256-hash.zip", json!({})),
    ] {
        let id = Uuid::new_v4();
        tx.execute("INSERT INTO sequent_backend.document(id,tenant_id,election_event_id,name,is_public,annotations) VALUES($1,$2,$3,$4,false,$5)",
            &[&id,&tenant,&event,&name,&annotations]).await.unwrap();
        tx.execute(
            "UPDATE sequent_backend.document SET labels='{}'::jsonb WHERE id=$1",
            &[&id],
        )
        .await
        .unwrap();
        for sql in [
            "UPDATE sequent_backend.document SET name='ordinary.csv' WHERE id=$1",
            "UPDATE sequent_backend.document SET is_public=true WHERE id=$1",
            "UPDATE sequent_backend.document SET election_event_id=NULL WHERE id=$1",
        ] {
            tx.batch_execute("SAVEPOINT immutable_document")
                .await
                .unwrap();
            assert!(tx
                .execute(sql, &[&id])
                .await
                .unwrap_err()
                .as_db_error()
                .unwrap()
                .message()
                .contains("immutable"));
            tx.batch_execute("ROLLBACK TO SAVEPOINT immutable_document")
                .await
                .unwrap();
        }
        if name == "scoped.csv" {
            tx.batch_execute("SAVEPOINT immutable_document")
                .await
                .unwrap();
            assert!(tx
                .execute(
                    "UPDATE sequent_backend.document SET annotations='{}'::jsonb WHERE id=$1",
                    &[&id]
                )
                .await
                .is_err());
            tx.batch_execute("ROLLBACK TO SAVEPOINT immutable_document")
                .await
                .unwrap();
        }
    }
}

#[test]
fn privacy_participation_and_ordinary_pages_keep_total_order_for_equal_and_null_values() {
    event_database::run(async {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let (tenant, event) = event(&tx, true).await;
        let database = EventDatabase::create(tenant, event).await;
        database
            .store
            .set_ballot_identity(&event.to_string(), BallotIdentity::Pseudonym)
            .await
            .unwrap();
        tx.execute(
            "UPDATE sequent_backend.election_event SET bulletin_board_reference=$2 WHERE id=$1",
            &[
                &event,
                &json!({"id":1,"database_name":database.board,"is_archived":false}),
            ],
        )
        .await
        .unwrap();
        tx.batch_execute("CREATE SCHEMA privacy_paging; SET LOCAL search_path TO privacy_paging,public;
            CREATE TABLE realm(id text PRIMARY KEY,name text);
            CREATE TABLE user_entity(id text PRIMARY KEY,realm_id text,username text,email text,email_verified boolean DEFAULT false,enabled boolean,first_name text,last_name text,created_timestamp bigint,service_account_client_link text);
            CREATE TABLE user_attribute(id text,name text,value text,user_id text);
            CREATE INDEX ON user_attribute(user_id); INSERT INTO realm VALUES('realm','privacy-pages');").await.unwrap();
        let ids: Vec<String> = (0..37)
            .map(|n| format!("00000000-0000-4000-8000-{n:012}"))
            .collect();
        for (n, id) in ids.iter().enumerate().rev() {
            let surname = if n < 30 { Some("same") } else { None };
            tx.execute("INSERT INTO user_entity(id,realm_id,username,enabled,last_name) VALUES($1,'realm',$1,true,$2)", &[id,&surname]).await.unwrap();
            if n < 30 {
                tx.execute("INSERT INTO user_attribute(id,user_id,name,value) VALUES($1,$1,'region','same')",&[id]).await.unwrap();
            }
        }
        for participation in [false, true] {
            for field in ["last_name", "enabled", "region", "id", "actions"] {
                for order in ["ASC", "DESC"] {
                    let mut expected = ids.clone();
                    if order == "DESC" {
                        if field == "id" {
                            expected.reverse();
                        } else if field == "last_name" || field == "region" {
                            expected.rotate_left(30);
                        }
                    }
                    let mut actual = Vec::new();
                    for offset in [0, 10, 20, 30, 40] {
                        let filter = ListUsersFilter {
                            election_event_id: Some(event.to_string()),
                            limit: Some(10),
                            offset: Some(offset),
                            has_voted: participation.then_some(false),
                            permitted_election_ids: Some(vec![]),
                            sort: Some(std::collections::HashMap::from([
                                ("'field'".into(), field.into()),
                                ("'order'".into(), order.into()),
                            ])),
                            ..ListUsersFilter::new(&tenant.to_string(), "privacy-pages")
                        };
                        let (users, count) = if participation {
                            list_users_has_voted(&tx, &tx, filter, &tenant.to_string())
                                .await
                                .unwrap()
                        } else {
                            list_users(&tx, &tx, filter).await.unwrap()
                        };
                        assert_eq!(count, 37);
                        actual.extend(users.into_iter().map(|user| user.id.unwrap()));
                    }
                    assert_eq!(
                        actual, expected,
                        "participation={participation}, {field} {order}"
                    );
                }
            }
        }
        database.drop().await;
    });
}

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
            // The same scoped vote projection used by the CSV and queued send
            // must not classify a voter through participation in hidden B.
            for (audience, expected) in [
                (
                    sequent_core::types::templates::AudienceSelection::VOTED,
                    vec![ids[0].clone()],
                ),
                (
                    sequent_core::types::templates::AudienceSelection::NOT_VOTED,
                    ids[1..].to_vec(),
                ),
            ] {
                let mut filter = base.clone();
                filter.has_voted = None;
                filter.limit = Some(10);
                filter.permitted_election_ids = Some(vec![election.to_string()]);
                let (mut users, count) =
                    windmill::services::users::list_users_with_vote_info(&tx, &tx, filter)
                        .await
                        .unwrap();
                assert_eq!(count, 4);
                windmill::tasks::send_template::retain_participation_audience(
                    &mut users, &audience,
                );
                let mut actual: Vec<_> = users.into_iter().map(|user| user.id.unwrap()).collect();
                actual.sort();
                let mut expected = expected;
                expected.sort();
                assert_eq!(actual, expected);
            }
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
