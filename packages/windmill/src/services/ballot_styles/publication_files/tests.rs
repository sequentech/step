// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::publication_files::{
    MemoryGeneratedStyles, MemoryPublicationObjects, MemoryPublicationRows, PublicationRecord,
};
use aws_sdk_s3::Client;
use deadpool_postgres::Manager;
use sequent_core::services::s3::get_private_bucket;

#[test]
fn imported_references_cannot_escape_the_publication_scope() {
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let publication = Uuid::new_v4();
    let root = format!(
        "tenant-{tenant}/event-{event}/publication-{publication}/{}",
        Uuid::new_v4()
    );
    assert!(validate_publication_root(&root, tenant, event, publication).is_ok());
    assert!(validate_publication_root(&root, Uuid::new_v4(), event, publication).is_err());
    assert!(validate_publication_root(&root, tenant, Uuid::new_v4(), publication).is_err());
    assert!(validate_publication_root(&root, tenant, event, Uuid::new_v4()).is_err());
    assert!(
        validate_publication_root(&format!("{root}/../../other"), tenant, event, publication)
            .is_err()
    );
}

#[test]
fn shared_event_presentation_preserves_exact_signed_eml_bytes() {
    for eml in [
        r#"{"id":"style", "election_event_presentation": {"z":1,"a":"escaped \" text"},"contests":[]}"#,
        r#"{"contests":[],"election_event_presentation":null}"#,
        r#"{"contests":[]}"#,
    ] {
        let (prefix, shared, suffix) = split_event_presentation(eml).unwrap();
        assert_eq!(format!("{prefix}{shared}{suffix}"), eml);
    }
}

#[test]
fn equivalent_presentation_encodings_preserve_original_styles() -> Result<()> {
    let variants = [
        r#"{"election_event_presentation":{"z":[{"b":2,"a":1}],"a":"é"},"contests":[]}"#,
        r#"{"election_event_presentation": {"a":"\u00e9", "z":[{"a":1,"b":2}]},"contests":[]}"#,
    ];
    for first in variants {
        let mut shared = None;
        for eml in [first, variants[0], variants[1]] {
            let mut style = json!({"ballot_eml":eml, "ballot_signature":"original signature"});
            share_event_presentation(&mut style, &mut shared)?;
            let reconstructed = style["ballot_eml"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    format!(
                        "{}{}{}",
                        style["ballot_eml_prefix"].as_str().unwrap(),
                        shared.as_ref().unwrap().0,
                        style["ballot_eml_suffix"].as_str().unwrap()
                    )
                });
            assert_eq!(reconstructed, eml);
            assert_eq!(style["ballot_signature"], "original signature");
        }
        let mut different =
            json!({"ballot_eml":r#"{"election_event_presentation":{"a":"different"}}"#});
        assert!(share_event_presentation(&mut different, &mut shared).is_err());
    }
    Ok(())
}

#[test]
fn new_styles_order_nested_object_keys_without_reordering_arrays() -> Result<()> {
    let mut left = HashMap::new();
    left.insert("z", json!([{"b":2,"a":1}, 0]));
    left.insert("a", json!("é"));
    let mut right = HashMap::new();
    right.insert("a", json!("é"));
    right.insert("z", json!([{"a":1,"b":2}, 0]));
    let canonical = ordered_json_string(&left)?;
    assert_eq!(canonical, ordered_json_string(&right)?);
    assert_eq!(canonical, r#"{"a":"é","z":[{"a":1,"b":2},0]}"#);
    let mut shared = None;
    for presentation in [ordered_json_string(&left)?, ordered_json_string(&right)?] {
        let mut style =
            json!({"ballot_eml":format!(r#"{{"election_event_presentation":{presentation}}}"#)});
        share_event_presentation(&mut style, &mut shared)?;
        assert!(style["ballot_eml"].is_null());
    }
    Ok(())
}

async fn claim(pool: &Pool, tenant: &str, event: &str, publication: &str) -> Result<PgLock> {
    let lease = PgLock {
        key: format!("create_ballot_style-{tenant}-{event}-{publication}"),
        value: Uuid::new_v4().to_string(),
        expiry_date: None,
    };
    let changed = pool.get().await?.execute(
        "INSERT INTO sequent_backend.lock (key,value,expiry_date) VALUES ($1,$2,clock_timestamp()+interval '5 minutes') ON CONFLICT (key) DO UPDATE SET value=EXCLUDED.value,expiry_date=EXCLUDED.expiry_date WHERE sequent_backend.lock.expiry_date<clock_timestamp()",
        &[&lease.key, &lease.value],
    ).await?;
    assert_eq!(changed, 1);
    Ok(lease)
}

async fn generation_data(
    pool: &Pool,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<PublicationData> {
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    let data = publication_data(&tx, tenant, event, publication).await?;
    tx.commit().await?;
    Ok(data)
}

async fn files(
    pool: &Pool,
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
) -> Result<Value> {
    let references = {
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        let references = load_voter_files(&tx, tenant, event, area, elections).await?;
        tx.commit().await?;
        references
    };
    presign_voter_files(references).await
}

async fn object(client: &Client, bucket: &str, key: &str) -> Result<Value> {
    let bytes = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?
        .body
        .collect()
        .await?
        .into_bytes();
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL and private S3; use scripts/test_ballot_files.py"]
async fn publication_objects_and_authorized_references() -> Result<()> {
    let config = std::env::var("BALLOT_FILES_TEST_DSN")?.parse::<tokio_postgres::Config>()?;
    // A retained connection would deadlock the next generation heartbeat.
    let pool = Pool::builder(Manager::new(config, tokio_postgres::NoTls))
        .max_size(1)
        .build()?;
    {
        let db = pool.get().await?;
        db.batch_execute("DROP SCHEMA sequent_backend CASCADE; CREATE SCHEMA sequent_backend;
            CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY, tenant_id uuid, presentation jsonb, description text, status jsonb);
            CREATE TABLE sequent_backend.election(id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, annotations jsonb, created_at timestamptz, description text, is_consolidated_ballot_encoding bool, labels jsonb, last_updated_at timestamptz, presentation jsonb, spoil_ballot_option bool, num_allowed_revotes integer, voting_channels jsonb, status jsonb);
            CREATE TABLE sequent_backend.ballot_publication(id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb, labels jsonb, created_at timestamptz DEFAULT now(), created_by_user_id text, election_ids uuid[], election_id uuid, is_generated bool DEFAULT false, published_at timestamptz, deleted_at timestamptz, PRIMARY KEY (id,tenant_id,election_event_id));
            CREATE TABLE sequent_backend.ballot_style(id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, election_id uuid, area_id uuid, created_at timestamptz, last_updated_at timestamptz, annotations jsonb, labels jsonb, ballot_eml text, ballot_signature bytea, status text, deleted_at timestamptz, ballot_publication_id uuid);
            CREATE TABLE sequent_backend.lock(key text PRIMARY KEY,value text,expiry_date timestamptz);
            CREATE TABLE sequent_backend.document(id uuid PRIMARY KEY,tenant_id uuid,election_event_id uuid,name text,media_type text,size bigint,is_public bool,annotations jsonb,labels jsonb,created_at timestamptz,last_updated_at timestamptz);
            -- No configured or signed voting close: the empty inputs of the voters' display closes.
            CREATE TABLE sequent_backend.scheduled_event(id uuid, tenant_id uuid, election_event_id uuid, task_id text, event_payload jsonb, cron_config jsonb, archived_at timestamptz);
            CREATE TABLE sequent_backend.election_voting_window(tenant_id uuid, election_event_id uuid, election_id uuid, start_date text, end_date text);
            CREATE TABLE sequent_backend.signed_voting_boundary(tenant_id uuid, election_event_id uuid, election_id uuid, scheduled_event_id text, event_processor text, channels jsonb, source_timezone text, scheduled_date text, scheduled_at timestamptz, fingerprint text);
            CREATE TABLE sequent_backend.lifecycle_fired(tenant_id uuid, election_event_id uuid, scheduled_event_id uuid, election_id uuid, fingerprint text, fired_at timestamptz, executed_channels jsonb);
            CREATE FUNCTION sequent_backend.voting_window_election_id(sequent_backend.scheduled_event) RETURNS uuid LANGUAGE sql IMMUTABLE AS 'SELECT NULL::uuid';
            CREATE FUNCTION sequent_backend.voting_window_is_event_close(sequent_backend.scheduled_event) RETURNS boolean LANGUAGE sql IMMUTABLE AS 'SELECT false';
            CREATE FUNCTION sequent_backend.live_voting_close_allowed(uuid, uuid, uuid) RETURNS boolean LANGUAGE sql STABLE AS 'SELECT true';").await?;
        db.batch_execute(include_str!("../../../../../../hasura/migrations/backend-db/1788909000000_ballot_publication_style_index/up.sql")).await?;
    }
    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let election = Uuid::new_v4();
    let other_election = Uuid::new_v4();
    let area = Uuid::new_v4();
    let publication = Uuid::new_v4();
    let style = Uuid::new_v4();
    let other_style = Uuid::new_v4();
    let (t, ev, p, a) = (
        tenant.to_string(),
        event.to_string(),
        publication.to_string(),
        area.to_string(),
    );
    let ids = vec![election.to_string()];
    let eml = r#"{"contests": [], "public_key": {"public_key": "archived-public-key", "is_demo": false}, "election_event_presentation": {"title":"escaped \" text","nested":{"z":2,"a":1}}, "area_presentation":{}}"#;
    let other_eml = r#"{"contests": [], "public_key": {"public_key": "archived-public-key", "is_demo": false}, "election_event_presentation": {"nested":{"a":1,"z":2}, "title":"escaped \" text"}, "area_presentation":{}}"#;
    let client = get_shared_s3_client(s3::S3Endpoint::Server).await?;
    let bucket = get_private_bucket()?;
    let objects = S3PublicationStorage {
        endpoint: S3Endpoint::Server,
    }
    .open()
    .await?;
    let result: Result<()> = async {
        {
            let db = pool.get().await?;
            db.execute("INSERT INTO sequent_backend.election_event VALUES ($1,$2,'{}','before','{}')", &[&event,&tenant]).await?;
            for id in [election, other_election] {
                db.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,presentation,status,num_allowed_revotes) VALUES ($1,$2,$3,'{}','{}',1)", &[&id,&tenant,&event]).await?;
            }
            db.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids) VALUES ($1,$2,$3,$4)", &[&publication,&tenant,&event,&vec![election,other_election]]).await?;
            for (id, e, eml) in [(style,election,eml),(other_style,other_election,other_eml)] {
                db.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES ($1,$2,$3,$4,$5,$6,$7)", &[&id,&tenant,&event,&e,&area,&publication,&eml]).await?;
            }
            for _ in 0..STYLE_PAGE_SIZE {
                db.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES ($1,$2,$3,$4,$5,$6,$7)",
                    &[&Uuid::new_v4(),&tenant,&event,&election,&Uuid::new_v4(),&publication,&eml]).await?;
            }
        }
        let failed_publication = Uuid::new_v4();
        let failed_id = failed_publication.to_string();
        pool.get().await?.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids) VALUES ($1,$2,$3,$4)", &[&failed_publication,&tenant,&event,&vec![election]]).await?;
        pool.get().await?.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES ($1,$2,$3,$4,$5,$6,$7)", &[&Uuid::new_v4(),&tenant,&event,&election,&area,&failed_publication,&eml]).await?;
        let first_data = generation_data(&pool, &t, &ev, &failed_id).await?;
        let first_lease = claim(&pool, &t, &ev, &failed_id).await?;
        let failed_root = format!("tenant-{tenant}/event-{event}/publication-{failed_publication}/{}",first_lease.value);
        // Fail after the other objects upload; neither a reference nor readiness may commit.
        client.put_object().bucket(&bucket).key(format!("{failed_root}/event.json"))
            .body(ByteStream::from_static(b"{}")).send().await?;
        let upload_result = prepare_publication_files(&pool,&t,&ev,&failed_id,&first_lease,first_data).await;
        assert!(upload_result.is_err());
        let task_id = Uuid::new_v4();
        let task: sequent_core::types::hasura::core::TasksExecution = serde_json::from_value(json!({
            "id": task_id, "tenant_id": tenant, "election_event_id": event,
            "name": "generate", "task_type": "generate", "execution_status": "IN_PROGRESS",
            "created_at": chrono::Utc::now(), "executed_by_user": "test"
        }))?;
        pool.get().await?.batch_execute("CREATE TABLE sequent_backend.tasks_execution(id uuid, tenant_id uuid, execution_status text, logs jsonb, end_at timestamptz, annotations jsonb)").await?;
        pool.get().await?.execute("INSERT INTO sequent_backend.tasks_execution(id,tenant_id,execution_status) VALUES ($1,$2,'IN_PROGRESS')", &[&task_id,&tenant]).await?;
        assert!(crate::tasks::update_election_event_ballot_styles::record_generation_result(&task,upload_result).await.is_err());
        let task_row = pool.get().await?.query_one("SELECT execution_status,logs,end_at IS NOT NULL FROM sequent_backend.tasks_execution WHERE id=$1", &[&task_id]).await?;
        assert_eq!(task_row.get::<_,String>(0), "FAILED");
        assert!(task_row.get::<_,Value>(1).to_string().contains("Ballot publication object verification failed"));
        assert!(task_row.get::<_,bool>(2));
        {
            let db = pool.get().await?;
            let row = db.query_one("SELECT is_generated,annotations FROM sequent_backend.ballot_publication WHERE id=$1", &[&failed_publication]).await?;
            assert_eq!(row.get::<_,Option<bool>>(0), Some(false));
            assert!(row.get::<_,Option<Value>>(1).is_none());
            db.execute("UPDATE sequent_backend.lock SET expiry_date=clock_timestamp()-interval '1 second' WHERE key=$1", &[&first_lease.key]).await?;
            // A fresh generation uses current metadata after administrative edits.
            db.execute("UPDATE sequent_backend.election_event SET description='after', presentation=$1", &[&json!({"logo_url":"current-logo"})]).await?;
        }
        let lease = claim(&pool, &t, &ev, &p).await?;
        {
            let mut db = pool.get().await?;
            let tx = db.transaction().await?;
            assert!(lock_generation(&tx, &first_lease).await.is_err());
            tx.rollback().await?;
        }
        tokio::time::timeout(Duration::from_secs(30), prepare_publication_files(&pool,&t,&ev,&p,&lease,generation_data(&pool,&t,&ev,&p).await?)).await??;
        let root = format!("tenant-{tenant}/event-{event}/publication-{publication}/{}",lease.value);
        let event_object = object(&client,&bucket,&format!("{root}/event.json")).await?;
        assert_eq!(event_object["description"], "after");
        let style_object = object(&client, &bucket, &format!("{root}/style-{style}.json")).await?;
        for (id, original) in [(style, eml), (other_style, other_eml)] {
            let wire = object(&client,&bucket,&format!("{root}/style-{id}.json")).await?;
            let restored = wire["ballot_eml"].as_str().map(str::to_owned).unwrap_or_else(|| {
                format!("{}{}{}", wire["ballot_eml_prefix"].as_str().unwrap(),
                    event_object["ballot_eml_presentation"].as_str().unwrap(),
                    wire["ballot_eml_suffix"].as_str().unwrap())
            });
            assert_eq!(restored, original);
        }
        let public_bucket = get_public_bucket()?;
        let config_key = get_public_election_event_document_name_key(&t,&ev,EVENT_CONFIG_FILE_NAME);
        let public_object = object(&client,&public_bucket,&config_key).await?;
        assert_eq!(public_object["election_event_id"], ev);
        assert_eq!(public_object["election_event_presentation"]["logo_url"], "current-logo");
        assert_eq!(pool.get().await?.query_one("SELECT count(*) FROM sequent_backend.document WHERE election_event_id=$1 AND is_public", &[&event]).await?.get::<_,i64>(0),1);

        // A full first page of uncommitted documents must not hide an allowed
        // document on the next page. Publication objects never enter this archive.
        let prefix = format!("tenant-{tenant}/event-{event}/");
        let uncommitted = Uuid::nil();
        let document = Uuid::from_u128(u128::MAX).to_string();
        let padding = (0..1001).map(|i| (format!("{prefix}document-{uncommitted}/{i:04}.json"), json!({})));
        upload_batch(&objects,padding.collect()).await?;
        objects.put_immutable_json(&format!("{prefix}document-{document}/export.json"),&json!({"document":true})).await?;
        let archive = sequent_core::services::s3::get_files_from_s3(bucket.clone(),prefix,&HashSet::from([document.clone()])).await?;
        assert_eq!(archive.len(),1);
        assert!(archive[0].file_name().unwrap().to_str().unwrap().contains(&format!("document_{document}_export.json")));
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&archive[0])?)?,json!({"document":true}));
        assert_eq!(files(&pool,&t,&ev,&a,&ids).await?["files"], json!([]));
        pool.get().await?.execute("UPDATE sequent_backend.ballot_publication SET published_at=now() WHERE id=$1", &[&publication]).await?;
        super::super::publication_archive::tests::round_trip(&pool,&t,&ev,&p).await?;
        let response = files(&pool,&t,&ev,&a,&ids).await?;
        assert_eq!(response["files"].as_array().unwrap().len(), 1);
        assert_eq!(response["files"][0]["version"], root);
        assert!(!response.to_string().contains("ballot_eml"));
        for denied in [vec![],vec![Uuid::new_v4().to_string()]] {
            assert_eq!(files(&pool,&t,&ev,&a,&denied).await?["files"],json!([]));
        }
        assert_eq!(files(&pool,&t,&ev,&Uuid::new_v4().to_string(),&ids).await?["files"],json!([]));
        assert!(files(&pool,&Uuid::new_v4().to_string(),&ev,&a,&ids).await.is_err());
        assert!(files(&pool,&t,&Uuid::new_v4().to_string(),&a,&ids).await.is_err());

        let partial = Uuid::new_v4();
        let replacement = Uuid::new_v4();
        {
            let db = pool.get().await?;
            db.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids,election_id) VALUES ($1,$2,$3,$4,$5)", &[&partial,&tenant,&event,&vec![election],&election]).await?;
            db.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES ($1,$2,$3,$4,$5,$6,$7)", &[&replacement,&tenant,&event,&election,&area,&partial,&eml]).await?;
        }
        let partial_id = partial.to_string();
        let partial_data = generation_data(&pool,&t,&ev,&partial_id).await?;
        let partial_lease = claim(&pool,&t,&ev,&partial_id).await?;
        prepare_publication_files(&pool,&t,&ev,&partial_id,&partial_lease,partial_data).await?;
        assert_eq!(files(&pool,&t,&ev,&a,&ids).await?["files"][0]["id"],json!(style));
        {
            let mut db = pool.get().await?;
            let tx = db.transaction().await?;
            crate::postgres::ballot_publication::soft_delete_other_ballot_publications(
                &tx,&partial_id,&ev,&t,Some(election.to_string())).await?;
            tx.execute("UPDATE sequent_backend.ballot_publication SET published_at=now() WHERE id=$1", &[&partial]).await?;
            tx.commit().await?;
        }
        let both = files(&pool,&t,&ev,&a,&[election.to_string(),other_election.to_string()]).await?;
        assert_eq!(both["files"].as_array().unwrap().len(),2);
        assert_eq!(files(&pool,&t,&ev,&a,&ids).await?["files"][0]["id"],json!(replacement));
        assert_eq!(object(&client,&bucket,&format!("{root}/style-{style}.json")).await?,style_object);
        pool.get().await?.execute("UPDATE sequent_backend.election SET status='{\"voting_status\":\"PAUSED\"}',num_allowed_revotes=7 WHERE id=$1", &[&election]).await?;
        let changed = files(&pool,&t,&ev,&a,&ids).await?;
        assert_eq!(changed["files"][0]["status"]["voting_status"],"PAUSED");
        assert_eq!(changed["files"][0]["num_allowed_revotes"],7);
        pool.get().await?.execute("UPDATE sequent_backend.ballot_publication SET deleted_at=now() WHERE id=$1", &[&partial]).await?;
        assert_eq!(files(&pool,&t,&ev,&a,&ids).await?["files"],json!([]));
        assert!(generation_data(&pool,&t,&ev,&partial_id).await.is_err());
        Ok(())
    }.await;
    let cleanup = sequent_core::services::s3::delete_files_from_s3(
        bucket,
        format!("tenant-{tenant}/event-{event}/"),
        s3::S3Endpoint::Server,
    )
    .await;
    let public_cleanup = sequent_core::services::s3::delete_files_from_s3(
        get_public_bucket()?,
        format!("tenant-{tenant}/event-{event}/"),
        s3::S3Endpoint::Server,
    )
    .await;
    result?;
    cleanup?;
    public_cleanup?;
    Ok(())
}

const TENANT: Uuid = Uuid::from_u128(0xa1);
const EVENT: Uuid = Uuid::from_u128(0xe1);
const PUBLICATION: Uuid = Uuid::from_u128(0xb1);
const OTHER_PUBLICATION: Uuid = Uuid::from_u128(0xb2);
const AREA: Uuid = Uuid::from_u128(0xa2);
const ELECTION: Uuid = Uuid::from_u128(0xc1);
const OTHER_ELECTION: Uuid = Uuid::from_u128(0xc2);
const STYLE: Uuid = Uuid::from_u128(0x51);
const OTHER_STYLE: Uuid = Uuid::from_u128(0x52);
const ROOT: &str = "tenant-00000000-0000-0000-0000-0000000000a1/event-00000000-0000-0000-0000-0000000000e1/publication-00000000-0000-0000-0000-0000000000b1/00000000-0000-0000-0000-000000000001";
const RETRY_ROOT: &str = "tenant-00000000-0000-0000-0000-0000000000a1/event-00000000-0000-0000-0000-0000000000e1/publication-00000000-0000-0000-0000-0000000000b1/00000000-0000-0000-0000-000000000002";
const OTHER_ROOT: &str = "tenant-00000000-0000-0000-0000-0000000000a1/event-00000000-0000-0000-0000-0000000000e1/publication-00000000-0000-0000-0000-0000000000b2/00000000-0000-0000-0000-000000000009";
const PRESENTATION: &str = r#"{"language_conf":{"enabled_language_codes":["en","es"]}}"#;

/// Signed EML bytes, spaced irregularly around the event presentation.
fn eml(presentation: &str) -> String {
    format!(
        r#"{{"id":"style", "election_event_presentation": {presentation},"area_presentation":{{"name":"North"}},"election_dates":{{"first_started_at":"2026-10-01T08:00:00Z"}},"contests":[]}}"#
    )
}

fn style_row(id: Uuid, election: Uuid, eml: &str) -> Value {
    json!({"id": id, "election_id": election, "area_id": AREA, "ballot_eml": eml, "status": "PUBLISHED"})
}

fn event_object() -> Value {
    json!({"id": EVENT, "presentation": {"name": "Event"}, "description": null})
}

fn election_object(id: Uuid) -> Value {
    json!({"id": id, "presentation": {"name": "Election"}})
}

fn two_elections() -> Vec<Value> {
    vec![election_object(ELECTION), election_object(OTHER_ELECTION)]
}

fn consistent_styles() -> Vec<Value> {
    vec![
        style_row(STYLE, ELECTION, &eml(PRESENTATION)),
        style_row(OTHER_STYLE, OTHER_ELECTION, &eml(PRESENTATION)),
    ]
}

/// What a generation committed: the event and elections it read, and its styles.
fn generated(
    event: Value,
    elections: Vec<Value>,
    styles: Vec<Value>,
) -> (PublicationData, MemoryGeneratedStyles) {
    let data = PublicationData {
        event,
        elections,
        style_count: styles.len() as i64,
    };
    let styles = styles
        .into_iter()
        .map(|style| {
            let id = style["id"].as_str().and_then(|id| Uuid::parse_str(id).ok());
            (id.expect("style id"), style)
        })
        .collect();
    (data, MemoryGeneratedStyles::new(styles))
}

async fn write(
    objects: &MemoryPublicationObjects,
    root: &str,
    elections: Vec<Value>,
    styles: Vec<Value>,
) -> Result<()> {
    let (data, styles) = generated(event_object(), elections, styles);
    write_publication_objects(&styles, objects, root, data).await
}

fn keys(root: &str, names: &[String]) -> Vec<String> {
    names.iter().map(|name| format!("{root}/{name}")).collect()
}

fn election_keys() -> Vec<String> {
    keys(
        ROOT,
        &[
            format!("election-{ELECTION}.json"),
            format!("election-{OTHER_ELECTION}.json"),
        ],
    )
}

fn sorted(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys
}

#[tokio::test]
async fn uploading_writes_elections_then_each_style_then_the_event_under_the_attempt_root() {
    let objects = MemoryPublicationObjects::default();
    write(&objects, ROOT, two_elections(), consistent_styles())
        .await
        .unwrap();

    // Objects of one batch upload concurrently, in any order.
    let written = objects.keys();
    assert_eq!(sorted(written[..2].to_vec()), election_keys());
    assert_eq!(
        sorted(written[2..6].to_vec()),
        sorted(keys(
            ROOT,
            &[
                format!("summary-{STYLE}.json"),
                format!("style-{STYLE}.json"),
                format!("summary-{OTHER_STYLE}.json"),
                format!("style-{OTHER_STYLE}.json"),
            ],
        ))
    );
    assert_eq!(written[6..], [format!("{ROOT}/event.json")]);
}

#[tokio::test]
async fn uploaded_styles_drop_their_eml_and_rebuild_it_exactly_with_the_event() {
    let objects = MemoryPublicationObjects::default();
    write(&objects, ROOT, two_elections(), consistent_styles())
        .await
        .unwrap();

    let style = objects.json(&format!("{ROOT}/style-{STYLE}.json")).unwrap();
    assert_eq!(
        style,
        json!({
            "id": STYLE,
            "election_id": ELECTION,
            "area_id": AREA,
            "ballot_eml": null,
            "status": "PUBLISHED",
            "ballot_eml_prefix": r#"{"id":"style", "election_event_presentation": "#,
            "ballot_eml_suffix": r#","area_presentation":{"name":"North"},"election_dates":{"first_started_at":"2026-10-01T08:00:00Z"},"contests":[]}"#,
        })
    );
    let event = objects.json(&format!("{ROOT}/event.json")).unwrap();
    assert_eq!(
        event,
        json!({"id": EVENT, "presentation": {"name": "Event"}, "description": null, "ballot_eml_presentation": PRESENTATION})
    );
    let rebuilt = [
        &style["ballot_eml_prefix"],
        &event["ballot_eml_presentation"],
        &style["ballot_eml_suffix"],
    ]
    .map(|part| part.as_str().unwrap())
    .concat();
    assert_eq!(rebuilt, eml(PRESENTATION));
    assert_eq!(
        objects.json(&format!("{ROOT}/summary-{STYLE}.json")),
        Some(json!({
            "id": STYLE,
            "area_presentation": {"name": "North"},
            "election_dates": {"first_started_at": "2026-10-01T08:00:00Z"},
        }))
    );
    assert_eq!(
        objects.json(&format!("{ROOT}/election-{ELECTION}.json")),
        Some(election_object(ELECTION))
    );
}

#[tokio::test]
async fn equivalent_presentation_json_preserves_each_styles_exact_signed_eml() {
    let first = r#"{"i18n":{"en":{"name":"Event"},"es":{"name":"Evento"}}}"#;
    for equivalent in [
        r#"{"i18n":{"es":{"name":"Evento"},"en":{"name":"Event"}}}"#,
        r#"{ "i18n" : { "en" : { "name" : "Event" }, "es" : { "name" : "Evento" } } }"#,
    ] {
        let first_eml = eml(first);
        let second_eml = eml(equivalent);
        let mut first_row = style_row(STYLE, ELECTION, &first_eml);
        first_row["ballot_signature"] = json!("first-signature");
        let mut second_row = style_row(OTHER_STYLE, OTHER_ELECTION, &second_eml);
        second_row["ballot_signature"] = json!("second-signature");
        let objects = MemoryPublicationObjects::default();
        write(&objects, ROOT, two_elections(), vec![first_row, second_row])
            .await
            .unwrap();

        let event = objects.json(&event_key(ROOT)).unwrap();
        assert_eq!(event["ballot_eml_presentation"], first);
        let first_style = objects.json(&style_key(ROOT, STYLE)).unwrap();
        assert!(first_style["ballot_eml"].is_null());
        let rebuilt = [
            &first_style["ballot_eml_prefix"],
            &event["ballot_eml_presentation"],
            &first_style["ballot_eml_suffix"],
        ]
        .map(|part| part.as_str().unwrap())
        .concat();
        assert_eq!(rebuilt, first_eml);
        assert_eq!(first_style["ballot_signature"], "first-signature");

        let second_style = objects.json(&style_key(ROOT, OTHER_STYLE)).unwrap();
        assert_eq!(second_style["ballot_eml"], second_eml);
        assert!(second_style.get("ballot_eml_prefix").is_none());
        assert!(second_style.get("ballot_eml_suffix").is_none());
        assert_eq!(second_style["ballot_signature"], "second-signature");
    }
}

#[tokio::test]
async fn missing_and_present_event_presentations_remain_inconsistent() {
    for missing in [r#"{"contests":[]}"#.to_string(), eml("null")] {
        for (first, second) in [
            (missing.clone(), eml(PRESENTATION)),
            (eml(PRESENTATION), missing.clone()),
        ] {
            let objects = MemoryPublicationObjects::default();
            let error = write(
                &objects,
                ROOT,
                two_elections(),
                vec![
                    style_row(STYLE, ELECTION, &first),
                    style_row(OTHER_STYLE, OTHER_ELECTION, &second),
                ],
            )
            .await
            .unwrap_err();
            assert_eq!(
                error.to_string(),
                "Inconsistent event presentation within publication"
            );
            assert!(objects.json(&event_key(ROOT)).is_none());
        }
    }
}

#[tokio::test]
async fn a_style_without_an_event_presentation_keeps_its_whole_eml() {
    let eml = r#"{"contests":[]}"#;
    let objects = MemoryPublicationObjects::default();
    write(
        &objects,
        ROOT,
        two_elections(),
        vec![style_row(STYLE, ELECTION, eml)],
    )
    .await
    .unwrap();

    let style = objects.json(&format!("{ROOT}/style-{STYLE}.json")).unwrap();
    assert_eq!(style["ballot_eml_prefix"], eml);
    assert_eq!(style["ballot_eml_suffix"], "");
    assert_eq!(
        objects.json(&format!("{ROOT}/summary-{STYLE}.json")),
        Some(json!({"id": STYLE, "area_presentation": null, "election_dates": null}))
    );
    assert_eq!(
        objects.json(&format!("{ROOT}/event.json")).unwrap()["ballot_eml_presentation"],
        ""
    );
}

#[tokio::test]
async fn a_publication_without_styles_still_gets_its_event_object() {
    let objects = MemoryPublicationObjects::default();
    write(&objects, ROOT, two_elections(), Vec::new())
        .await
        .unwrap();

    let written = objects.keys();
    assert_eq!(sorted(written[..2].to_vec()), election_keys());
    assert_eq!(written[2..], [format!("{ROOT}/event.json")]);
    assert_eq!(
        objects.json(&format!("{ROOT}/event.json")).unwrap()["ballot_eml_presentation"],
        ""
    );
}

#[tokio::test]
async fn a_lost_lease_stops_the_upload_before_the_bucket_opens() {
    let (data, styles) = generated(event_object(), two_elections(), consistent_styles());
    // Opening this bucket fails, so the lease error shows it was never opened.
    let error = write_publication_objects(
        &styles.lease_lost(),
        &MemoryPublicationObjects::unavailable(),
        ROOT,
        data,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Ballot generation lease expired or was replaced"
    );
}

#[test]
fn only_a_text_root_inside_the_publication_makes_its_files_ready() {
    let publication = |annotations: Value| -> BallotPublication {
        serde_json::from_value(json!({
            "id": PUBLICATION, "tenant_id": TENANT, "election_event_id": EVENT,
            "annotations": annotations,
        }))
        .unwrap()
    };
    assert!(require_publication_files(&publication(json!({FILES_ANNOTATION: ROOT}))).is_ok());
    for (annotations, error) in [
        (
            json!({FILES_ANNOTATION: OTHER_ROOT}),
            "Publication object scope mismatch",
        ),
        (
            json!({FILES_ANNOTATION: format!("tenant-{EVENT}/event-{TENANT}/publication-{PUBLICATION}/{STYLE}")}),
            "Publication object scope mismatch",
        ),
        (
            json!({FILES_ANNOTATION: format!("tenant-{TENANT}/event-{EVENT}/publication-{PUBLICATION}/latest")}),
            "Invalid publication object version",
        ),
        (
            json!({FILES_ANNOTATION: format!("{ROOT}/../../publication-{OTHER_PUBLICATION}")}),
            "Invalid publication object version",
        ),
        (
            json!({FILES_ANNOTATION: {"root": ROOT}}),
            "Publication files are not ready; generate a new publication",
        ),
        (
            json!({"imported": true}),
            "Publication files are not ready; generate a new publication",
        ),
    ] {
        let result = require_publication_files(&publication(annotations.clone()));
        assert_eq!(result.unwrap_err().to_string(), error, "{annotations}");
    }
}

#[tokio::test]
async fn a_retry_after_a_failed_attempt_writes_under_a_new_root() {
    let objects = MemoryPublicationObjects::failing_at(format!("{ROOT}/event.json"));
    let error = write(&objects, ROOT, two_elections(), consistent_styles())
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "upload rejected");
    let failed_attempt = objects.keys();
    assert_eq!(failed_attempt.len(), 6);

    write(&objects, RETRY_ROOT, two_elections(), consistent_styles())
        .await
        .unwrap();
    let keys = objects.keys();
    assert_eq!(keys[..6], failed_attempt[..]);
    assert_eq!(keys.len(), 13);
    assert!(keys[6..]
        .iter()
        .all(|key| key.starts_with(&format!("{RETRY_ROOT}/"))));
}

#[tokio::test]
async fn different_event_presentations_in_one_publication_fail_before_their_styles() {
    let other = r#"{"language_conf":{"enabled_language_codes":["en"]}}"#;
    let objects = MemoryPublicationObjects::default();
    let error = write(
        &objects,
        ROOT,
        two_elections(),
        vec![
            style_row(STYLE, ELECTION, &eml(PRESENTATION)),
            style_row(OTHER_STYLE, OTHER_ELECTION, &eml(other)),
        ],
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Inconsistent event presentation within publication"
    );
    assert_eq!(sorted(objects.keys()), election_keys());
}

#[tokio::test]
async fn rows_without_their_identifiers_or_eml_fail_before_uploading() {
    for (elections, styles, error) in [
        (
            vec![json!({"presentation": null})],
            vec![],
            "Missing election id",
        ),
        (vec![], vec![json!({"id": STYLE})], "Missing ballot EML"),
        (
            vec![],
            vec![json!({"id": STYLE, "ballot_eml": {"contests": []}})],
            "Missing ballot EML",
        ),
    ] {
        let objects = MemoryPublicationObjects::default();
        let result = write(&objects, ROOT, elections, styles).await;

        assert_eq!(result.unwrap_err().to_string(), error);
        assert!(objects.keys().is_empty(), "{error}");
    }
}

#[tokio::test]
async fn a_style_with_malformed_eml_fails_after_the_election_objects() {
    for malformed in ["invalid", r#""contests""#] {
        let objects = MemoryPublicationObjects::default();
        let error = write(
            &objects,
            ROOT,
            two_elections(),
            vec![style_row(STYLE, ELECTION, malformed)],
        )
        .await
        .unwrap_err();

        assert!(error.is::<serde_json::Error>(), "{malformed}: {error}");
        assert_eq!(sorted(objects.keys()), election_keys(), "{malformed}");
    }
}

#[tokio::test]
async fn a_failure_while_reading_styles_writes_no_event() {
    let (data, styles) = generated(event_object(), two_elections(), consistent_styles());
    let objects = MemoryPublicationObjects::default();
    let error = write_publication_objects(&styles.failing_after(2), &objects, ROOT, data)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "style page read failed");
    assert_eq!(objects.keys().len(), 6);
    assert!(objects.json(&event_key(ROOT)).is_none());
}

#[tokio::test]
async fn styles_removed_or_added_during_the_upload_fail_it() {
    let mut removed = consistent_styles();
    removed[1]["deleted_at"] = json!("2026-10-01T08:00:00Z");
    let objects = MemoryPublicationObjects::default();
    let error = write(&objects, ROOT, two_elections(), removed)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Publication was removed during upload");

    let (mut data, styles) = generated(event_object(), two_elections(), consistent_styles());
    data.style_count = 1;
    let objects = MemoryPublicationObjects::default();
    let error = write_publication_objects(&styles, &objects, ROOT, data)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Ballot styles changed during publication upload"
    );
    assert!(objects.json(&event_key(ROOT)).is_none());
}

#[tokio::test]
async fn an_unavailable_bucket_fails_the_upload() {
    let (data, styles) = generated(event_object(), two_elections(), consistent_styles());
    let error = write_publication_objects(
        &styles,
        &MemoryPublicationObjects::unavailable(),
        ROOT,
        data,
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "private bucket unavailable");
}

#[tokio::test]
async fn malformed_publication_identifiers_are_rejected_before_any_upload() {
    let pool = Pool::builder(Manager::new(
        tokio_postgres::Config::new(),
        tokio_postgres::NoTls,
    ))
    .build()
    .unwrap();
    let (tenant, event, id) = (
        TENANT.to_string(),
        EVENT.to_string(),
        PUBLICATION.to_string(),
    );
    let lease = |value: &str| PgLock {
        key: "create_ballot_style".to_owned(),
        value: value.to_owned(),
        expiry_date: None,
    };
    let attempt = Uuid::new_v4().to_string();
    for (tenant, event, id, lease) in [
        ("tenant", event.as_str(), id.as_str(), lease(&attempt)),
        (tenant.as_str(), "event", id.as_str(), lease(&attempt)),
        (
            tenant.as_str(),
            event.as_str(),
            "publication",
            lease(&attempt),
        ),
        (
            tenant.as_str(),
            event.as_str(),
            id.as_str(),
            lease("attempt"),
        ),
    ] {
        let (data, _) = generated(event_object(), two_elections(), consistent_styles());
        let error = prepare_publication_files(&pool, tenant, event, id, &lease, data)
            .await
            .unwrap_err();
        assert!(error.is::<uuid::Error>(), "{error}");
    }
}

fn published(
    id: Uuid,
    election: Uuid,
    publication: Uuid,
    root: Option<&str>,
) -> PublishedBallotStyle {
    PublishedBallotStyle {
        id,
        election_id: election,
        publication_id: publication,
        root: root.map(str::to_owned),
        status: Some(json!({"voting_status": "OPEN"})),
        num_allowed_revotes: Some(2),
        voting_channels: Some(json!({"online": true, "kiosk": false})),
    }
}

fn voter_rows(styles: Vec<PublishedBallotStyle>) -> MemoryPublicationRows {
    MemoryPublicationRows::default()
        .with_event(
            TENANT,
            EVENT,
            event_object(),
            Some(json!({"is_published": true})),
        )
        .with_published_styles(TENANT, EVENT, AREA, styles)
}

async fn voter_files(
    rows: &MemoryPublicationRows,
    objects: &MemoryPublicationObjects,
    elections: &[Uuid],
) -> Result<Value> {
    let elections: Vec<String> = elections.iter().map(Uuid::to_string).collect();
    let references = load_references(
        rows,
        &TENANT.to_string(),
        &EVENT.to_string(),
        &AREA.to_string(),
        &elections,
    )
    .await?;
    presign_references(objects, references).await
}

/// The object and lifetime a fake presigned URL grants, without its signature.
fn grant(url: &Value) -> &str {
    let url = url.as_str().unwrap();
    &url[..url.find("&signature=").unwrap()]
}

#[tokio::test]
async fn voters_get_their_styles_with_the_live_election_policy_and_no_eml() {
    let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
    let mut response = voter_files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
        .await
        .unwrap();

    let urls = response["files"][0].as_object_mut().unwrap().remove("urls");
    assert!(urls.is_some());
    assert_eq!(
        response,
        json!({
            "event_id": EVENT,
            "status": {"is_published": true},
            "files": [{
                "id": STYLE,
                "election_id": ELECTION,
                "version": ROOT,
                "status": {"voting_status": "OPEN"},
                "num_allowed_revotes": 2,
                "voting_channels": {"online": true, "kiosk": false},
            }],
        })
    );
}

#[tokio::test]
async fn each_file_links_its_event_election_summary_and_style_for_five_minutes() {
    let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
    let response = voter_files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
        .await
        .unwrap();

    let urls = &response["files"][0]["urls"];
    assert_eq!(urls.as_object().unwrap().len(), 4);
    for (name, object) in [
        ("event_url", "event.json".to_owned()),
        ("election_url", format!("election-{ELECTION}.json")),
        ("summary_url", format!("summary-{STYLE}.json")),
        ("style_url", format!("style-{STYLE}.json")),
    ] {
        assert_eq!(
            grant(&urls[name]),
            format!("https://bucket.test/{ROOT}/{object}?expires=300"),
            "{name}"
        );
    }
}

#[tokio::test]
async fn an_object_shared_by_several_files_is_signed_once() {
    let rows = voter_rows(vec![
        published(STYLE, ELECTION, PUBLICATION, Some(ROOT)),
        published(OTHER_STYLE, OTHER_ELECTION, PUBLICATION, Some(ROOT)),
    ]);
    let response = voter_files(
        &rows,
        &MemoryPublicationObjects::default(),
        &[ELECTION, OTHER_ELECTION],
    )
    .await
    .unwrap();

    let (first, second) = (&response["files"][0]["urls"], &response["files"][1]["urls"]);
    // Every signature is new, so an identical URL was signed once.
    assert_eq!(first["event_url"], second["event_url"]);
    for name in ["election_url", "summary_url", "style_url"] {
        assert_ne!(grant(&first[name]), grant(&second[name]), "{name}");
    }
}

#[tokio::test]
async fn files_keep_the_newest_publication_first_order_of_the_query() {
    let rows = voter_rows(vec![
        published(
            OTHER_STYLE,
            OTHER_ELECTION,
            OTHER_PUBLICATION,
            Some(OTHER_ROOT),
        ),
        published(STYLE, ELECTION, PUBLICATION, Some(ROOT)),
    ]);
    let response = voter_files(
        &rows,
        &MemoryPublicationObjects::default(),
        &[ELECTION, OTHER_ELECTION],
    )
    .await
    .unwrap();

    let versions: Vec<&str> = response["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, [OTHER_ROOT, ROOT]);
}

#[tokio::test]
async fn only_the_requested_elections_are_served() {
    let rows = voter_rows(vec![
        published(STYLE, ELECTION, PUBLICATION, Some(ROOT)),
        published(OTHER_STYLE, OTHER_ELECTION, PUBLICATION, Some(ROOT)),
    ]);
    let objects = MemoryPublicationObjects::default();

    let response = voter_files(&rows, &objects, &[OTHER_ELECTION])
        .await
        .unwrap();
    assert_eq!(response["files"].as_array().unwrap().len(), 1);
    assert_eq!(response["files"][0]["id"], json!(OTHER_STYLE));
    assert_eq!(
        voter_files(&rows, &objects, &[]).await.unwrap(),
        json!({"event_id": EVENT, "status": {"is_published": true}, "files": []})
    );
}

#[tokio::test]
async fn no_files_are_signed_without_opening_the_bucket() {
    let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
    assert_eq!(
        voter_files(
            &rows,
            &MemoryPublicationObjects::unavailable(),
            &[OTHER_ELECTION]
        )
        .await
        .unwrap()["files"],
        json!([])
    );
}

#[tokio::test]
async fn a_second_active_style_for_one_election_is_rejected() {
    let rows = voter_rows(vec![
        published(STYLE, ELECTION, PUBLICATION, Some(ROOT)),
        published(OTHER_STYLE, ELECTION, OTHER_PUBLICATION, Some(OTHER_ROOT)),
    ]);
    let error = voter_files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Multiple active ballot styles for one election"
    );
}

#[tokio::test]
async fn a_publication_without_prepared_objects_cannot_be_served() {
    let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, None)]);
    let error = voter_files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Publication files are not prepared");
}

#[tokio::test]
async fn a_root_outside_the_style_publication_is_rejected() {
    for (root, error) in [
        (OTHER_ROOT.to_owned(), "Publication object scope mismatch"),
        (
            format!("tenant-{TENANT}/event-{EVENT}/publication-{PUBLICATION}/latest"),
            "Invalid publication object version",
        ),
    ] {
        let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(&root))]);
        let result = voter_files(&rows, &MemoryPublicationObjects::default(), &[ELECTION]).await;
        assert_eq!(result.unwrap_err().to_string(), error, "{root}");
    }
}

#[tokio::test]
async fn malformed_voter_identifiers_are_rejected() {
    let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
    let (tenant, event, area, election) = (
        TENANT.to_string(),
        EVENT.to_string(),
        AREA.to_string(),
        ELECTION.to_string(),
    );
    for (tenant, event, area, elections) in [
        (
            "tenant",
            event.as_str(),
            area.as_str(),
            vec![election.clone()],
        ),
        (
            tenant.as_str(),
            "event",
            area.as_str(),
            vec![election.clone()],
        ),
        (
            tenant.as_str(),
            event.as_str(),
            "area",
            vec![election.clone()],
        ),
        (
            tenant.as_str(),
            event.as_str(),
            area.as_str(),
            vec![election.clone(), "election".to_owned()],
        ),
    ] {
        let result = load_references(&rows, tenant, event, area, &elections).await;
        let error = result.err().expect("an identifier error");
        assert!(error.is::<uuid::Error>(), "{error}");
    }
}

#[tokio::test]
async fn an_unknown_event_is_rejected_before_opening_the_bucket() {
    let rows = MemoryPublicationRows::default().with_published_styles(
        TENANT,
        EVENT,
        AREA,
        vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))],
    );
    let error = voter_files(&rows, &MemoryPublicationObjects::unavailable(), &[ELECTION])
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Election event not found");
}

#[tokio::test]
async fn a_url_that_cannot_be_signed_fails_the_whole_response() {
    let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
    let error = voter_files(
        &rows,
        &MemoryPublicationObjects::failing_presign(),
        &[ELECTION],
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "presigning failed");
}

const EDITED_PRESENTATION: &str = r#"{"css":".edited {}"}"#;

fn edited_election() -> Value {
    json!({"id": ELECTION, "presentation": {"name": "Edited election"}})
}

/// The rows after an administrator edited an election, with the files
/// annotation of a publication.
fn edited(publication: Uuid, annotations: Option<Value>) -> MemoryPublicationRows {
    MemoryPublicationRows::default().with_publication(
        TENANT,
        EVENT,
        publication,
        PublicationRecord {
            annotations,
            elections: vec![edited_election()],
            ..PublicationRecord::default()
        },
    )
}

fn publication(record: PublicationRecord) -> MemoryPublicationRows {
    MemoryPublicationRows::default().with_publication(TENANT, EVENT, PUBLICATION, record)
}

fn files_annotation(root: &str) -> Option<Value> {
    Some(json!({ FILES_ANNOTATION: root }))
}

fn stored(objects: &MemoryPublicationObjects) -> Vec<(String, Value)> {
    objects
        .keys()
        .into_iter()
        .map(|key| {
            let value = objects.json(&key).unwrap();
            (key, value)
        })
        .collect()
}

async fn snapshot(
    rows: &MemoryPublicationRows,
    objects: &MemoryPublicationObjects,
) -> Result<Option<PublicationSnapshot>> {
    read_snapshot(
        rows,
        objects,
        &TENANT.to_string(),
        &EVENT.to_string(),
        &PUBLICATION.to_string(),
    )
    .await
}

#[tokio::test]
async fn edits_after_generating_reach_voters_only_through_a_new_publication() {
    let objects = MemoryPublicationObjects::default();
    write(&objects, ROOT, two_elections(), consistent_styles())
        .await
        .unwrap();
    let published_objects = stored(&objects);

    let serving = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
    let served = voter_files(&serving, &objects, &[ELECTION]).await.unwrap();
    assert_eq!(served["files"][0]["version"], ROOT);
    assert!(grant(&served["files"][0]["urls"]["event_url"])
        .starts_with(&format!("https://bucket.test/{ROOT}/event.json?")));

    // A new publication of the edited rows is written apart, with the edits.
    let (data, styles) = generated(
        json!({"id": EVENT, "presentation": {"name": "Edited event"}, "description": null}),
        vec![edited_election()],
        vec![style_row(STYLE, ELECTION, &eml(EDITED_PRESENTATION))],
    );
    write_publication_objects(&styles, &objects, OTHER_ROOT, data)
        .await
        .unwrap();
    assert_eq!(
        objects.json(&format!("{OTHER_ROOT}/event.json")).unwrap(),
        json!({
            "id": EVENT,
            "presentation": {"name": "Edited event"},
            "description": null,
            "ballot_eml_presentation": EDITED_PRESENTATION,
        })
    );
    assert_eq!(
        objects
            .json(&format!("{OTHER_ROOT}/election-{ELECTION}.json"))
            .unwrap()["presentation"],
        json!({"name": "Edited election"})
    );
    assert_eq!(
        stored(&objects)[..published_objects.len()],
        published_objects
    );

    let serving = voter_rows(vec![published(
        STYLE,
        ELECTION,
        OTHER_PUBLICATION,
        Some(OTHER_ROOT),
    )]);
    let served = voter_files(&serving, &objects, &[ELECTION]).await.unwrap();
    assert_eq!(served["files"][0]["version"], OTHER_ROOT);
}

#[tokio::test]
async fn a_snapshot_is_the_event_and_elections_as_written_not_as_they_are_now() {
    let objects = MemoryPublicationObjects::default();
    write(&objects, ROOT, two_elections(), consistent_styles())
        .await
        .unwrap();

    let after_edit = edited(PUBLICATION, files_annotation(ROOT));
    assert_eq!(
        snapshot(&after_edit, &objects).await.unwrap(),
        Some(PublicationSnapshot {
            event: json!({
                "id": EVENT,
                "presentation": {"name": "Event"},
                "description": null,
                "ballot_eml_presentation": PRESENTATION,
            }),
            elections: vec![election_object(ELECTION)],
        })
    );
}

#[tokio::test]
async fn a_publication_without_objects_has_no_snapshot() {
    let rows = publication(PublicationRecord {
        elections: two_elections(),
        ..PublicationRecord::default()
    });
    // Opening this bucket fails, so success shows it was never opened.
    let objects = MemoryPublicationObjects::unavailable();
    assert_eq!(snapshot(&rows, &objects).await.unwrap(), None);
}

#[tokio::test]
async fn a_snapshot_that_cannot_be_read_whole_is_an_error() {
    let outside = publication(PublicationRecord {
        annotations: files_annotation(OTHER_ROOT),
        elections: two_elections(),
        ..PublicationRecord::default()
    });
    let error = snapshot(&outside, &MemoryPublicationObjects::default())
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Publication object scope mismatch");

    let prepared = publication(PublicationRecord {
        annotations: files_annotation(ROOT),
        elections: two_elections(),
        ..PublicationRecord::default()
    });
    let error = snapshot(&prepared, &MemoryPublicationObjects::default())
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "no such object");

    let error = snapshot(&prepared, &MemoryPublicationObjects::unavailable())
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "private bucket unavailable");

    let unknown = MemoryPublicationRows::default();
    assert!(snapshot(&unknown, &MemoryPublicationObjects::default())
        .await
        .is_err());
    assert!(read_snapshot(
        &prepared,
        &MemoryPublicationObjects::default(),
        "not-a-uuid",
        &EVENT.to_string(),
        &PUBLICATION.to_string(),
    )
    .await
    .is_err());
}
