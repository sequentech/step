// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::ballot_style::{ElectionEventConfig, EVENT_CONFIG_FILE_NAME};
use crate::adapters::publication_files::{
    PgGeneratedStyles, PgPublicationRows, S3Endpoint, S3PublicationStorage,
};
pub use crate::domain::publication_files::FILES_ANNOTATION;
use crate::domain::publication_files::{
    election_key, event_key, publication_root, split_event_presentation, style_key, summary_key,
    validate_publication_root, PublishedBallotStyle,
};
use crate::ports::publication_files::{
    GeneratedStyles, PublicationObjects, PublicationRows, PublicationStorage,
};
use crate::postgres::ballot_publication::lock_publication_event;
use crate::postgres::document::insert_document;
use crate::postgres::election::get_display_voting_closes;
use crate::postgres::publication_files::{
    complete_ballot_publication_files, get_publication_elections, get_publication_event,
};
use crate::services::pg_lock::PgLock;
use anyhow::{bail, Context, Result};
use aws_sdk_s3::primitives::ByteStream;
use deadpool_postgres::{Pool, Transaction};
use futures::{stream, StreamExt, TryStreamExt};
use sequent_core::services::s3::{
    self, get_public_bucket, get_public_election_event_document_name_key, get_shared_s3_client,
};
use sequent_core::types::hasura::core::BallotPublication;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use uuid::Uuid;

pub const GENERATION_LEASE_SECONDS: i64 = 300;
const STYLE_PAGE_SIZE: i64 = 8;
const UPLOAD_CONCURRENCY: usize = 4;
const VOTER_URL_LIFETIME: Duration = Duration::from_secs(300);

pub struct PublicationData {
    pub(crate) event: Value,
    pub(crate) elections: Vec<Value>,
    style_count: i64,
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

pub(crate) async fn heartbeat(pool: &Pool, lease: &PgLock) -> Result<()> {
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
    let style_count = tx.query_one(
        "SELECT count(*) FROM sequent_backend.ballot_style WHERE tenant_id=$1 AND election_event_id=$2 AND ballot_publication_id=$3",
        &[&tenant, &event, &publication],
    ).await?.get(0);
    Ok(PublicationData {
        event: get_publication_event(tx, tenant, event).await?,
        elections: get_publication_elections(tx, tenant, event, publication).await?,
        style_count,
    })
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

async fn upload_batch(
    objects: &impl PublicationObjects,
    batch: Vec<(String, Value)>,
) -> Result<()> {
    stream::iter(batch)
        .map(|(key, value)| async move { objects.put_immutable_json(&key, &value).await })
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
    data: PublicationData,
) -> Result<()> {
    let tenant_id = Uuid::parse_str(tenant)?;
    let event_id = Uuid::parse_str(event)?;
    let publication_id = Uuid::parse_str(publication)?;
    let attempt = Uuid::parse_str(&lease.value)?;
    let root = publication_root(tenant_id, event_id, publication_id, attempt);
    write_publication_objects(
        &PgGeneratedStyles {
            pool,
            lease,
            tenant: tenant_id,
            event: event_id,
            publication: publication_id,
        },
        &S3PublicationStorage {
            endpoint: S3Endpoint::Server,
        },
        &root,
        data,
    )
    .await?;
    heartbeat(pool, lease).await?;
    complete_generation(pool, tenant, event, publication_id, lease, &root, attempt).await
}

/// Writes one attempt's immutable objects under `root`: the elections, each
/// style with its summary, then the event with the presentation the styles
/// share. Nothing refers to them until the generation completes.
async fn write_publication_objects(
    styles: &impl GeneratedStyles,
    storage: &impl PublicationStorage,
    root: &str,
    mut data: PublicationData,
) -> Result<()> {
    styles.keep_alive().await?;
    let objects = storage.open().await?;
    for elections in data.elections.chunks(STYLE_PAGE_SIZE as usize) {
        styles.keep_alive().await?;
        let mut batch = Vec::new();
        for election in elections {
            let id = election["id"].as_str().context("Missing election id")?;
            batch.push((election_key(root, id), election.clone()));
        }
        upload_batch(&objects, batch).await?;
    }
    let mut cursor: Option<Uuid> = None;
    let mut shared_presentation: Option<(String, Value)> = None;
    let mut styles_read = 0i64;
    loop {
        styles.keep_alive().await?;
        let page = styles.page(cursor, STYLE_PAGE_SIZE).await?;
        if page.is_empty() {
            break;
        }
        let mut batch = Vec::new();
        for (id, mut style) in page {
            if !style["deleted_at"].is_null() {
                bail!("Publication was removed during upload");
            }
            let eml = share_event_presentation(&mut style, &mut shared_presentation)?;
            // List information is small; contests/candidates are fetched only on selection.
            let summary = json!({"id":id, "area_presentation":eml.get("area_presentation"), "election_dates":eml.get("election_dates")});
            batch.push((summary_key(root, id), summary));
            batch.push((style_key(root, id), style));
            cursor = Some(id);
            styles_read += 1;
        }
        upload_batch(&objects, batch).await?;
    }
    if styles_read != data.style_count {
        bail!("Ballot styles changed during publication upload");
    }
    data.event["ballot_eml_presentation"] =
        Value::String(shared_presentation.map(|(raw, _)| raw).unwrap_or_default());
    objects
        .put_immutable_json(&event_key(root), &data.event)
        .await
}

/// Activates `root` unless the publication was removed, completed or taken
/// over by another task meanwhile, and refreshes the public login configuration.
async fn complete_generation(
    pool: &Pool,
    tenant: &str,
    event: &str,
    publication: Uuid,
    lease: &PgLock,
    root: &str,
    attempt: Uuid,
) -> Result<()> {
    let tenant_id = Uuid::parse_str(tenant)?;
    let event_id = Uuid::parse_str(event)?;
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    lock_publication_event(&tx, tenant, event).await?;
    lock_generation(&tx, lease).await?;
    if !complete_ballot_publication_files(
        &tx,
        tenant_id,
        event_id,
        publication,
        FILES_ANNOTATION,
        root,
    )
    .await?
    {
        bail!("Publication was removed, completed or replaced during upload");
    }
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
    if let Some(bytes) = public_config {
        let client = get_shared_s3_client(s3::S3Endpoint::Server).await?;
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

/// The event and elections as they were written with a publication.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicationSnapshot {
    pub event: Value,
    pub elections: Vec<Value>,
}

/// `None` for a publication whose objects were never prepared.
pub async fn publication_snapshot(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<Option<PublicationSnapshot>> {
    read_snapshot(
        &PgPublicationRows { transaction: tx },
        &S3PublicationStorage {
            endpoint: S3Endpoint::Server,
        },
        tenant,
        event,
        publication,
    )
    .await
}

async fn read_snapshot(
    rows: &impl PublicationRows,
    storage: &impl PublicationStorage,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<Option<PublicationSnapshot>> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let publication = Uuid::parse_str(publication)?;
    let annotations = rows.annotations(tenant, event, publication).await?;
    let Some(root) = annotations
        .as_ref()
        .and_then(|v| v.get(FILES_ANNOTATION))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    validate_publication_root(root, tenant, event, publication)?;
    let objects = storage.open().await?;
    let mut elections = Vec::new();
    for row in rows.elections(tenant, event, publication).await? {
        let id = row["id"].as_str().context("Missing election id")?;
        elections.push(objects.get_json(&election_key(root, id)).await?);
    }
    Ok(Some(PublicationSnapshot {
        event: objects.get_json(&event_key(root)).await?,
        elections,
    }))
}

pub struct VoterFileReferences {
    event: Uuid,
    status: Option<Value>,
    files: Vec<VoterFileReference>,
}

struct VoterFileReference {
    style: PublishedBallotStyle,
    root: String,
    signed_close: Option<Value>,
}

/// Read only identifiers, active references and live policy, never EML or publication metadata.
pub async fn load_voter_files(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
) -> Result<VoterFileReferences> {
    let mut references = load_references(
        &PgPublicationRows { transaction: tx },
        tenant,
        event,
        area,
        elections,
    )
    .await?;
    // Only metadata for files actually authorized and visible in this area.
    let visible: Vec<String> = references
        .files
        .iter()
        .map(|file| file.style.election_id.to_string())
        .collect();
    let closes = get_display_voting_closes(tx, tenant, event, &visible).await?;
    for file in &mut references.files {
        if let Some(close) = closes.get(&file.style.election_id.to_string()) {
            file.signed_close = Some(serde_json::to_value(close)?);
        }
    }
    Ok(references)
}

async fn load_references(
    rows: &impl PublicationRows,
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
    let styles = rows
        .published_styles(tenant, event, area, &elections)
        .await?;
    let status = rows
        .event_status(tenant, event)
        .await?
        .context("Election event not found")?;
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for style in styles {
        if !seen.insert(style.election_id) {
            bail!("Multiple active ballot styles for one election");
        }
        let root = style
            .root
            .clone()
            .context("Publication files are not prepared")?;
        validate_publication_root(&root, tenant, event, style.publication_id)?;
        files.push(VoterFileReference {
            style,
            root,
            signed_close: None,
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
    presign_references(
        &S3PublicationStorage {
            endpoint: S3Endpoint::Public,
        },
        references,
    )
    .await
}

async fn presign_references(
    storage: &impl PublicationStorage,
    references: VoterFileReferences,
) -> Result<Value> {
    if references.files.is_empty() {
        return Ok(json!({"event_id":references.event, "status":references.status, "files":[]}));
    }
    let objects = storage.open().await?;
    let mut files = Vec::new();
    let mut signed_urls = HashMap::<String, String>::new();
    for file in references.files {
        let (id, election, root) = (file.style.id, file.style.election_id, &file.root);
        let mut urls = serde_json::Map::new();
        for (name, key) in [
            ("event_url", event_key(root)),
            ("election_url", election_key(root, election)),
            ("summary_url", summary_key(root, id)),
            ("style_url", style_key(root, id)),
        ] {
            let url = if let Some(url) = signed_urls.get(&key) {
                url.clone()
            } else {
                let url = objects.presign_get(&key, VOTER_URL_LIFETIME).await?;
                signed_urls.insert(key, url.clone());
                url
            };
            urls.insert(name.into(), Value::String(url));
        }
        let mut entry = json!({"id":id,"election_id":election,"version":root,"urls":urls,
            "status":file.style.status,"num_allowed_revotes":file.style.num_allowed_revotes,
            "voting_channels":file.style.voting_channels});
        if let Some(close) = file.signed_close {
            entry["signed_close"] = close;
        }
        files.push(entry);
    }
    Ok(json!({"event_id":references.event,"status":references.status,"files":files}))
}

#[cfg(test)]
mod tests;
