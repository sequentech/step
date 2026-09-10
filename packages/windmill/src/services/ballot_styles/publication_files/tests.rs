// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use deadpool_postgres::Manager;

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
            CREATE TABLE sequent_backend.ballot_publication(id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb, election_ids uuid[], election_id uuid, is_generated bool DEFAULT false, published_at timestamptz, deleted_at timestamptz, PRIMARY KEY (id,tenant_id,election_event_id));
            CREATE TABLE sequent_backend.ballot_style(id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, election_id uuid, area_id uuid, created_at timestamptz, last_updated_at timestamptz, annotations jsonb, labels jsonb, ballot_eml text, ballot_signature bytea, status text, deleted_at timestamptz, ballot_publication_id uuid);
            CREATE TABLE sequent_backend.lock(key text PRIMARY KEY,value text,expiry_date timestamptz);
            CREATE TABLE sequent_backend.document(id uuid PRIMARY KEY,tenant_id uuid,election_event_id uuid,name text,media_type text,size bigint,is_public bool,annotations jsonb,labels jsonb,created_at timestamptz,last_updated_at timestamptz);").await?;
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
    let eml = r#"{"contests": [], "election_event_presentation": {"title":"escaped \" text","nested":{"z":2,"a":1}}, "area_presentation":{}}"#;
    let other_eml = r#"{"contests": [], "election_event_presentation": {"nested":{"a":1,"z":2}, "title":"escaped \" text"}, "area_presentation":{}}"#;
    let client = get_shared_s3_client(S3Endpoint::Server).await?;
    let bucket = get_private_bucket()?;
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
        upload_batch(&client,&bucket,padding.collect()).await?;
        upload(&client,&bucket,&format!("{prefix}document-{document}/export.json"),&json!({"document":true})).await?;
        let archive = sequent_core::services::s3::get_files_from_s3(bucket.clone(),prefix,&HashSet::from([document.clone()])).await?;
        assert_eq!(archive.len(),1);
        assert!(archive[0].file_name().unwrap().to_str().unwrap().contains(&format!("document_{document}_export.json")));
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&archive[0])?)?,json!({"document":true}));
        assert_eq!(files(&pool,&t,&ev,&a,&ids).await?["files"], json!([]));
        pool.get().await?.execute("UPDATE sequent_backend.ballot_publication SET published_at=now() WHERE id=$1", &[&publication]).await?;
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
        S3Endpoint::Server,
    )
    .await;
    let public_cleanup = sequent_core::services::s3::delete_files_from_s3(
        get_public_bucket()?,
        format!("tenant-{tenant}/event-{event}/"),
        S3Endpoint::Server,
    )
    .await;
    result?;
    cleanup?;
    public_cleanup?;
    Ok(())
}
