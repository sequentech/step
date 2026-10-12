// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use aws_sdk_s3::{presigning::PresigningConfig, primitives::ByteStream, Client};
use deadpool_postgres::{Pool, Transaction};
use futures::{stream, StreamExt, TryStreamExt};
use sequent_core::services::s3::{
    get_private_bucket, get_public_bucket, get_public_election_event_document_name_key,
    get_shared_s3_client, S3Endpoint,
};
use sequent_core::types::hasura::core::BallotPublication;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use uuid::Uuid;

use super::ballot_style::{ElectionEventConfig, EVENT_CONFIG_FILE_NAME};
use crate::postgres::{ballot_publication::lock_publication_event, document::insert_document};
use crate::services::pg_lock::PgLock;

pub const FILES_ANNOTATION: &str = "ballot_files_v1";
pub const GENERATION_LEASE_SECONDS: i64 = 300;
const STYLE_PAGE_SIZE: i64 = 8;
const UPLOAD_CONCURRENCY: usize = 4;

pub struct PublicationData {
    pub(crate) event: Value,
    pub(crate) elections: Vec<Value>,
    style_count: i64,
}

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

pub fn require_publication_files(publication: &BallotPublication) -> Result<()> {
    let root = publication
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(FILES_ANNOTATION))
        .and_then(Value::as_str)
        .context("Publication files are not ready; generate a new publication")?;
    validate_publication_root(
        root,
        Uuid::parse_str(&publication.tenant_id)?,
        Uuid::parse_str(&publication.election_event_id)?,
        Uuid::parse_str(&publication.id)?,
    )
}

/// A transaction holding this row cannot be overtaken by an expired task lease.
pub async fn lock_generation(tx: &Transaction<'_>, lease: &PgLock) -> Result<()> {
    tx.query_opt(
        include_str!("../../postgres/sql/lock_publication_generation.sql"),
        &[&lease.key, &lease.value],
    )
    .await?
    .context("Ballot generation lease expired or was replaced")?;
    Ok(())
}

pub async fn renew_generation(tx: &Transaction<'_>, lease: &PgLock) -> Result<()> {
    // Call while holding lock_generation's row lock. A long database-only
    // generation can safely extend its own lease before releasing that lock.
    let changed = tx.execute(
        "UPDATE sequent_backend.lock SET expiry_date=clock_timestamp()+$3::bigint*interval '1 second' WHERE key=$1 AND value=$2",
        &[&lease.key, &lease.value, &GENERATION_LEASE_SECONDS],
    ).await?;
    if changed != 1 {
        bail!("Ballot generation lease was replaced");
    }
    Ok(())
}

async fn heartbeat(pool: &Pool, lease: &PgLock) -> Result<()> {
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    lock_generation(&tx, lease).await?;
    renew_generation(&tx, lease).await?;
    tx.commit().await?;
    Ok(())
}

/// Read metadata in the transaction that generates the ballot styles.
pub async fn publication_data(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<PublicationData> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let publication = Uuid::parse_str(publication)?;
    // Recheck current eligibility before committing potentially expensive generation.
    tx.query_opt(
        "SELECT id FROM sequent_backend.ballot_publication WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3 AND deleted_at IS NULL AND published_at IS NULL FOR NO KEY UPDATE",
        &[&tenant, &event, &publication],
    ).await?.context("Publication was removed during ballot generation")?;
    let event_data: Value = tx.query_one(
        "SELECT jsonb_build_object('id',id,'presentation',presentation,'description',description) FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
        &[&tenant, &event],
    ).await?.get(0);
    let elections = tx
        .query(
            r#"
        SELECT jsonb_build_object('id', id, 'tenant_id', tenant_id,
            'election_event_id', election_event_id, 'annotations', annotations,
            'created_at', created_at, 'description', description,
            'is_consolidated_ballot_encoding', is_consolidated_ballot_encoding,
            'labels', labels, 'last_updated_at', last_updated_at,
            'presentation', presentation, 'spoil_ballot_option', spoil_ballot_option)
        FROM sequent_backend.election
        WHERE tenant_id=$1 AND election_event_id=$2
          AND id = ANY(SELECT unnest(election_ids) FROM sequent_backend.ballot_publication
              WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3)
        ORDER BY id
    "#,
            &[&tenant, &event, &publication],
        )
        .await?
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    let style_count = tx.query_one(
        "SELECT count(*) FROM sequent_backend.ballot_style WHERE tenant_id=$1 AND election_event_id=$2 AND ballot_publication_id=$3",
        &[&tenant, &event, &publication],
    ).await?.get(0);
    Ok(PublicationData {
        event: event_data,
        elections,
        style_count,
    })
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

/// Sort object keys before signing new styles so shared metadata has stable bytes.
pub(super) fn ordered_json_string(value: &impl Serialize) -> Result<String> {
    let mut value = serde_json::to_value(value)?;
    value.sort_all_objects();
    Ok(serde_json::to_string(&value)?)
}

/// Existing signed styles can have equivalent JSON encoded in a different order.
/// Keep their original EML when it cannot reuse the shared bytes exactly.
pub(crate) fn share_event_presentation(
    data: &mut Value,
    shared: &mut Option<(String, Value)>,
) -> Result<Value> {
    let eml = data["ballot_eml"].as_str().context("Missing ballot EML")?;
    let parsed: Value = serde_json::from_str(eml)?;
    let (prefix, presentation, suffix) = split_event_presentation(eml)?;
    let semantic = &parsed["election_event_presentation"];
    if let Some((existing, existing_value)) = shared.as_ref() {
        if existing_value != semantic {
            bail!("Inconsistent event presentation within publication");
        }
        if existing != &presentation {
            return Ok(parsed);
        }
    } else {
        *shared = Some((presentation, semantic.clone()));
    }
    data["ballot_eml"] = Value::Null;
    data["ballot_eml_prefix"] = Value::String(prefix);
    data["ballot_eml_suffix"] = Value::String(suffix);
    Ok(parsed)
}

pub(crate) async fn upload(client: &Client, bucket: &str, key: &str, value: &Value) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(60), async {
        let bytes = serde_json::to_vec(value)?;
        let result = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type("application/json")
            .cache_control("private, max-age=300")
            .if_none_match("*")
            .body(ByteStream::from(bytes.clone()))
            .send()
            .await;
        if let Err(error) = result {
            // A retry after a lost PUT response can find the immutable object already there.
            if error
                .raw_response()
                .map(|response| response.status().as_u16())
                != Some(412)
            {
                return Err(error).context("Cannot upload ballot publication object");
            }
        }
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
    })
    .await
    .context("Ballot publication upload timed out")?
}

async fn upload_batch(client: &Client, bucket: &str, objects: Vec<(String, Value)>) -> Result<()> {
    stream::iter(objects)
        .map(|(key, value)| async move { upload(client, bucket, &key, &value).await })
        .buffer_unordered(UPLOAD_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?;
    Ok(())
}

/// Upload generation metadata and paged styles without retaining a database connection.
pub async fn prepare_publication_files(
    pool: &Pool,
    tenant: &str,
    event: &str,
    publication: &str,
    lease: &PgLock,
    mut data: PublicationData,
) -> Result<()> {
    let tenant_id = Uuid::parse_str(tenant)?;
    let event_id = Uuid::parse_str(event)?;
    let publication_id = Uuid::parse_str(publication)?;
    let attempt = Uuid::parse_str(&lease.value)?;
    let root =
        format!("tenant-{tenant_id}/event-{event_id}/publication-{publication_id}/{attempt}");
    let bucket = get_private_bucket()?;
    let client = get_shared_s3_client(S3Endpoint::Server).await?;
    heartbeat(pool, lease).await?;
    for elections in data.elections.chunks(STYLE_PAGE_SIZE as usize) {
        heartbeat(pool, lease).await?;
        let mut objects = Vec::new();
        for data in elections {
            let id = data["id"].as_str().context("Missing election id")?;
            objects.push((format!("{root}/election-{id}.json"), data.clone()));
        }
        upload_batch(&client, &bucket, objects).await?;
    }
    let mut cursor: Option<Uuid> = None;
    let mut shared_presentation: Option<(String, Value)> = None;
    let mut styles_read = 0i64;
    loop {
        heartbeat(pool, lease).await?;
        let rows = {
            let db = pool.get().await?;
            db.query(
                include_str!("../../postgres/sql/publication_file_styles.sql"),
                &[
                    &tenant_id,
                    &event_id,
                    &publication_id,
                    &cursor,
                    &STYLE_PAGE_SIZE,
                ],
            )
            .await?
        };
        if rows.is_empty() {
            break;
        }
        let mut objects = Vec::new();
        for row in rows {
            let id: Uuid = row.try_get("id")?;
            let mut data: Value = row.try_get("data")?;
            if !data["deleted_at"].is_null() {
                bail!("Publication was removed during upload");
            }
            let summary_data = share_event_presentation(&mut data, &mut shared_presentation)?;
            let summary = json!({"id":id, "area_presentation":summary_data.get("area_presentation"), "election_dates":summary_data.get("election_dates")});
            objects.push((format!("{root}/summary-{id}.json"), summary));
            objects.push((format!("{root}/style-{id}.json"), data));
            cursor = Some(id);
            styles_read += 1;
        }
        upload_batch(&client, &bucket, objects).await?;
    }
    if styles_read != data.style_count {
        bail!("Ballot styles changed during publication upload");
    }
    data.event["ballot_eml_presentation"] =
        Value::String(shared_presentation.map(|(raw, _)| raw).unwrap_or_default());
    upload(&client, &bucket, &format!("{root}/event.json"), &data.event).await?;
    heartbeat(pool, lease).await?;

    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    lock_publication_event(&tx, tenant, event).await?;
    lock_generation(&tx, lease).await?;
    tx.query_opt(
        include_str!("../../postgres/sql/complete_ballot_publication_files.sql"),
        &[
            &tenant_id,
            &event_id,
            &publication_id,
            &root,
            &FILES_ANNOTATION,
        ],
    )
    .await?
    .context("Publication was removed, completed or replaced during upload")?;
    // This one mutable login configuration is event-wide. Serialize its write
    // with event edits/deletion and read live metadata: an older draft finishing
    // late must not overwrite it with obsolete metadata.
    let event_presentation: Option<Value> = tx
        .query_one(
            "SELECT presentation FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
            &[&tenant_id, &event_id],
        )
        .await?
        .get(0);
    let public_config = match event_presentation.as_ref().filter(|value| !value.is_null()) {
        Some(presentation) => Some(serde_json::to_vec(&ElectionEventConfig {
            id: attempt.to_string(),
            tenant_id: tenant.to_owned(),
            election_event_id: event.to_owned(),
            election_event_presentation: serde_json::from_value(presentation.clone())?,
        })?),
        None => None,
    };
    if let Some(bytes) = &public_config {
        tokio::time::timeout(
            Duration::from_secs(60),
            client
                .put_object()
                .bucket(get_public_bucket()?)
                .key(get_public_election_event_document_name_key(
                    tenant,
                    event,
                    EVENT_CONFIG_FILE_NAME,
                ))
                .content_type("application/json")
                .content_disposition(format!("attachment; filename=\"{EVENT_CONFIG_FILE_NAME}\""))
                .body(ByteStream::from(bytes.clone()))
                .send(),
        )
        .await
        .context("Public election event configuration upload timed out")?
        .context("Cannot upload public election event configuration")?;
    }

    if let Some(bytes) = public_config {
        insert_document(
            &tx,
            tenant,
            Some(event.to_owned()),
            EVENT_CONFIG_FILE_NAME,
            "application/json",
            bytes.len().try_into()?,
            true,
            Some(attempt.to_string()),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub struct VoterFileReferences {
    event: Uuid,
    status: Option<Value>,
    files: Vec<VoterFileReference>,
}

struct VoterFileReference {
    id: Uuid,
    election: Uuid,
    root: String,
    status: Option<Value>,
    revotes: Option<i64>,
    channels: Option<Value>,
}

/// Read only identifiers, active references and live policy, never EML or publication metadata.
pub async fn load_voter_files(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
) -> Result<VoterFileReferences> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let area = Uuid::parse_str(area)?;
    let elections = elections
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = tx
        .query(
            r#"
        SELECT s.id, s.election_id, p.id AS publication_id, p.annotations->>$5::text AS root,
            e.status, e.num_allowed_revotes::bigint AS num_allowed_revotes, e.voting_channels
        FROM sequent_backend.ballot_style s
        JOIN sequent_backend.ballot_publication p ON p.id=s.ballot_publication_id
          AND p.tenant_id=s.tenant_id AND p.election_event_id=s.election_event_id
        JOIN sequent_backend.election e ON e.id=s.election_id
          AND e.tenant_id=s.tenant_id AND e.election_event_id=s.election_event_id
        WHERE s.tenant_id=$1 AND s.election_event_id=$2 AND s.area_id=$3
          AND s.election_id=ANY($4) AND s.deleted_at IS NULL
          AND p.is_generated IS TRUE AND p.published_at IS NOT NULL AND p.deleted_at IS NULL
        ORDER BY p.published_at DESC, s.election_id
    "#,
            &[&tenant, &event, &area, &elections, &FILES_ANNOTATION],
        )
        .await?;
    let status = tx
        .query_opt(
            "SELECT status FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
            &[&tenant, &event],
        )
        .await?
        .context("Election event not found")?
        .try_get(0)?;
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let election: Uuid = row.try_get("election_id")?;
        if !seen.insert(election) {
            bail!("Multiple active ballot styles for one election");
        }
        let root: String = row
            .try_get::<_, Option<String>>("root")?
            .context("Publication files are not prepared")?;
        validate_publication_root(&root, tenant, event, row.try_get("publication_id")?)?;
        files.push(VoterFileReference {
            id,
            election,
            root,
            status: row.try_get("status")?,
            revotes: row.try_get("num_allowed_revotes")?,
            channels: row.try_get("voting_channels")?,
        });
    }
    Ok(VoterFileReferences {
        event,
        status,
        files,
    })
}

/// Signing may refresh role credentials; callers release their database lease first.
pub async fn presign_voter_files(references: VoterFileReferences) -> Result<Value> {
    if references.files.is_empty() {
        return Ok(json!({"event_id":references.event, "status":references.status, "files":[]}));
    }
    let bucket = get_private_bucket()?;
    let client = get_shared_s3_client(S3Endpoint::Client).await?;
    let mut files = Vec::new();
    let mut signed_urls = HashMap::<String, String>::new();
    for file in references.files {
        let mut urls = serde_json::Map::new();
        for (name, key) in [
            ("event_url", format!("{}/event.json", file.root)),
            (
                "election_url",
                format!("{}/election-{}.json", file.root, file.election),
            ),
            (
                "summary_url",
                format!("{}/summary-{}.json", file.root, file.id),
            ),
            ("style_url", format!("{}/style-{}.json", file.root, file.id)),
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
        files.push(json!({"id":file.id,"election_id":file.election,"version":file.root,"urls":urls,
            "status":file.status,"num_allowed_revotes":file.revotes,"voting_channels":file.channels}));
    }
    Ok(json!({"event_id":references.event,"status":references.status,"files":files}))
}

#[cfg(test)]
mod tests;
