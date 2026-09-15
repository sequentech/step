// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use aws_sdk_s3::{presigning::PresigningConfig, primitives::ByteStream, Client};
use deadpool_postgres::Transaction;
use futures::TryStreamExt;
use sequent_core::services::s3::{get_private_bucket, get_s3_client};
use sequent_core::util::aws::get_s3_aws_config;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use uuid::Uuid;

pub const FILES_ANNOTATION: &str = "ballot_files_v1";

fn validate_publication_root(
    root: &str,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<()> {
    let prefix = format!("tenant-{tenant}/event-{event}/publication-{publication}/");
    let attempt = root
        .strip_prefix(&prefix)
        .context("Publication object scope mismatch")?;
    Uuid::parse_str(attempt).context("Invalid publication object version")?;
    Ok(())
}

/// Split only the shared JSON value, preserving the exact original EML bytes.
fn split_event_presentation(eml: &str) -> Result<(String, String, String)> {
    #[derive(serde::Deserialize)]
    struct Envelope<'a> {
        #[serde(borrow)]
        election_event_presentation: Option<&'a serde_json::value::RawValue>,
    }
    let parsed: Envelope<'_> = serde_json::from_str(eml)?;
    let Some(raw) = parsed.election_event_presentation else {
        return Ok((eml.to_owned(), String::new(), String::new()));
    };
    let value = raw.get();
    let offset = value.as_ptr() as usize - eml.as_ptr() as usize;
    Ok((
        eml[..offset].to_owned(),
        value.to_owned(),
        eml[offset + value.len()..].to_owned(),
    ))
}

async fn upload(client: &Client, bucket: &str, key: &str, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type("application/json")
        .cache_control("private, max-age=300")
        .if_none_match("*")
        .body(ByteStream::from(bytes.clone()))
        .send()
        .await
        .context("Cannot upload ballot publication object")?;
    let stored = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("Cannot validate ballot publication object")?
        .body
        .collect()
        .await?
        .into_bytes();
    if stored.as_ref() != bytes.as_slice() {
        bail!("Ballot publication object verification failed");
    }
    Ok(())
}

/// Called under the event publication lock. The reference becomes visible only
/// when the transaction commits, after every immutable object has been verified.
pub async fn prepare_publication_files(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<()> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let publication = Uuid::parse_str(publication)?;
    let row = tx.query_one("SELECT annotations FROM sequent_backend.ballot_publication WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3", &[&tenant, &event, &publication]).await?;
    let annotations: Option<Value> = row.get(0);
    if let Some(root) = annotations
        .as_ref()
        .and_then(|v| v.get(FILES_ANNOTATION))
        .and_then(Value::as_str)
    {
        validate_publication_root(root, tenant, event, publication)?;
        return Ok(());
    }
    // A new attempt never overwrites objects from a failed or active attempt.
    let root = format!(
        "tenant-{tenant}/event-{event}/publication-{publication}/{}",
        Uuid::new_v4()
    );
    let bucket = get_private_bucket()?;
    let client = get_s3_client(get_s3_aws_config(true).await?).await?;
    let mut event_data: Value = tx.query_one("SELECT jsonb_build_object('id', id, 'presentation', presentation, 'description', description) FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2", &[&tenant,&event]).await?.get(0);

    let elections = tx
        .query(
            r#"
            SELECT jsonb_build_object('id', id,
                'tenant_id', tenant_id,
                'election_event_id', election_event_id,
                'annotations', annotations,
                'created_at', created_at,
                'description', description,
                'is_consolidated_ballot_encoding', is_consolidated_ballot_encoding,
                'labels', labels,
                'last_updated_at', last_updated_at,
                'presentation', presentation,
                'spoil_ballot_option', spoil_ballot_option)
            FROM sequent_backend.election
            WHERE tenant_id=$1
              AND election_event_id=$2
              AND id = ANY(SELECT unnest(election_ids)
            FROM sequent_backend.ballot_publication
            WHERE id=$3
              AND tenant_id=$1)
        "#,
            &[&tenant, &event, &publication],
        )
        .await?;
    for row in elections {
        let data: Value = row.get(0);
        let id = data["id"].as_str().context("Missing election id")?;
        upload(
            &client,
            &bucket,
            &format!("{root}/election-{id}.json"),
            &data,
        )
        .await?;
    }
    // Stream full styles: publication memory does not grow with the number of areas.
    let rows = tx
        .query_raw(
            r#"
            SELECT jsonb_build_object('id', id,
                'tenant_id', tenant_id,
                'election_event_id', election_event_id,
                'election_id', election_id,
                'area_id', area_id,
                'created_at', created_at,
                'last_updated_at', last_updated_at,
                'annotations', annotations,
                'labels', labels,
                'ballot_eml', ballot_eml,
                'ballot_signature', ballot_signature,
                'status', status,
                'deleted_at', deleted_at)
            FROM sequent_backend.ballot_style
            WHERE tenant_id=$1
              AND election_event_id=$2
              AND ballot_publication_id=$3
        "#,
            [&tenant, &event, &publication],
        )
        .await?;
    futures::pin_mut!(rows);
    let mut shared_presentation: Option<String> = None;
    while let Some(row) = rows.try_next().await? {
        let mut data: Value = row.get(0);
        let id = data["id"].as_str().context("Missing style id")?.to_owned();
        let eml: Value =
            serde_json::from_str(data["ballot_eml"].as_str().context("Missing ballot EML")?)?;
        let (prefix, presentation, suffix) =
            split_event_presentation(data["ballot_eml"].as_str().context("Missing ballot EML")?)?;
        if let Some(existing) = shared_presentation.as_ref() {
            if existing != &presentation {
                bail!("Inconsistent event presentation within publication");
            }
        } else {
            shared_presentation = Some(presentation);
        }
        data["ballot_eml"] = Value::Null;
        data["ballot_eml_prefix"] = Value::String(prefix);
        data["ballot_eml_suffix"] = Value::String(suffix);
        // List information is small; contests/candidates are fetched only on selection.
        let summary = json!({"id":id, "area_presentation":eml.get("area_presentation"), "election_dates":eml.get("election_dates")});
        upload(
            &client,
            &bucket,
            &format!("{root}/summary-{id}.json"),
            &summary,
        )
        .await?;
        upload(&client, &bucket, &format!("{root}/style-{id}.json"), &data).await?;
    }
    event_data["ballot_eml_presentation"] = Value::String(shared_presentation.unwrap_or_default());
    upload(&client, &bucket, &format!("{root}/event.json"), &event_data).await?;
    tx.execute(r#"
            UPDATE sequent_backend.ballot_publication
            SET annotations=COALESCE(annotations,'{}'::jsonb) || jsonb_build_object($4::text,$5::text)
            WHERE tenant_id=$1
              AND election_event_id=$2
              AND id=$3
        "#, &[&tenant,&event,&publication,&FILES_ANNOTATION,&root]).await?;
    Ok(())
}

/// Only identifiers, active references and live policy are read here, never EML.
pub async fn voter_files(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
) -> Result<Value> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let area = Uuid::parse_str(area)?;
    let elections = elections
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<Vec<_>, _>>()?;
    let rows=tx.query(r#"
            SELECT s.id, s.election_id, p.id AS publication_id, p.annotations->>$5::text AS root, e.status, e.num_allowed_revotes::bigint AS num_allowed_revotes, e.voting_channels
            FROM sequent_backend.ballot_style s
            JOIN sequent_backend.ballot_publication p ON p.id=s.ballot_publication_id
              AND p.tenant_id=s.tenant_id
              AND p.election_event_id=s.election_event_id
            JOIN sequent_backend.election e ON e.id=s.election_id
              AND e.tenant_id=s.tenant_id
              AND e.election_event_id=s.election_event_id
            WHERE s.tenant_id=$1
              AND s.election_event_id=$2
              AND s.area_id=$3
              AND s.election_id=ANY($4)
              AND s.deleted_at IS NULL
              AND p.published_at IS NOT NULL
              AND p.deleted_at IS NULL
            ORDER BY p.published_at DESC, s.election_id
        "#, &[&tenant,&event,&area,&elections,&FILES_ANNOTATION]).await?;
    let event_row = tx
        .query_opt(
            "SELECT status FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
            &[&tenant, &event],
        )
        .await?
        .context("Election event not found")?;
    let status: Option<Value> = event_row.get(0);
    let bucket = get_private_bucket()?;
    let client = get_s3_client(get_s3_aws_config(false).await?).await?;
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let mut signed_urls = HashMap::<String, String>::new();
    for row in rows {
        let id: Uuid = row.get("id");
        let election: Uuid = row.get("election_id");
        if !seen.insert(election) {
            bail!("Multiple active ballot styles for one election");
        }
        let root: Option<String> = row.get("root");
        let root = root.context(
            "Publication requires S3 preparation; publish it again before serving voters",
        )?;
        validate_publication_root(&root, tenant, event, row.get("publication_id"))?;
        let mut urls = serde_json::Map::new();
        for (name, key) in [
            ("event_url", format!("{root}/event.json")),
            ("election_url", format!("{root}/election-{election}.json")),
            ("summary_url", format!("{root}/summary-{id}.json")),
            ("style_url", format!("{root}/style-{id}.json")),
        ] {
            let url = if let Some(url) = signed_urls.get(&key) {
                url.clone()
            } else {
                let signed = client
                    .get_object()
                    .bucket(&bucket)
                    .key(&key)
                    .presigned(PresigningConfig::expires_in(Duration::from_secs(300))?)
                    .await?;
                let url = signed.uri().to_string();
                signed_urls.insert(key, url.clone());
                url
            };
            urls.insert(name.into(), Value::String(url));
        }
        files.push(json!({"id":id,"election_id":election,"version":root,"urls":urls,"status":row.get::<_,Option<Value>>("status"),"num_allowed_revotes":row.try_get::<_,Option<i64>>("num_allowed_revotes")?,"voting_channels":row.get::<_,Option<Value>>("voting_channels")}));
    }
    Ok(json!({"event_id":event,"status":status,"files":files}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use deadpool_postgres::{Manager, Pool};

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
        assert!(validate_publication_root(
            &format!("{root}/../../other"),
            tenant,
            event,
            publication
        )
        .is_err());
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

    #[tokio::test]
    #[ignore = "requires disposable PostgreSQL and local private S3; use scripts/test_ballot_files.py"]
    async fn publication_objects_and_authorized_references() -> Result<()> {
        let config = std::env::var("BALLOT_FILES_TEST_DSN")?.parse::<tokio_postgres::Config>()?;
        let pool = Pool::builder(Manager::new(config, tokio_postgres::NoTls))
            .max_size(2)
            .build()?;
        let mut db = pool.get().await?;
        db.batch_execute("DROP SCHEMA sequent_backend CASCADE; CREATE SCHEMA sequent_backend;
            CREATE TABLE sequent_backend.election_event(id uuid, tenant_id uuid, presentation jsonb, description text, status jsonb);
            CREATE TABLE sequent_backend.election(id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb, created_at timestamptz, description text, is_consolidated_ballot_encoding bool, labels jsonb, last_updated_at timestamptz, presentation jsonb, spoil_ballot_option bool, num_allowed_revotes integer, voting_channels jsonb, status jsonb);
            CREATE TABLE sequent_backend.ballot_publication(id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb, election_ids uuid[], published_at timestamptz, deleted_at timestamptz);
            CREATE TABLE sequent_backend.ballot_style(id uuid, tenant_id uuid, election_event_id uuid, election_id uuid, area_id uuid, created_at timestamptz, last_updated_at timestamptz, annotations jsonb, labels jsonb, ballot_eml text, ballot_signature bytea, status text, deleted_at timestamptz, ballot_publication_id uuid);").await?;
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        let election = Uuid::new_v4();
        let area = Uuid::new_v4();
        let publication = Uuid::new_v4();
        let style = Uuid::new_v4();
        db.execute(
            "INSERT INTO sequent_backend.election_event VALUES ($1,$2,'{}',NULL,'{}')",
            &[&event, &tenant],
        )
        .await?;
        db.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,presentation,status,num_allowed_revotes) VALUES ($1,$2,$3,'{}','{}',1)", &[&election,&tenant,&event]).await?;
        db.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids,published_at) VALUES ($1,$2,$3,$4,NOW())", &[&publication,&tenant,&event,&vec![election]]).await?;
        db.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES ($1,$2,$3,$4,$5,$6,'{\"contests\": [], \"area_presentation\":{}}')", &[&style,&tenant,&event,&election,&area,&publication]).await?;
        let other_election = Uuid::new_v4();
        let other_style = Uuid::new_v4();
        db.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,presentation,status,num_allowed_revotes) VALUES ($1,$2,$3,'{}','{}',1)", &[&other_election,&tenant,&event]).await?;
        db.execute("UPDATE sequent_backend.ballot_publication SET election_ids=array_append(election_ids,$1)", &[&other_election]).await?;
        db.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) SELECT $1,tenant_id,election_event_id,$2,area_id,ballot_publication_id,ballot_eml FROM sequent_backend.ballot_style WHERE id=$3", &[&other_style,&other_election,&style]).await?;
        let t = tenant.to_string();
        let ev = event.to_string();
        let p = publication.to_string();
        let a = area.to_string();
        let ids = vec![election.to_string()];
        let tx = db.transaction().await?;
        assert!(voter_files(&tx, &t, &ev, &a, &ids).await.is_err());
        prepare_publication_files(&tx, &t, &ev, &p).await?;
        let first: Value = tx
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        prepare_publication_files(&tx, &t, &ev, &p).await?;
        let second: Value = tx
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        assert_eq!(first, second);
        let files = voter_files(&tx, &t, &ev, &a, &ids).await?;
        assert_eq!(files["files"].as_array().unwrap().len(), 1);
        assert!(!files.to_string().contains("ballot_eml"));
        for denied in [vec![], vec![Uuid::new_v4().to_string()]] {
            assert_eq!(
                voter_files(&tx, &t, &ev, &a, &denied).await?["files"],
                json!([])
            );
        }
        assert_eq!(
            voter_files(&tx, &t, &ev, &Uuid::new_v4().to_string(), &ids).await?["files"],
            json!([])
        );
        assert!(voter_files(&tx, &Uuid::new_v4().to_string(), &ev, &a, &ids)
            .await
            .is_err());
        assert!(voter_files(&tx, &t, &Uuid::new_v4().to_string(), &a, &ids)
            .await
            .is_err());
        // Publishing one election replaces only that election's immutable reference.
        let partial = Uuid::new_v4();
        let replacement_style = Uuid::new_v4();
        tx.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids) VALUES ($1,$2,$3,$4)", &[&partial,&tenant,&event,&vec![election]]).await?;
        tx.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) SELECT $1,tenant_id,election_event_id,election_id,area_id,$2,ballot_eml FROM sequent_backend.ballot_style WHERE id=$3", &[&replacement_style,&partial,&style]).await?;
        prepare_publication_files(&tx, &t, &ev, &partial.to_string()).await?;
        assert_eq!(
            voter_files(&tx, &t, &ev, &a, &ids).await?["files"][0]["id"],
            json!(style)
        );
        tx.execute(
            "UPDATE sequent_backend.ballot_style SET deleted_at=NOW() WHERE id=$1",
            &[&style],
        )
        .await?;
        tx.execute(
            "UPDATE sequent_backend.ballot_publication SET published_at=NOW() WHERE id=$1",
            &[&partial],
        )
        .await?;
        let both = voter_files(
            &tx,
            &t,
            &ev,
            &a,
            &vec![election.to_string(), other_election.to_string()],
        )
        .await?;
        let both = both["files"].as_array().unwrap();
        assert_eq!(both.len(), 2);
        assert!(both
            .iter()
            .any(|file| file["id"] == json!(replacement_style)));
        assert!(both.iter().any(|file| file["id"] == json!(other_style)));
        assert_ne!(both[0]["version"], both[1]["version"]);
        // Live status changes do not rewrite or invalidate immutable content.
        tx.execute(
            "UPDATE sequent_backend.election SET status='{\"voting_status\":\"CLOSED\"}'",
            &[],
        )
        .await?;
        assert_eq!(
            voter_files(&tx, &t, &ev, &a, &ids).await?["files"][0]["status"]["voting_status"],
            "CLOSED"
        );
        // Staged or deleted publications do not grant object URLs.
        tx.execute(
            "UPDATE sequent_backend.ballot_publication SET published_at=NULL",
            &[],
        )
        .await?;
        assert_eq!(
            voter_files(&tx, &t, &ev, &a, &ids).await?["files"],
            json!([])
        );
        tx.execute(
            "UPDATE sequent_backend.ballot_publication SET published_at=NOW(), deleted_at=NOW()",
            &[],
        )
        .await?;
        assert_eq!(
            voter_files(&tx, &t, &ev, &a, &ids).await?["files"],
            json!([])
        );
        tx.execute(
            "UPDATE sequent_backend.ballot_publication SET deleted_at=NULL",
            &[],
        )
        .await?;
        tx.rollback().await?;
        let unpublished: Option<Value> = db
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        assert!(unpublished.is_none());
        // A malformed style fails after some objects upload; no reference is activated.
        db.execute(
            "UPDATE sequent_backend.ballot_style SET ballot_eml='invalid'",
            &[],
        )
        .await?;
        let tx = db.transaction().await?;
        assert!(prepare_publication_files(&tx, &t, &ev, &p).await.is_err());
        tx.rollback().await?;
        let missing: Option<Value> = db
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        assert!(missing.is_none());
        db.execute(
            "UPDATE sequent_backend.ballot_style SET ballot_eml='{}'",
            &[],
        )
        .await?;
        let tx = db.transaction().await?;
        prepare_publication_files(&tx, &t, &ev, &p).await?;
        tx.commit().await?;
        let active: Value = db
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        let tx = db.transaction().await?;
        tx.execute(
            "UPDATE sequent_backend.ballot_publication SET annotations=NULL",
            &[],
        )
        .await?;
        prepare_publication_files(&tx, &t, &ev, &p).await?;
        let replacement: Value = tx
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        assert_ne!(active, replacement);
        tx.rollback().await?;
        let retained: Value = db
            .query_one(
                "SELECT annotations FROM sequent_backend.ballot_publication WHERE id=$1",
                &[&publication],
            )
            .await?
            .get(0);
        assert_eq!(active, retained);
        Ok(())
    }
}
