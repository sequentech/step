// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(all(feature = "sqlite", feature = "default_features"))]

use rusqlite::{types::Value as SqlValue, Connection};
use sequent_core::sqlite::*;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn entity<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("valid synthetic database entity")
}

fn strings(connection: &Connection, sql: &str) -> Vec<SqlValue> {
    connection
        .query_row(sql, [], |row| {
            (0..row.as_ref().column_count())
                .map(|i| row.get(i))
                .collect()
        })
        .unwrap()
}

// Literal SELECT lists and expected cells make field swaps visible. These
// assertions inspect SQLite itself, independently of the production mapper.
macro_rules! mapping {
    ($name:ident, $create:path, $table:literal, $fixture:expr, $query:literal, $expected:expr) => {
        #[tokio::test]
        async fn $name() {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            $create(&tx, vec![entity($fixture)]).await.unwrap();
            assert_eq!(strings(&tx, $query), $expected);
            tx.commit().unwrap();
            assert_eq!(
                db.query_row(
                    concat!("SELECT COUNT(*) FROM ", $table),
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
        }
    };
}

mapping!(area_fields_preserve_unicode_json_and_parent,
    area::create_area_sqlite, "area",
    json!({"id":"area", "tenant_id":"tenant", "election_event_id":"event",
        "name":"Árbol", "description":"district", "type":"precinct", "parent_id":"parent",
        "labels":{"label":1}, "annotations":{"note":2},
        "created_at":"2026-01-02T03:04:05Z", "last_updated_at":"2026-02-03T04:05:06Z"}),
    "SELECT id,tenant_id,election_event_id,name,description,type,parent_id,labels,annotations FROM area",
    vec![SqlValue::Text("area".into()),SqlValue::Text("tenant".into()),SqlValue::Text("event".into()),SqlValue::Text("Árbol".into()),SqlValue::Text("district".into()),SqlValue::Text("precinct".into()),SqlValue::Text("parent".into()),SqlValue::Text("{\"label\":1}".into()),SqlValue::Text("{\"note\":2}".into())]);

mapping!(contest_counts_flags_and_metadata_keep_their_columns,
    contests::create_contest_sqlite, "contest",
    json!({"id":"contest","tenant_id":"tenant","election_event_id":"event","election_id":"election",
        "min_votes":2,"max_votes":7,"winning_candidates_num":3,"is_acclaimed":false,"is_active":true,
        "is_encrypted":true,"external_id":"external","presentation":{"p":1},"tally_configuration":{"t":2},"conditions":{"c":3}}),
    "SELECT min_votes,max_votes,winning_candidates_num,is_acclaimed,is_active,is_encrypted,external_id,presentation,tally_configuration,conditions FROM contest",
    vec![2.into(),7.into(),3.into(),0.into(),1.into(),1.into(),SqlValue::Text("external".into()),SqlValue::Text("{\"p\":1}".into()),SqlValue::Text("{\"t\":2}".into()),SqlValue::Text("{\"c\":3}".into())]);

mapping!(election_absent_kiosk_defaults_false_without_filling_optional_counts,
    election::create_election_sqlite, "election",
    json!({"id":"election","tenant_id":"tenant","election_event_id":"event"}),
    "SELECT id,tenant_id,election_event_id,is_kiosk,num_allowed_revotes,statistics FROM election",
    vec![SqlValue::Text("election".into()),SqlValue::Text("tenant".into()),SqlValue::Text("event".into()),0.into(),SqlValue::Null,SqlValue::Null]);

mapping!(candidate_result_counts_and_percentages_are_distinct,
    results_contest_candidate::create_results_contest_candidates_sqlite, "results_contest_candidate",
    json!({"id":"row","tenant_id":"tenant","election_event_id":"event","election_id":"election","contest_id":"contest","candidate_id":"candidate","results_event_id":"results",
        "cast_votes":17,"winning_position":2,"points":31,"cast_votes_percent":12.5}),
    "SELECT candidate_id,results_event_id,cast_votes,winning_position,points,cast_votes_percent FROM results_contest_candidate",
    vec![SqlValue::Text("candidate".into()),SqlValue::Text("results".into()),17.into(),2.into(),31.into(),12.5.into()]);

mapping!(area_candidate_results_keep_area_and_candidate_separate,
    results_area_contest_candidate::create_results_area_contest_candidates_sqlite, "results_area_contest_candidate",
    json!({"id":"row","tenant_id":"tenant","election_event_id":"event","election_id":"election","contest_id":"contest","area_id":"area","candidate_id":"candidate","results_event_id":"results",
        "cast_votes":0,"points":9}),
    "SELECT area_id,candidate_id,cast_votes,winning_position,points,cast_votes_percent FROM results_area_contest_candidate",
    vec![SqlValue::Text("area".into()),SqlValue::Text("candidate".into()),0.into(),SqlValue::Null,9.into(),SqlValue::Null]);

mapping!(contest_result_vote_categories_do_not_shift,
    results_contest::create_results_contest_sqlite, "results_contest",
    json!({"id":"row","tenant_id":"tenant","election_event_id":"event","election_id":"election","contest_id":"contest","results_event_id":"results",
        "elegible_census":101,"total_valid_votes":11,"explicit_invalid_votes":2,"implicit_invalid_votes":3,"total_blank_votes":13,"explicit_blank_votes":5,"implicit_blank_votes":8,
        "total_invalid_votes":5,"total_votes":29,"total_auditable_votes":7,"total_auditable_votes_percent":25.0,"labels":{"l":1},"annotations":{"a":2}}),
    "SELECT elegible_census,total_valid_votes,explicit_invalid_votes,implicit_invalid_votes,total_blank_votes,explicit_blank_votes,implicit_blank_votes,total_invalid_votes,total_votes,total_auditable_votes,total_auditable_votes_percent,labels,annotations FROM results_contest",
    vec![101.into(),11.into(),2.into(),3.into(),13.into(),5.into(),8.into(),5.into(),29.into(),7.into(),25.0.into(),SqlValue::Text("{\"l\":1}".into()),SqlValue::Text("{\"a\":2}".into())]);

#[tokio::test]
async fn area_contest_uses_the_requested_tenant_and_event() {
    let mut db = Connection::open_in_memory().unwrap();
    let tx = db.transaction().unwrap();
    area_contest::create_area_contest_sqlite(
        &tx,
        "tenant",
        "event",
        vec![entity(
            json!({"id":"link","area_id":"area","contest_id":"contest"}),
        )],
    )
    .await
    .unwrap();
    assert_eq!(strings(&tx,"SELECT id,tenant_id,election_event_id,area_id,contest_id,labels,annotations FROM area_contest"),vec![SqlValue::Text("link".into()),SqlValue::Text("tenant".into()),SqlValue::Text("event".into()),SqlValue::Text("area".into()),SqlValue::Text("contest".into()),SqlValue::Null,SqlValue::Null]);
}

#[tokio::test]
async fn duplicate_entity_rejection_can_roll_back_the_whole_export() {
    let mut db = Connection::open_in_memory().unwrap();
    let tx = db.transaction().unwrap();
    let fixture =
        json!({"id":"area","tenant_id":"tenant","election_event_id":"event"});
    let error = area::create_area_sqlite(
        &tx,
        vec![entity(fixture.clone()), entity(fixture)],
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error.downcast_ref::<rusqlite::Error>(),Some(rusqlite::Error::SqliteFailure(e,_)) if e.code == rusqlite::ErrorCode::ConstraintViolation)
    );
    tx.rollback().unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='area'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn results_event_lookup_and_update_enforce_event_and_tenant() {
    use sequent_core::types::results::ResultDocuments;
    let mut db = Connection::open_in_memory().unwrap();
    let tx = db.transaction().unwrap();
    assert_eq!(
        results_event::create_results_event_sqlite(
            &tx, "tenant", "event", "result"
        )
        .await
        .unwrap(),
        "result"
    );
    tx.execute("UPDATE results_event SET name='Published',labels='{\"label\":1}',annotations='{\"note\":2}',created_at='2026-01-02 03:04:05',last_updated_at='2026-02-03 04:05:06'",[]).unwrap();
    let found =
        results_event::find_results_event_sqlite(&tx, "tenant", "event")
            .unwrap();
    assert_eq!(found.id, "result");
    assert_eq!(found.name.as_deref(), Some("Published"));
    assert_eq!(found.labels, Some(json!({"label":1})));
    assert_eq!(found.annotations, Some(json!({"note":2})));
    assert_eq!(
        found
            .created_at
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        "2026-01-02 03:04:05"
    );
    assert!(
        results_event::find_results_event_sqlite(&tx, "other", "event")
            .unwrap_err()
            .to_string()
            .contains("Query returned no rows")
    );
    let docs = ResultDocuments::default();
    results_event::update_results_event_documents_sqlite(
        &tx, "tenant", "result", "event", &docs,
    )
    .unwrap();
    assert_eq!(
        results_event::update_results_event_documents_sqlite(
            &tx, "other", "result", "event", &docs
        )
        .unwrap_err()
        .to_string(),
        "Rows not found in table results_event"
    );
}

#[test]
fn translation_replacement_rejects_corrupt_presentations_and_preserves_other_events(
) {
    use election_event::replace_election_event_translation_overrides_sqlite as replace;
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE election_event(id TEXT,presentation TEXT);INSERT INTO election_event VALUES('one',NULL),('two','{\"css\":\"keep\"}');").unwrap();
    replace(&db, "one", Some(&json!({"en":{"key":"value"}}))).unwrap();
    assert_eq!(
        strings(
            &db,
            "SELECT presentation FROM election_event WHERE id='one'"
        ),
        vec![SqlValue::Text(
            "{\"i18n\":{\"en\":{\"key\":\"value\"}}}".into()
        )]
    );
    assert_eq!(
        strings(
            &db,
            "SELECT presentation FROM election_event WHERE id='two'"
        ),
        vec![SqlValue::Text("{\"css\":\"keep\"}".into())]
    );
    for (raw, message) in [
        ("{", "Invalid election event presentation"),
        ("[]", "is not an object"),
    ] {
        db.execute(
            "UPDATE election_event SET presentation=?1 WHERE id='one'",
            [raw],
        )
        .unwrap();
        assert!(replace(&db, "one", None)
            .unwrap_err()
            .to_string()
            .contains(message));
    }
    assert!(replace(&db, "missing", None)
        .unwrap_err()
        .to_string()
        .contains("missing"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn candidate_csv_valid_null_and_boolean_controls_and_rejected_rows() {
    use std::io::Write;
    let mut db = Connection::open_in_memory().unwrap();
    let tx = db.transaction().unwrap();
    candidate::create_candidate_sqlite(&tx).await.unwrap();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file,"id,tenant,event,contest,created,updated,labels,annotations,description,type,presentation,is_public,image,external\ncandidate,tenant,event,contest,,,,,Árbol,person,,true,,external").unwrap();
    candidate::import_candidate_sqlite(&tx, file.path())
        .await
        .unwrap();
    assert_eq!(strings(&tx,"SELECT id,description,is_public,image_document_id,external_id FROM candidate"),vec![SqlValue::Text("candidate".into()),SqlValue::Text("Árbol".into()),1.into(),SqlValue::Null,SqlValue::Text("external".into())]);
    for (text, message) in [
        (
            "a,b,c,d,e,f,g,h,i,j,k,l,m,n\nx,t,e,c,,,,,Name,person,,maybe,,\n",
            "Invalid boolean in is_public column: maybe",
        ),
        ("a,b,c\nx,t,e\n", "Error fetching String record"),
        ("a,b,c\nx,t\n", "CSV parse error"),
    ] {
        let mut invalid = tempfile::NamedTempFile::new().unwrap();
        invalid.write_all(text.as_bytes()).unwrap();
        assert!(candidate::import_candidate_sqlite(&tx, invalid.path())
            .await
            .unwrap_err()
            .to_string()
            .contains(message));
    }
    assert_eq!(
        tx.query_row("SELECT COUNT(*) FROM candidate", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

mapping!(election_results_keep_census_turnout_and_blanks_separate,
    results_election::create_results_election_sqlite,"results_election",
    json!({"id":"row","tenant_id":"tenant","election_event_id":"event","election_id":"election","results_event_id":"results","name":"Name","elegible_census":101,"total_voters":41,"total_voters_percent":40.5,"blank_ballots":3,"blank_ballots_percent":7.5}),
    "SELECT elegible_census,total_voters,total_voters_percent,blank_ballots,blank_ballots_percent FROM results_election",
    vec![101.into(),41.into(),40.5.into(),3.into(),7.5.into()]);

mapping!(area_results_preserve_null_zero_and_auditable_counts,
    results_area_contest::create_results_area_contests_sqlite,"results_area_contest",
    json!({"id":"row","tenant_id":"tenant","election_event_id":"event","election_id":"election","contest_id":"contest","area_id":"area","results_event_id":"results","total_votes":0,"total_auditable_votes":7,"total_auditable_votes_percent":12.5}),
    "SELECT total_votes,total_valid_votes,total_auditable_votes,total_auditable_votes_percent FROM results_area_contest",
    vec![0.into(),SqlValue::Null,7.into(),12.5.into()]);

mapping!(tally_resolution_status_and_identity_are_persisted,
    tally_session_resolution::create_tally_session_resolutions_sqlite,"tally_session_resolution",
    json!({"id":"resolution","tenant_id":"tenant","election_event_id":"event","tally_session_id":"session","contest_id":"contest","resolution_type":"irv_tie_break","status":"resolved","resolved_by_user":"operator","resolved_at":"2026-01-02T03:04:05Z","labels":{"l":1},"annotations":{"a":2}}),
    "SELECT tally_session_id,contest_id,resolution_type,status,resolved_by_user,resolved_at FROM tally_session_resolution",
    vec![SqlValue::Text("session".into()),SqlValue::Text("contest".into()),SqlValue::Text("irv_tie_break".into()),SqlValue::Text("resolved".into()),SqlValue::Text("operator".into()),SqlValue::Text("2026-01-02T03:04:05+00:00".into())]);

#[tokio::test]
async fn event_creation_and_area_documents_preserve_the_supplied_content() {
    use sequent_core::types::results::ResultDocuments;
    let mut db = Connection::open_in_memory().unwrap();
    let tx = db.transaction().unwrap();
    election_event::create_election_event_sqlite(&tx,entity(json!({"id":"event","tenant_id":"tenant","encryption_protocol":"ELGAMAL","description":"Event description","presentation":{"p":1},"is_archived":true,"is_audit":false}))).await.unwrap();
    assert_eq!(strings(&tx,"SELECT id,tenant_id,encryption_protocol,description,presentation,is_archived,is_audit FROM election_event"),vec![SqlValue::Text("event".into()),SqlValue::Text("tenant".into()),SqlValue::Text("ELGAMAL".into()),SqlValue::Text("Event description".into()),SqlValue::Text("{\"p\":1}".into()),1.into(),0.into()]);
    let docs = ResultDocuments {
        pdf: Some("report.pdf".into()),
        json: Some("report.json".into()),
        ..Default::default()
    };
    results_election_area::create_results_election_area_sqlite(
        &tx,
        "tenant",
        "results",
        "event",
        "election",
        "area",
        "Árbol",
        &docs,
        Some(0),
        None,
    )
    .await
    .unwrap();
    assert_eq!(strings(&tx,"SELECT area_id,name,blank_ballots,blank_ballots_percent,json_extract(documents,'$.pdf'),json_extract(documents,'$.json') FROM results_election_area"),vec![SqlValue::Text("area".into()),SqlValue::Text("Árbol".into()),0.into(),SqlValue::Null,SqlValue::Text("report.pdf".into()),SqlValue::Text("report.json".into())]);
}

#[tokio::test]
async fn document_updates_match_every_scope_key_and_keep_existing_annotations()
{
    use sequent_core::types::results::ResultDocuments;
    let mut db = Connection::open_in_memory().unwrap();
    let tx = db.transaction().unwrap();
    let fixture = json!({"id":"row","tenant_id":"tenant","election_event_id":"event","election_id":"election","contest_id":"contest","area_id":"area","results_event_id":"results"});
    results_election::create_results_election_sqlite(
        &tx,
        vec![entity(fixture.clone())],
    )
    .await
    .unwrap();
    results_contest::create_results_contest_sqlite(
        &tx,
        vec![entity(fixture.clone())],
    )
    .await
    .unwrap();
    results_area_contest::create_results_area_contests_sqlite(
        &tx,
        vec![entity(fixture)],
    )
    .await
    .unwrap();
    tx.execute("UPDATE results_election SET annotations='{\"keep\":1}'", [])
        .unwrap();
    let docs = ResultDocuments {
        html: Some("report.html".into()),
        ..Default::default()
    };
    results_election::update_results_election_documents_sqlite(
        &tx, "tenant", "results", "event", "election", &docs, "abc123",
    )
    .await
    .unwrap();
    results_contest::update_results_contest_documents_sqlite(
        &tx, "tenant", "results", "event", "election", "contest", &docs,
    )
    .await
    .unwrap();
    results_area_contest::update_results_area_contest_documents_sqlite(
        &tx, "tenant", "results", "event", "election", "contest", "area", &docs,
    )
    .await
    .unwrap();
    assert_eq!(strings(&tx,"SELECT json_extract(annotations,'$.keep'),json_extract(annotations,'$.results_hash'),json_extract(documents,'$.html') FROM results_election"),vec![1.into(),SqlValue::Text("abc123".into()),SqlValue::Text("report.html".into())]);
    for scope in 0..6 {
        let mut keys =
            ["tenant", "results", "event", "election", "contest", "area"];
        keys[scope] = "other";
        assert_eq!(
            results_area_contest::update_results_area_contest_documents_sqlite(
                &tx, keys[0], keys[1], keys[2], keys[3], keys[4], keys[5],
                &docs
            )
            .await
            .unwrap_err()
            .to_string(),
            "Rows not found in table results_area_contest"
        );
        if scope < 5 {
            assert_eq!(
                results_contest::update_results_contest_documents_sqlite(
                    &tx, keys[0], keys[1], keys[2], keys[3], keys[4], &docs
                )
                .await
                .unwrap_err()
                .to_string(),
                "Rows not found in table results_contest"
            );
        }
        if scope < 4 {
            assert_eq!(
                results_election::update_results_election_documents_sqlite(
                    &tx, keys[0], keys[1], keys[2], keys[3], &docs, "changed"
                )
                .await
                .unwrap_err()
                .to_string(),
                "Rows not found in table results_election"
            );
        }
    }
}
