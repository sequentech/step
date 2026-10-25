// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use crate::route_services::http::{Exchange, HttpServer};
use crate::route_services::{json, post, rows, Services};
use crate::services::transcript_export::TranscriptSource;
use crate::test_claims::Claims;
use protocol_board::{verification::PublicTranscript, BoardHandle};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Source(Mutex<Vec<(String, Option<String>)>>);
#[rocket::async_trait]
impl TranscriptSource for Source {
    async fn fetch(
        &self,
        dkg: &BoardName,
        tally: Option<&BoardName>,
    ) -> Result<PublicTranscript> {
        self.0
            .lock()
            .unwrap()
            .push((dkg.to_string(), tally.map(ToString::to_string)));
        Ok(PublicTranscript::new(
            vec![vec![0, 1, 2]],
            tally.map(|_| vec![vec![3, 4]]),
        ))
    }
}
struct Fixture {
    event: rows::Event,
    ceremony: Uuid,
    parent: Uuid,
    child: Uuid,
    session: Uuid,
    election: Uuid,
}
async fn fixture(services: &Services) -> Fixture {
    let event = rows::event(&services.hasura).await;
    let tenant = Uuid::parse_str(&event.tenant_id).unwrap();
    let event_id = Uuid::parse_str(&event.election_event_id).unwrap();
    let ceremony = Uuid::new_v4();
    let parent = Uuid::new_v4();
    let child = Uuid::new_v4();
    let session = Uuid::new_v4();
    rows::execute(&services.hasura,
        "INSERT INTO sequent_backend.keys_ceremony (id,tenant_id,election_event_id,trustee_ids,threshold) VALUES ($1,$2,$3,'{}',1)", &[&ceremony,&tenant,&event_id]).await;
    let election =
        Uuid::parse_str(&event.election(&services.hasura).await).unwrap();
    // Seed fixture state through the server-trusted write context; triggers stay enabled.
    rows::execute(&services.hasura,"UPDATE sequent_backend.election SET status=$2 WHERE id=$1 AND set_config('sequent.trusted_write', 'on', true)='on'",
        &[&election,&json!({"voting_status":"CLOSED","is_published":true,"allow_tally":"allowed"})]).await;
    rows::execute(&services.hasura,
        "INSERT INTO sequent_backend.tally_session (id,tenant_id,election_event_id,keys_ceremony_id,threshold,tally_type,execution_status,is_execution_completed,election_ids) VALUES ($1,$2,$3,$4,1,'ELECTORAL_RESULTS','SUCCESS',true,$5)",
        &[&session,&tenant,&event_id,&ceremony,&vec![election]]).await;
    rows::execute(&services.hasura,
        "INSERT INTO sequent_backend.protocol_board (id,tenant_id,election_event_id,keys_ceremony_id,name,manager_message) VALUES ($1,$2,$3,$4,$5,$6)",
        &[&parent,&tenant,&event_id,&ceremony,&BoardName::for_dkg(&ceremony).to_string(),&vec![0u8]]).await;
    rows::execute(&services.hasura,
        "INSERT INTO sequent_backend.protocol_board (id,tenant_id,election_event_id,keys_ceremony_id,parent_id,tally_session_id,batch,name,manager_message) VALUES ($1,$2,$3,$4,$5,$6,0,$7,$8)",
        &[&child,&tenant,&event_id,&ceremony,&parent,&session,&BoardName::for_tally(&session,0).to_string(),&vec![0u8]]).await;
    Fixture {
        event,
        ceremony,
        parent,
        child,
        session,
        election,
    }
}
fn request(f: &Fixture, board: Uuid) -> Value {
    json!({"election_event_id":f.event.election_event_id,"protocol_board_id":board})
}
async fn call(s: &Services, c: &Claims, input: Value) -> (Status, Value) {
    let client = s.client().await;
    json(post(&client, "/export-verification-transcript", c, &input).await)
        .await
}
fn claims(f: &Fixture) -> Claims {
    Claims::new(&f.event.tenant_id, "board")
        .roles([Permissions::ADMIN_CEREMONY])
}

#[rocket::async_test]
async fn exports_raw_public_evidence_to_a_new_private_event_document() {
    use crate::ports::documents::DocumentStorage;
    let mut s = Services::on_test_database().await;
    let source = Arc::new(Source::default());
    s.transcripts = source.clone();
    let f = fixture(&s).await;
    for board in [f.child] {
        let (status, body) = call(&s, &claims(&f), request(&f, board)).await;
        assert_eq!(status, Status::Ok, "{body}");
        let id = body["document_id"].as_str().unwrap();
        assert_eq!(body["url"], format!("https://documents.test/{id}"));
        let mut db = s.hasura.get().await.unwrap();
        let tx = db.transaction().await.unwrap();
        let doc = windmill::postgres::document::get_document(
            &tx,
            &f.event.tenant_id,
            Some(f.event.election_event_id.clone()),
            id,
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(doc.is_public, Some(false));
        assert_eq!(doc.media_type.as_deref(), Some("application/json"));
        assert_eq!(
            doc.name.as_deref(),
            Some(format!("verification-{board}.json").as_str())
        );
        let file = s
            .documents
            .download(&f.event.tenant_id, &doc)
            .await
            .unwrap();
        let contents: Value =
            serde_json::from_slice(&std::fs::read(file.path()).unwrap())
                .unwrap();
        assert_eq!(contents["version"], 1);
        assert_eq!(contents["dkg_messages"], json!(["AAEC"]));
        assert_eq!(
            contents["tally_messages"],
            if board == f.parent {
                Value::Null
            } else {
                json!(["AwQ="])
            }
        );
    }
    assert_eq!(
        *source.0.lock().unwrap(),
        vec![(
            BoardName::for_dkg(&f.ceremony).to_string(),
            Some(BoardName::for_tally(&f.session, 0).to_string())
        )]
    );
}

#[rocket::async_test]
async fn rejects_cross_tenant_cross_event_missing_board_and_tally_labels_before_fetch(
) {
    let mut s = Services::on_test_database().await;
    let source = Arc::new(Source::default());
    s.transcripts = source.clone();
    let f = fixture(&s).await;
    let other = fixture(&s).await;
    for (c, input) in [
        (claims(&other), request(&f, f.child)),
        (
            claims(&f),
            json!({"election_event_id":other.event.election_event_id,"protocol_board_id":f.child}),
        ),
        (claims(&f), request(&f, Uuid::new_v4())),
    ] {
        assert_eq!(call(&s, &c, input).await.0, Status::NotFound);
    }
    rows::execute(&s.hasura,"UPDATE sequent_backend.tally_session SET permission_label=ARRAY['restricted'] WHERE id=$1",&[&f.session]).await;
    assert_eq!(
        call(&s, &claims(&f), request(&f, f.child)).await.0,
        Status::NotFound
    );
    rows::execute(&s.hasura,"UPDATE sequent_backend.tally_session SET permission_label=NULL WHERE id=$1",&[&f.session]).await;
    rows::execute(&s.hasura,"UPDATE sequent_backend.keys_ceremony SET permission_label=ARRAY['restricted'] WHERE id=$1",&[&f.ceremony]).await;
    for board in [f.child] {
        assert_eq!(
            call(&s, &claims(&f), request(&f, board)).await.0,
            Status::NotFound
        );
    }
    assert!(source.0.lock().unwrap().is_empty());
    let permitted = claims(&f).permission_labels(&["restricted"]);
    for board in [f.child] {
        assert_eq!(
            call(&s, &permitted, request(&f, board)).await.0,
            Status::Ok
        );
    }
    rows::execute(&s.hasura,"UPDATE sequent_backend.election SET permission_label='election-only' WHERE id=$1",&[&f.election]).await;
    assert_eq!(
        call(&s, &permitted, request(&f, f.child)).await.0,
        Status::NotFound
    );
    let all_labels =
        claims(&f).permission_labels(&["restricted", "election-only"]);
    assert_eq!(
        call(&s, &all_labels, request(&f, f.child)).await.0,
        Status::Ok
    );
}

#[rocket::async_test]
async fn rejects_wrong_parent_ceremony_and_noncanonical_board_name() {
    let mut s = Services::on_test_database().await;
    let source = Arc::new(Source::default());
    s.transcripts = source.clone();
    let f = fixture(&s).await;
    let ceremony = Uuid::new_v4();
    let parent = Uuid::new_v4();
    let tenant = Uuid::parse_str(&f.event.tenant_id).unwrap();
    let event = Uuid::parse_str(&f.event.election_event_id).unwrap();
    rows::execute(&s.hasura,"INSERT INTO sequent_backend.keys_ceremony (id,tenant_id,election_event_id,trustee_ids,threshold) VALUES($1,$2,$3,'{}',1)",&[&ceremony,&tenant,&event]).await;
    rows::execute(&s.hasura,"INSERT INTO sequent_backend.protocol_board(id,tenant_id,election_event_id,keys_ceremony_id,name,manager_message) VALUES($1,$2,$3,$4,$5,$6)",&[&parent,&tenant,&event,&ceremony,&BoardName::for_dkg(&ceremony).to_string(),&vec![0u8]]).await;
    rows::execute(
        &s.hasura,
        "UPDATE sequent_backend.protocol_board SET parent_id=$2 WHERE id=$1",
        &[&f.child, &parent],
    )
    .await;
    assert_eq!(
        call(&s, &claims(&f), request(&f, f.child)).await.0,
        Status::InternalServerError
    );
    rows::execute(
        &s.hasura,
        "UPDATE sequent_backend.protocol_board SET parent_id=$2 WHERE id=$1",
        &[&f.child, &f.parent],
    )
    .await;
    rows::execute(&s.hasura,"UPDATE sequent_backend.protocol_board SET name='../../other' WHERE id=$1",&[&f.parent]).await;
    assert_eq!(
        call(&s, &claims(&f), request(&f, f.child)).await.0,
        Status::InternalServerError
    );
    assert!(source.0.lock().unwrap().is_empty());
}

#[rocket::async_test]
async fn accepts_no_caller_url_path_or_tenant_override() {
    let s = Services::without_database();
    let c = Claims::new(&Uuid::new_v4().to_string(), "board")
        .roles([Permissions::ADMIN_CEREMONY]);
    for field in ["board_url", "tenant_id", "document_id", "name"] {
        let mut input = json!({"election_event_id":Uuid::new_v4(),"protocol_board_id":Uuid::new_v4()});
        input[field] = json!("forbidden");
        let client = s.client().await;
        assert_eq!(
            post(&client, "/export-verification-transcript", &c, &input)
                .await
                .status(),
            Status::UnprocessableEntity
        );
    }
}

fn listing(content: Value, url: Option<&str>) -> Value {
    json!({"messages":[{"id":"1","version":"2","content_type":content,"download_url":url}]})
}
#[rocket::async_test]
async fn native_export_preserves_bytes_and_enforces_metadata_message_and_version_bounds(
) {
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let path = format!("/boards/{board}/messages");
    for (body, metadata, bytes, count, success) in [
        (listing(json!({"message":"AAEC"}), None), 4096, 3, 1, true),
        (listing(json!({"message":"AAEC"}), None), 10, 3, 1, false),
        (listing(json!({"message":"AAEC"}), None), 4096, 2, 1, false),
        (listing(json!({"message":"AAEC"}), None), 4096, 3, 0, false),
        (
            json!({"messages":[{"id":"1","version":"wrong","content_type":{"message":"AAEC"},"download_url":null}]}),
            4096,
            3,
            1,
            false,
        ),
    ] {
        let peer =
            HttpServer::start(vec![Exchange::json("GET", &path, 200, body)]);
        let result = BoardHandle::new(&peer.url, board.clone())
            .export_messages_bounded(
                Duration::from_secs(2),
                metadata,
                bytes,
                count,
                format!("{}/bucket/{board}/messages/", peer.url),
            )
            .await;
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(result.unwrap(), vec![vec![0, 1, 2]]);
        }
        assert_eq!(peer.finish().len(), 1);
    }
}

#[rocket::async_test]
async fn native_export_checks_blob_origin_bucket_board_key_and_refuses_redirects(
) {
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let object = Uuid::new_v4().to_string();
    let key = format!("{board}/messages/{object}");
    let blobpath = format!("/bucket/{key}");
    let blob = HttpServer::start(vec![Exchange::json(
        "GET",
        &blobpath,
        200,
        Value::Null,
    )
    .body("abc")]);
    let good = format!("{}{blobpath}?synthetic=1", blob.url);
    let prefix = format!("{}/bucket/{board}/messages/", blob.url);
    for (url, selectedkey, success) in [
        (good.clone(), key.clone(), true),
        (good.replace("/bucket/", "/other/"), key.clone(), false),
        (good.replace("127.0.0.1", "localhost"), key.clone(), false),
        (good.replace(&object, "../another"), key.clone(), false),
        (good.clone(), format!("other/messages/{object}"), false),
    ] {
        let path = format!("/boards/{board}/messages");
        let peer = HttpServer::start(vec![Exchange::json(
            "GET",
            &path,
            200,
            listing(json!({"key":selectedkey}), Some(&url)),
        )]);
        let result = BoardHandle::new(&peer.url, board.clone())
            .export_messages_bounded(
                Duration::from_secs(2),
                4096,
                10,
                1,
                prefix.clone(),
            )
            .await;
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(result.unwrap(), vec![b"abc".to_vec()]);
        }
        assert_eq!(peer.finish().len(), 1);
    }
    assert_eq!(blob.finish().len(), 1);
    let path = format!("/boards/{board}/messages");
    let peer =
        HttpServer::start(vec![Exchange::json("GET", &path, 302, Value::Null)
            .header("Location", "http://127.0.0.1:1/forbidden")]);
    assert!(BoardHandle::new(&peer.url, board)
        .export_messages_bounded(Duration::from_secs(2), 4096, 10, 1, prefix)
        .await
        .is_err());
    assert_eq!(peer.finish().len(), 1);
}

#[rocket::async_test]
async fn native_export_timeout_cancels_the_actual_http_socket() {
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut buffer = [0; 2048];
        assert!(stream.read(&mut buffer).unwrap() > 0);
        assert_eq!(stream.read(&mut buffer).unwrap(), 0);
    });
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let start = std::time::Instant::now();
    let result = BoardHandle::new(&url, board.clone())
        .export_messages_bounded(
            Duration::from_millis(100),
            4096,
            10,
            1,
            format!("{url}/bucket/{board}/messages/"),
        )
        .await;
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
    peer.join().unwrap();
}

#[rocket::async_test]
async fn canceled_callers_do_not_release_running_native_export_capacity() {
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (accepted_tx, accepted_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let accepted = accepted_tx.clone();
            workers.push(std::thread::spawn(move || {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut buffer = [0; 2048];
                assert!(stream.read(&mut buffer).unwrap() > 0);
                accepted.send(()).unwrap();
                assert_eq!(stream.read(&mut buffer).unwrap(), 0);
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let mut callers = Vec::new();
    for _ in 0..2 {
        let handle = BoardHandle::new(&url, board.clone());
        let prefix = format!("{url}/bucket/{board}/messages/");
        callers.push(tokio::spawn(async move {
            handle
                .export_messages_bounded(
                    Duration::from_millis(500),
                    4096,
                    10,
                    1,
                    prefix,
                )
                .await
        }));
    }
    // Poll asynchronously so both callers get to dispatch before cancellation.
    tokio::time::timeout(Duration::from_secs(1), async {
        for _ in 0..2 {
            loop {
                if accepted_rx.try_recv().is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    })
    .await
    .unwrap();
    for caller in callers {
        caller.abort();
        let _ = caller.await;
    }
    let third = BoardHandle::new(&url, board)
        .export_messages_bounded(
            Duration::from_secs(1),
            4096,
            10,
            1,
            "http://127.0.0.1/bucket/".into(),
        )
        .await;
    assert!(third.unwrap_err().to_string().contains("busy"));
    tokio::time::sleep(Duration::from_millis(600)).await;
    peer.join().unwrap();
}

#[rocket::async_test]
async fn transport_failure_does_not_return_presigned_urls_or_create_documents()
{
    struct FailingSource;
    #[rocket::async_trait]
    impl TranscriptSource for FailingSource {
        async fn fetch(
            &self,
            _: &BoardName,
            _: Option<&BoardName>,
        ) -> Result<PublicTranscript> {
            anyhow::bail!("https://private.test/object?token=synthetic-secret")
        }
    }
    let mut s = Services::on_test_database().await;
    s.transcripts = Arc::new(FailingSource);
    let f = fixture(&s).await;
    let (status, body) = call(&s, &claims(&f), request(&f, f.child)).await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(body["message"], "Could not export the public transcript");
    assert!(!body.to_string().contains("synthetic-secret"));
    let db = s.hasura.get().await.unwrap();
    let count:i64=db.query_one("SELECT count(*) FROM sequent_backend.document WHERE tenant_id=$1 AND election_event_id=$2",&[&Uuid::parse_str(&f.event.tenant_id).unwrap(),&Uuid::parse_str(&f.event.election_event_id).unwrap()]).await.unwrap().get(0);
    assert_eq!(count, 0);
}

#[rocket::async_test]
async fn native_export_bounds_chunked_bodies_without_content_length() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut buffer = [0; 2048];
        assert!(stream.read(&mut buffer).unwrap() > 0);
        stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n14\r\n01234567890123456789\r\n0\r\n\r\n").unwrap();
    });
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let result = BoardHandle::new(&url, board.clone())
        .export_messages_bounded(
            Duration::from_secs(1),
            10,
            10,
            1,
            format!("{url}/bucket/{board}/messages/"),
        )
        .await;
    assert!(result.unwrap_err().to_string().contains("byte limit"));
    peer.join().unwrap();
}

#[rocket::async_test]
async fn refuses_dkg_initialization_incomplete_results_and_open_voting() {
    let mut s = Services::on_test_database().await;
    let source = Arc::new(Source::default());
    s.transcripts = source.clone();
    let f = fixture(&s).await;
    assert_eq!(
        call(&s, &claims(&f), request(&f, f.parent)).await.0,
        Status::NotFound
    );
    for update in [
        "tally_type='INITIALIZATION_REPORT'",
        "tally_type=NULL",
        "execution_status='IN_PROGRESS'",
        "is_execution_completed=false",
        "election_ids=NULL",
    ] {
        rows::execute(
            &s.hasura,
            &format!(
                "UPDATE sequent_backend.tally_session SET {update} WHERE id=$1"
            ),
            &[&f.session],
        )
        .await;
        assert_eq!(
            call(&s, &claims(&f), request(&f, f.child)).await.0,
            Status::NotFound,
            "{update}"
        );
        rows::execute(&s.hasura,"UPDATE sequent_backend.tally_session SET tally_type='ELECTORAL_RESULTS',execution_status='SUCCESS',is_execution_completed=true,election_ids=$2 WHERE id=$1",&[&f.session,&vec![f.election]]).await;
    }
    for status in [
        json!({"voting_status":"NOT_STARTED"}),
        json!({"voting_status":"OPEN"}),
        json!({"voting_status":"CLOSED","kiosk_voting_status":"OPEN"}),
        json!({"voting_status":"INVALID"}),
        Value::Null,
    ] {
        rows::execute(
            &s.hasura,
            "UPDATE sequent_backend.election SET status=$2 WHERE id=$1 AND set_config('sequent.trusted_write', 'on', true)='on'",
            &[&f.election, &status],
        )
        .await;
        assert_eq!(
            call(&s, &claims(&f), request(&f, f.child)).await.0,
            Status::NotFound
        );
    }
    assert!(source.0.lock().unwrap().is_empty());
}

#[rocket::async_test]
async fn rechecks_completed_tally_binding_after_fetch_before_storing_document()
{
    struct ChangingSource {
        pool: Arc<deadpool_postgres::Pool>,
        session: Uuid,
    }
    #[rocket::async_trait]
    impl TranscriptSource for ChangingSource {
        async fn fetch(
            &self,
            _: &BoardName,
            _: Option<&BoardName>,
        ) -> Result<PublicTranscript> {
            self.pool.get().await?.execute("UPDATE sequent_backend.tally_session SET execution_status='IN_PROGRESS' WHERE id=$1",&[&self.session]).await?;
            Ok(PublicTranscript::new(vec![vec![0]], Some(vec![vec![1]])))
        }
    }
    let mut s = Services::on_test_database().await;
    let f = fixture(&s).await;
    s.transcripts = Arc::new(ChangingSource {
        pool: s.hasura.clone(),
        session: f.session,
    });
    let (status, body) = call(&s, &claims(&f), request(&f, f.child)).await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(body["message"], "Could not export the public transcript");
    let db = s.hasura.get().await.unwrap();
    let count:i64=db.query_one("SELECT count(*) FROM sequent_backend.document WHERE tenant_id=$1 AND election_event_id=$2",&[&Uuid::parse_str(&f.event.tenant_id).unwrap(),&Uuid::parse_str(&f.event.election_event_id).unwrap()]).await.unwrap().get(0);
    assert_eq!(count, 0);
}
