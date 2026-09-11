// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::s3::{
    get_object_into_temp_file, get_private_bucket, get_shared_s3_client, S3Endpoint,
};
use sequent_core::types::hasura::core::{BallotPublication, BallotStyle};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tempfile::TempPath;
use uuid::Uuid;

use super::publication_files::{
    publication_data, require_publication_files, share_event_presentation, upload, FILES_ANNOTATION,
};
use crate::postgres::{
    ballot_publication::get_ballot_publication, ballot_style::export_event_ballot_styles,
};
use crate::services::import::import_election_event::replace_ids_in_filename;

pub const PUBLICATION_FILES_DIRECTORY: &str = "publication_files";
const ARCHIVE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct PublicationArchive {
    version: u32,
    publications: Vec<ArchivedPublication>,
}

impl PublicationArchive {
    pub fn has_generated_publications(&self) -> bool {
        self.publications
            .iter()
            .any(|item| item.publication.is_generated.unwrap_or(false))
    }
}

#[derive(Serialize, Deserialize)]
struct ArchivedPublication {
    publication: BallotPublication,
    ballot_styles: Vec<BallotStyle>,
}

fn file_names(publication: &ArchivedPublication) -> Vec<String> {
    let mut names = vec!["event.json".to_owned()];
    for id in publication
        .publication
        .election_ids
        .as_deref()
        .unwrap_or_default()
    {
        names.push(format!("election-{id}.json"));
    }
    for style in &publication.ballot_styles {
        names.push(format!("style-{}.json", style.id));
        names.push(format!("summary-{}.json", style.id));
    }
    names
}

fn archive_path(publication: &str, name: &str) -> String {
    format!("{PUBLICATION_FILES_DIRECTORY}/{publication}/{name}")
}

async fn download_file(client: &aws_sdk_s3::Client, bucket: &str, key: &str) -> Result<TempPath> {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let response = client.get_object().bucket(bucket).key(key).send().await?;
        let file = tempfile::NamedTempFile::new()?;
        let mut output = tokio::fs::File::from_std(file.reopen()?);
        tokio::io::copy(&mut response.body.into_async_read(), &mut output).await?;
        tokio::io::AsyncWriteExt::flush(&mut output).await?;
        Ok::<_, anyhow::Error>(file.into_temp_path())
    })
    .await
    .context("Publication archive download timed out")?
}

/// Export only the objects referenced by the selected publications, never orphan uploads.
pub async fn export_publication_archive(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<(PublicationArchive, Vec<(String, TempPath)>)> {
    let publications = get_ballot_publication(tx, tenant, event).await?;
    let styles = export_event_ballot_styles(tx, tenant, event).await?;
    let mut archive = PublicationArchive {
        version: ARCHIVE_VERSION,
        publications: Vec::new(),
    };
    let mut files = Vec::new();
    for publication in publications {
        let item = ArchivedPublication {
            ballot_styles: styles
                .iter()
                .filter(|style| style.ballot_publication_id == publication.id)
                .cloned()
                .collect(),
            publication,
        };
        if item
            .publication
            .annotations
            .as_ref()
            .and_then(|a| a.get(FILES_ANNOTATION))
            .is_some()
        {
            require_publication_files(&item.publication)?;
            let root = item
                .publication
                .annotations
                .as_ref()
                .and_then(|a| a[FILES_ANNOTATION].as_str())
                .context("Invalid publication file root")?;
            for name in file_names(&item) {
                let client = get_shared_s3_client(S3Endpoint::Server).await?;
                let file =
                    download_file(&client, &get_private_bucket()?, &format!("{root}/{name}"))
                        .await
                        .with_context(|| {
                            format!(
                                "Cannot export publication {} file {name}",
                                item.publication.id
                            )
                        })?;
                files.push((archive_path(&item.publication.id, &name), file));
            }
        }
        archive.publications.push(item);
    }
    Ok((archive, files))
}

fn validate_file_contents(
    item: &ArchivedPublication,
    objects: &HashMap<&str, &[u8]>,
) -> Result<()> {
    let read = |name: &str| -> Result<Value> {
        let path = archive_path(&item.publication.id, name);
        let bytes = objects
            .get(path.as_str())
            .with_context(|| format!("Missing publication file: {path}"))?;
        serde_json::from_slice(bytes).with_context(|| format!("Invalid publication JSON: {path}"))
    };
    let event = read("event.json")?;
    ensure!(
        event["id"] == item.publication.election_event_id,
        "Publication event file scope mismatch"
    );
    for id in item.publication.election_ids.as_deref().unwrap_or_default() {
        let election = read(&format!("election-{id}.json"))?;
        ensure!(
            election["id"] == *id
                && election["election_event_id"] == item.publication.election_event_id
                && election["tenant_id"] == item.publication.tenant_id,
            "Publication election file scope mismatch"
        );
    }
    for style in &item.ballot_styles {
        let data = read(&format!("style-{}.json", style.id))?;
        ensure!(
            data["id"] == style.id
                && data["tenant_id"] == style.tenant_id
                && data["election_event_id"] == style.election_event_id
                && data["election_id"] == style.election_id
                && data["area_id"].as_str() == style.area_id.as_deref(),
            "Publication style file scope mismatch"
        );
        let eml = if let Some(eml) = data["ballot_eml"].as_str() {
            eml.to_owned()
        } else {
            format!(
                "{}{}{}",
                data["ballot_eml_prefix"]
                    .as_str()
                    .context("Missing ballot prefix")?,
                event["ballot_eml_presentation"]
                    .as_str()
                    .context("Missing shared ballot presentation")?,
                data["ballot_eml_suffix"]
                    .as_str()
                    .context("Missing ballot suffix")?
            )
        };
        ensure!(
            style.ballot_eml.as_deref() == Some(eml.as_str()),
            "Publication file differs from its stored ballot"
        );
        ensure!(
            read(&format!("summary-{}.json", style.id))?["id"] == style.id,
            "Publication summary file scope mismatch"
        );
    }
    Ok(())
}

fn remap(value: &mut Value, ids: &HashMap<String, String>) {
    match value {
        Value::String(text) => *text = replace_ids_in_filename(text, ids),
        Value::Array(values) => values.iter_mut().for_each(|value| remap(value, ids)),
        Value::Object(values) => {
            let old = std::mem::take(values);
            for (key, mut value) in old {
                remap(&mut value, ids);
                values.insert(replace_ids_in_filename(&key, ids), value);
            }
        }
        _ => {}
    }
}

fn mapped_id(ids: &HashMap<String, String>, id: &str) -> Result<String> {
    let target = ids.get(id).with_context(|| {
        format!("Publication references an entity absent from the event archive: {id}")
    })?;
    Uuid::parse_str(target)?;
    Ok(target.clone())
}

fn read_archive(bytes: &[u8]) -> Result<PublicationArchive> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value.is_object() {
        let archive: PublicationArchive = serde_json::from_value(value)?;
        ensure!(
            archive.version == ARCHIVE_VERSION,
            "Unsupported publication archive version {}",
            archive.version
        );
        return Ok(archive);
    }
    // Old exports contain only EML. Restore these as generated drafts because
    // the format did not record publication dates or publication metadata.
    let designs = value.as_array().context("Invalid publication archive")?;
    let mut publications = Vec::new();
    for design in designs {
        let id = design["ballot_publication_id"]
            .as_str()
            .context("Missing publication id")?;
        Uuid::parse_str(id)?;
        let emls = design["ballot_styles"]
            .as_array()
            .context("Missing ballot styles")?;
        let first = emls
            .first()
            .context("Legacy publication contains no ballot styles")?;
        let now = chrono::Utc::now();
        let mut elections = Vec::new();
        let mut styles = Vec::new();
        for eml in emls {
            let election = eml["election_id"].as_str().context("Missing election id")?;
            if !elections.iter().any(|id| id == election) {
                elections.push(election.to_owned());
            }
            styles.push(serde_json::from_value(json!({
                "id": eml["id"], "tenant_id": eml["tenant_id"], "election_event_id": eml["election_event_id"],
                "election_id": election, "area_id": eml["area_id"], "ballot_publication_id": id,
                "ballot_eml": serde_json::to_string(eml)?, "created_at": now, "last_updated_at": now
            }))?);
        }
        let publication = serde_json::from_value(json!({
            "id": id, "tenant_id": first["tenant_id"], "election_event_id": first["election_event_id"],
            "election_ids": elections, "is_generated": true, "created_at": now
        }))?;
        publications.push(ArchivedPublication {
            publication,
            ballot_styles: styles,
        });
    }
    Ok(PublicationArchive {
        version: ARCHIVE_VERSION,
        publications,
    })
}

/// Restore records and files in the import transaction. Readiness is written only
/// after every required object has uploaded and passed readback verification.
pub async fn import_publication_archive(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    bytes: &[u8],
    entries: &[(String, Vec<u8>)],
    replacement_map: &HashMap<String, String>,
) -> Result<()> {
    let archive = read_archive(bytes)?;
    let mut ids = replacement_map.clone();
    let mut publication_ids = HashSet::new();
    let mut style_ids = HashSet::new();
    let mut objects = HashMap::new();
    for (name, data) in entries
        .iter()
        .filter(|(name, _)| name.starts_with(&format!("{PUBLICATION_FILES_DIRECTORY}/")))
    {
        ensure!(
            objects.insert(name.as_str(), data.as_slice()).is_none(),
            "Duplicate publication archive entry: {name}"
        );
    }
    // Validate all scopes and required files before creating records or uploading.
    for item in &archive.publications {
        let publication = &item.publication;
        Uuid::parse_str(&publication.id)?;
        ensure!(
            publication_ids.insert(publication.id.clone()),
            "Duplicate publication id"
        );
        ensure!(
            mapped_id(&ids, &publication.tenant_id)? == tenant
                && mapped_id(&ids, &publication.election_event_id)? == event,
            "Publication belongs to a different event"
        );
        ids.insert(publication.id.clone(), Uuid::new_v4().to_string());
        for election in publication.election_ids.as_deref().unwrap_or_default() {
            mapped_id(&ids, election)?;
        }
        if let Some(election) = &publication.election_id {
            mapped_id(&ids, election)?;
        }
        for style in &item.ballot_styles {
            Uuid::parse_str(&style.id)?;
            ensure!(
                style_ids.insert(style.id.clone()),
                "Duplicate ballot style id"
            );
            ensure!(
                style.tenant_id == publication.tenant_id
                    && style.election_event_id == publication.election_event_id
                    && style.ballot_publication_id == publication.id,
                "Ballot style scope differs from publication"
            );
            ensure!(
                publication
                    .election_ids
                    .as_ref()
                    .is_some_and(|elections| elections.contains(&style.election_id)),
                "Ballot style election is outside publication"
            );
            mapped_id(&ids, &style.election_id)?;
            mapped_id(
                &ids,
                style
                    .area_id
                    .as_deref()
                    .context("Missing ballot style area")?,
            )?;
            // This field is not populated by ballot generation. Do not retain an
            // invalid signature if an external archive supplies signed content.
            ensure!(
                style.ballot_signature.is_none(),
                "Cannot remap an externally signed ballot style"
            );
            ids.insert(style.id.clone(), Uuid::new_v4().to_string());
        }
        if publication
            .annotations
            .as_ref()
            .and_then(|a| a.get(FILES_ANNOTATION))
            .is_some()
        {
            require_publication_files(publication)?;
            validate_file_contents(item, &objects)?;
        }
    }
    for item in archive.publications {
        let source = &item.publication;
        let publication_id = mapped_id(&ids, &source.id)?;
        let mut publication = serde_json::to_value(source)?;
        remap(&mut publication, &ids);
        publication["is_generated"] = json!(false);
        publication["published_at"] = Value::Null;
        publication["deleted_at"] = Value::Null;
        if let Some(annotations) = publication["annotations"].as_object_mut() {
            annotations.remove(FILES_ANNOTATION);
        }
        tx.execute(
            "INSERT INTO sequent_backend.ballot_publication (id,tenant_id,election_event_id,labels,annotations,created_at,deleted_at,created_by_user_id,is_generated,election_ids,published_at,election_id) SELECT id,tenant_id,election_event_id,labels,annotations,COALESCE(created_at,now()),deleted_at,created_by_user_id,is_generated,election_ids,published_at,election_id FROM jsonb_populate_record(NULL::sequent_backend.ballot_publication,$1)",
            &[&publication],
        ).await?;
        for source_style in &item.ballot_styles {
            let mut style = serde_json::to_value(source_style)?;
            remap(&mut style, &ids);
            tx.execute(
                "INSERT INTO sequent_backend.ballot_style (id,tenant_id,election_event_id,election_id,area_id,created_at,last_updated_at,annotations,labels,ballot_eml,ballot_signature,status,deleted_at,ballot_publication_id) SELECT id,tenant_id,election_event_id,election_id,area_id,COALESCE(created_at,now()),COALESCE(last_updated_at,now()),annotations,labels,ballot_eml,ballot_signature,status,deleted_at,ballot_publication_id FROM jsonb_populate_record(NULL::sequent_backend.ballot_style,$1)",
                &[&style],
            ).await?;
        }
        if source.is_generated.unwrap_or(false) {
            let root = format!(
                "tenant-{tenant}/event-{event}/publication-{publication_id}/{}",
                Uuid::new_v4()
            );
            let client = get_shared_s3_client(S3Endpoint::Server).await?;
            let bucket = get_private_bucket()?;
            if source
                .annotations
                .as_ref()
                .and_then(|a| a.get(FILES_ANNOTATION))
                .is_some()
            {
                for name in file_names(&item) {
                    let path = archive_path(&source.id, &name);
                    let mut data: Value = serde_json::from_slice(objects[path.as_str()])?;
                    remap(&mut data, &ids);
                    upload(
                        &client,
                        &bucket,
                        &format!("{root}/{}", replace_ids_in_filename(&name, &ids)),
                        &data,
                    )
                    .await?;
                }
            } else {
                let mut data = publication_data(tx, tenant, event, &publication_id).await?;
                for election in &data.elections {
                    let id = election["id"].as_str().context("Missing election id")?;
                    upload(
                        &client,
                        &bucket,
                        &format!("{root}/election-{id}.json"),
                        election,
                    )
                    .await?;
                }
                let mut shared = None;
                let mut cursor: Option<Uuid> = None;
                loop {
                    let rows = tx
                        .query(
                            include_str!("../../postgres/sql/publication_file_styles.sql"),
                            &[
                                &Uuid::parse_str(tenant)?,
                                &Uuid::parse_str(event)?,
                                &Uuid::parse_str(&publication_id)?,
                                &cursor,
                                &8i64,
                            ],
                        )
                        .await?;
                    if rows.is_empty() {
                        break;
                    }
                    for row in rows {
                        let id: Uuid = row.get("id");
                        let mut style: Value = row.get("data");
                        let eml = share_event_presentation(&mut style, &mut shared)?;
                        upload(&client, &bucket, &format!("{root}/summary-{id}.json"), &json!({"id":id,"area_presentation":eml.get("area_presentation"),"election_dates":eml.get("election_dates")})).await?;
                        upload(&client, &bucket, &format!("{root}/style-{id}.json"), &style)
                            .await?;
                        cursor = Some(id);
                    }
                }
                data.event["ballot_eml_presentation"] =
                    json!(shared.map(|(raw, _)| raw).unwrap_or_default());
                upload(&client, &bucket, &format!("{root}/event.json"), &data.event).await?;
            }
            tx.execute("UPDATE sequent_backend.ballot_publication SET is_generated=true, published_at=$4, deleted_at=$5, annotations=COALESCE(annotations,'{}'::jsonb)||jsonb_build_object($6::text,$7::text) WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3",
                &[&Uuid::parse_str(tenant)?,&Uuid::parse_str(event)?,&Uuid::parse_str(&publication_id)?,&source.published_at,&source.deleted_at,&FILES_ANNOTATION,&root]).await?;
            // The voting portal reads this flag rather than publication history.
            // Restore it only after the published archive's files are ready; keep
            // the import's reset voting state and all other status fields intact.
            if source.published_at.is_some() && source.deleted_at.is_none() {
                tx.execute(
                    "UPDATE sequent_backend.election_event SET status=jsonb_set(COALESCE(status,'{}'::jsonb),'{is_published}','true'::jsonb) WHERE tenant_id=$1 AND id=$2",
                    &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event)?],
                ).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
