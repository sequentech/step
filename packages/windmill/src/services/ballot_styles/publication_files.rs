// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::publication_files::{PgPublicationRows, S3Endpoint, S3PublicationStorage};
use crate::adapters::system::RandomIds;
pub use crate::domain::publication_files::FILES_ANNOTATION;
use crate::domain::publication_files::{
    election_key, event_key, publication_root, split_event_presentation, style_key, summary_key,
    validate_publication_root,
};
use crate::ports::clock::IdGenerator;
use crate::ports::publication_files::{PublicationObjects, PublicationRows, PublicationStorage};
use anyhow::{bail, Context, Result};
use deadpool_postgres::Transaction;
use futures::TryStreamExt;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use uuid::Uuid;

const VOTER_URL_LIFETIME: Duration = Duration::from_secs(300);

/// Called under the event publication lock. The reference becomes visible only
/// when the transaction commits, after every immutable object has been verified.
pub async fn prepare_publication_files(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<()> {
    prepare_files(
        &PgPublicationRows { transaction: tx },
        &S3PublicationStorage {
            endpoint: S3Endpoint::Server,
        },
        &RandomIds,
        tenant,
        event,
        publication,
    )
    .await
}

async fn prepare_files(
    rows: &impl PublicationRows,
    storage: &impl PublicationStorage,
    ids: &impl IdGenerator,
    tenant: &str,
    event: &str,
    publication: &str,
) -> Result<()> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let publication = Uuid::parse_str(publication)?;
    let annotations = rows.annotations(tenant, event, publication).await?;
    if let Some(root) = annotations
        .as_ref()
        .and_then(|v| v.get(FILES_ANNOTATION))
        .and_then(Value::as_str)
    {
        validate_publication_root(root, tenant, event, publication)?;
        return Ok(());
    }
    // A new attempt never overwrites objects from a failed or active attempt.
    let root = publication_root(tenant, event, publication, ids.new_id());
    let objects = storage.open().await?;
    let mut event_data = rows.event(tenant, event).await?;

    for data in rows.elections(tenant, event, publication).await? {
        let id = data["id"].as_str().context("Missing election id")?;
        objects
            .put_immutable_json(&election_key(&root, id), &data)
            .await?;
    }
    let styles = rows.styles(tenant, event, publication).await?;
    futures::pin_mut!(styles);
    let mut shared_presentation: Option<String> = None;
    while let Some(mut data) = styles.try_next().await? {
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
        objects
            .put_immutable_json(&summary_key(&root, &id), &summary)
            .await?;
        objects
            .put_immutable_json(&style_key(&root, &id), &data)
            .await?;
    }
    event_data["ballot_eml_presentation"] = Value::String(shared_presentation.unwrap_or_default());
    objects
        .put_immutable_json(&event_key(&root), &event_data)
        .await?;
    rows.set_files_root(tenant, event, publication, &root).await
}

/// Only identifiers, active references and live policy are read here, never EML.
pub async fn voter_files(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
) -> Result<Value> {
    list_voter_files(
        &PgPublicationRows { transaction: tx },
        &S3PublicationStorage {
            endpoint: S3Endpoint::Public,
        },
        tenant,
        event,
        area,
        elections,
    )
    .await
}

async fn list_voter_files(
    rows: &impl PublicationRows,
    storage: &impl PublicationStorage,
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
    let styles = rows
        .published_styles(tenant, event, area, &elections)
        .await?;
    let status = rows
        .event_status(tenant, event)
        .await?
        .context("Election event not found")?;
    let objects = storage.open().await?;
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let mut signed_urls = HashMap::<String, String>::new();
    for style in styles {
        let id = style.id;
        let election = style.election_id;
        if !seen.insert(election) {
            bail!("Multiple active ballot styles for one election");
        }
        let root = style.root.context(
            "Publication requires S3 preparation; publish it again before serving voters",
        )?;
        validate_publication_root(&root, tenant, event, style.publication_id)?;
        let mut urls = serde_json::Map::new();
        for (name, key) in [
            ("event_url", event_key(&root)),
            ("election_url", election_key(&root, election)),
            ("summary_url", summary_key(&root, id)),
            ("style_url", style_key(&root, id)),
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
        files.push(json!({"id":id,"election_id":election,"version":root,"urls":urls,"status":style.status,"num_allowed_revotes":style.num_allowed_revotes,"voting_channels":style.voting_channels}));
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
