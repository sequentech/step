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
        let use_shared_presentation = match shared_presentation.as_ref() {
            Some(existing) if existing != &presentation => {
                if existing.is_empty()
                    || presentation.is_empty()
                    || serde_json::from_str::<Value>(existing)?
                        != serde_json::from_str::<Value>(&presentation)?
                {
                    bail!("Inconsistent event presentation within publication");
                }
                // Equivalent JSON can have different signed bytes. Preserve the
                // original full EML instead of reconstructing it with another style's fragment.
                false
            }
            Some(_) => true,
            None => {
                shared_presentation = Some(presentation);
                true
            }
        };
        if use_shared_presentation {
            data["ballot_eml"] = Value::Null;
            data["ballot_eml_prefix"] = Value::String(prefix);
            data["ballot_eml_suffix"] = Value::String(suffix);
        }
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
    use crate::adapters::memory::clock::SequentialIds;
    use crate::adapters::memory::publication_files::{
        MemoryPublicationObjects, MemoryPublicationRows, PublicationRecord,
    };
    use crate::domain::publication_files::PublishedBallotStyle;
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

    const TENANT: Uuid = Uuid::from_u128(0xa1);
    const EVENT: Uuid = Uuid::from_u128(0xe1);
    const PUBLICATION: Uuid = Uuid::from_u128(0xb1);
    const OTHER_PUBLICATION: Uuid = Uuid::from_u128(0xb2);
    const AREA: Uuid = Uuid::from_u128(0xa2);
    const ELECTION: Uuid = Uuid::from_u128(0xc1);
    const OTHER_ELECTION: Uuid = Uuid::from_u128(0xc2);
    const STYLE: Uuid = Uuid::from_u128(0x51);
    const OTHER_STYLE: Uuid = Uuid::from_u128(0x52);
    // The first and second attempts, as `SequentialIds` numbers them.
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

    fn two_elections(styles: Vec<Value>) -> PublicationRecord {
        PublicationRecord {
            elections: vec![election_object(ELECTION), election_object(OTHER_ELECTION)],
            styles,
            ..PublicationRecord::default()
        }
    }

    fn consistent_styles() -> Vec<Value> {
        vec![
            style_row(STYLE, ELECTION, &eml(PRESENTATION)),
            style_row(OTHER_STYLE, OTHER_ELECTION, &eml(PRESENTATION)),
        ]
    }

    fn publication(record: PublicationRecord) -> MemoryPublicationRows {
        MemoryPublicationRows::default()
            .with_event(TENANT, EVENT, event_object(), None)
            .with_publication(TENANT, EVENT, PUBLICATION, record)
    }

    async fn prepare(
        rows: &MemoryPublicationRows,
        objects: &MemoryPublicationObjects,
        ids: &SequentialIds,
    ) -> Result<()> {
        prepare_files(
            rows,
            objects,
            ids,
            &TENANT.to_string(),
            &EVENT.to_string(),
            &PUBLICATION.to_string(),
        )
        .await
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

    fn files_annotation(root: &str) -> Option<Value> {
        Some(json!({"ballot_files_v1": root}))
    }

    #[tokio::test]
    async fn preparing_writes_elections_then_each_style_then_the_event_under_a_new_root() {
        let rows = publication(two_elections(consistent_styles()));
        let objects = MemoryPublicationObjects::default();
        prepare(&rows, &objects, &SequentialIds::default())
            .await
            .unwrap();

        let mut expected = election_keys();
        expected.extend(keys(
            ROOT,
            &[
                format!("summary-{STYLE}.json"),
                format!("style-{STYLE}.json"),
                format!("summary-{OTHER_STYLE}.json"),
                format!("style-{OTHER_STYLE}.json"),
                "event.json".to_owned(),
            ],
        ));
        assert_eq!(objects.keys(), expected);
        assert_eq!(
            rows.stored_annotations(TENANT, EVENT, PUBLICATION),
            files_annotation(ROOT)
        );
    }

    #[tokio::test]
    async fn prepared_styles_drop_their_eml_and_rebuild_it_exactly_with_the_event() {
        let rows = publication(two_elections(consistent_styles()));
        let objects = MemoryPublicationObjects::default();
        prepare(&rows, &objects, &SequentialIds::default())
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
            let rows = publication(two_elections(vec![first_row, second_row]));
            let objects = MemoryPublicationObjects::default();
            prepare(&rows, &objects, &SequentialIds::default())
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
            assert_eq!(
                rows.stored_annotations(TENANT, EVENT, PUBLICATION),
                files_annotation(ROOT)
            );
        }
    }

    #[tokio::test]
    async fn missing_and_present_event_presentations_remain_inconsistent() {
        for missing in [r#"{"contests":[]}"#.to_string(), eml("null")] {
            for (first, second) in [
                (missing.clone(), eml(PRESENTATION)),
                (eml(PRESENTATION), missing.clone()),
            ] {
                let rows = publication(two_elections(vec![
                    style_row(STYLE, ELECTION, &first),
                    style_row(OTHER_STYLE, OTHER_ELECTION, &second),
                ]));
                let objects = MemoryPublicationObjects::default();
                let error = prepare(&rows, &objects, &SequentialIds::default())
                    .await
                    .unwrap_err();
                assert_eq!(
                    error.to_string(),
                    "Inconsistent event presentation within publication"
                );
                assert!(objects.json(&event_key(ROOT)).is_none());
                assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
            }
        }
    }

    #[tokio::test]
    async fn a_style_without_an_event_presentation_keeps_its_whole_eml() {
        let eml = r#"{"contests":[]}"#;
        let rows = publication(two_elections(vec![style_row(STYLE, ELECTION, eml)]));
        let objects = MemoryPublicationObjects::default();
        prepare(&rows, &objects, &SequentialIds::default())
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
        let rows = publication(two_elections(Vec::new()));
        let objects = MemoryPublicationObjects::default();
        prepare(&rows, &objects, &SequentialIds::default())
            .await
            .unwrap();

        let mut expected = election_keys();
        expected.push(format!("{ROOT}/event.json"));
        assert_eq!(objects.keys(), expected);
        assert_eq!(
            objects.json(&format!("{ROOT}/event.json")).unwrap()["ballot_eml_presentation"],
            ""
        );
        assert_eq!(
            rows.stored_annotations(TENANT, EVENT, PUBLICATION),
            files_annotation(ROOT)
        );
    }

    #[tokio::test]
    async fn a_prepared_publication_is_kept_without_opening_the_bucket() {
        let annotations = json!({"ballot_files_v1": ROOT, "imported": true});
        let rows = publication(PublicationRecord {
            annotations: Some(annotations.clone()),
            ..two_elections(consistent_styles())
        });
        // Opening this bucket fails, so success shows it was never opened.
        let objects = MemoryPublicationObjects::unavailable();
        prepare(&rows, &objects, &SequentialIds::default())
            .await
            .unwrap();

        assert!(objects.keys().is_empty());
        assert_eq!(
            rows.stored_annotations(TENANT, EVENT, PUBLICATION),
            Some(annotations)
        );
    }

    #[tokio::test]
    async fn a_recorded_root_outside_the_publication_is_rejected_without_writing() {
        for (root, error) in [
            (OTHER_ROOT.to_owned(), "Publication object scope mismatch"),
            (
                format!("tenant-{EVENT}/event-{TENANT}/publication-{PUBLICATION}/{STYLE}"),
                "Publication object scope mismatch",
            ),
            (
                format!("tenant-{TENANT}/event-{EVENT}/publication-{PUBLICATION}/latest"),
                "Invalid publication object version",
            ),
            (
                format!("{ROOT}/../../publication-{OTHER_PUBLICATION}"),
                "Invalid publication object version",
            ),
        ] {
            let annotations = json!({ "ballot_files_v1": root });
            let rows = publication(PublicationRecord {
                annotations: Some(annotations.clone()),
                ..two_elections(consistent_styles())
            });
            let objects = MemoryPublicationObjects::default();
            let result = prepare(&rows, &objects, &SequentialIds::default()).await;

            assert_eq!(result.unwrap_err().to_string(), error, "{root}");
            assert!(objects.keys().is_empty(), "{root}");
            assert_eq!(
                rows.stored_annotations(TENANT, EVENT, PUBLICATION),
                Some(annotations),
                "{root}"
            );
        }
    }

    #[tokio::test]
    async fn a_files_annotation_that_is_not_text_is_replaced_by_a_new_root() {
        let rows = publication(PublicationRecord {
            annotations: Some(json!({"ballot_files_v1": {"root": OTHER_ROOT}})),
            ..two_elections(consistent_styles())
        });
        let objects = MemoryPublicationObjects::default();
        prepare(&rows, &objects, &SequentialIds::default())
            .await
            .unwrap();

        let annotations = rows.stored_annotations(TENANT, EVENT, PUBLICATION).unwrap();
        assert_eq!(annotations["ballot_files_v1"], ROOT);
        assert_eq!(objects.keys().len(), 7);
    }

    #[tokio::test]
    async fn a_retry_after_a_failed_attempt_writes_under_a_new_root() {
        let ids = SequentialIds::default();
        let rows = publication(two_elections(consistent_styles()));
        let objects = MemoryPublicationObjects::failing_at(format!("{ROOT}/event.json"));
        let error = prepare(&rows, &objects, &ids).await.unwrap_err();
        assert_eq!(error.to_string(), "upload rejected");
        // The root is recorded only after its last object.
        assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
        let failed_attempt = objects.keys();
        assert_eq!(failed_attempt.len(), 6);

        prepare(&rows, &objects, &ids).await.unwrap();
        assert_eq!(
            rows.stored_annotations(TENANT, EVENT, PUBLICATION),
            files_annotation(RETRY_ROOT)
        );
        let keys = objects.keys();
        assert_eq!(keys[..6], failed_attempt[..]);
        assert_eq!(keys.len(), 13);
        assert!(keys[6..]
            .iter()
            .all(|key| key.starts_with(&format!("{RETRY_ROOT}/"))));
    }

    #[tokio::test]
    async fn different_event_presentations_in_one_publication_fail_before_the_event_object() {
        let other = r#"{"language_conf":{"enabled_language_codes":["en"]}}"#;
        let rows = publication(two_elections(vec![
            style_row(STYLE, ELECTION, &eml(PRESENTATION)),
            style_row(OTHER_STYLE, OTHER_ELECTION, &eml(other)),
        ]));
        let objects = MemoryPublicationObjects::default();
        let error = prepare(&rows, &objects, &SequentialIds::default())
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Inconsistent event presentation within publication"
        );
        let mut written = election_keys();
        written.extend(keys(
            ROOT,
            &[
                format!("summary-{STYLE}.json"),
                format!("style-{STYLE}.json"),
            ],
        ));
        assert_eq!(objects.keys(), written);
        assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
    }

    #[tokio::test]
    async fn rows_without_their_identifiers_or_eml_fail_without_recording_a_root() {
        for (elections, styles, error) in [
            (
                vec![json!({"presentation": null})],
                vec![],
                "Missing election id",
            ),
            (
                vec![],
                vec![json!({"ballot_eml": eml(PRESENTATION)})],
                "Missing style id",
            ),
            (vec![], vec![json!({"id": STYLE})], "Missing ballot EML"),
            (
                vec![],
                vec![json!({"id": STYLE, "ballot_eml": {"contests": []}})],
                "Missing ballot EML",
            ),
        ] {
            let rows = publication(PublicationRecord {
                elections,
                styles,
                ..PublicationRecord::default()
            });
            let objects = MemoryPublicationObjects::default();
            let result = prepare(&rows, &objects, &SequentialIds::default()).await;

            assert_eq!(result.unwrap_err().to_string(), error);
            assert!(objects.keys().is_empty(), "{error}");
            assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
        }
    }

    #[tokio::test]
    async fn a_style_with_malformed_eml_fails_after_the_election_objects() {
        for malformed in ["invalid", r#""contests""#] {
            let rows = publication(two_elections(vec![style_row(STYLE, ELECTION, malformed)]));
            let objects = MemoryPublicationObjects::default();
            let error = prepare(&rows, &objects, &SequentialIds::default())
                .await
                .unwrap_err();

            assert!(error.is::<serde_json::Error>(), "{malformed}: {error}");
            assert_eq!(objects.keys(), election_keys(), "{malformed}");
            assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
        }
    }

    #[tokio::test]
    async fn a_failure_while_streaming_styles_records_no_root() {
        let rows = publication(PublicationRecord {
            styles_fail_after: Some(1),
            ..two_elections(consistent_styles())
        });
        let objects = MemoryPublicationObjects::default();
        let error = prepare(&rows, &objects, &SequentialIds::default())
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "style row stream failed");
        let mut written = election_keys();
        written.extend(keys(
            ROOT,
            &[
                format!("summary-{STYLE}.json"),
                format!("style-{STYLE}.json"),
            ],
        ));
        assert_eq!(objects.keys(), written);
        assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
    }

    #[tokio::test]
    async fn an_unavailable_bucket_leaves_a_new_publication_unprepared() {
        let rows = publication(two_elections(consistent_styles()));
        let error = prepare(
            &rows,
            &MemoryPublicationObjects::unavailable(),
            &SequentialIds::default(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.to_string(), "private bucket unavailable");
        assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
    }

    #[tokio::test]
    async fn malformed_publication_identifiers_are_rejected() {
        let rows = publication(two_elections(consistent_styles()));
        let objects = MemoryPublicationObjects::default();
        let (tenant, event, id) = (
            TENANT.to_string(),
            EVENT.to_string(),
            PUBLICATION.to_string(),
        );
        for (tenant, event, id) in [
            ("tenant", event.as_str(), id.as_str()),
            (tenant.as_str(), "event", id.as_str()),
            (tenant.as_str(), event.as_str(), "publication"),
        ] {
            let result = prepare_files(
                &rows,
                &objects,
                &SequentialIds::default(),
                tenant,
                event,
                id,
            )
            .await;
            let error = result.unwrap_err();
            assert!(error.is::<uuid::Error>(), "{error}");
        }
        assert!(objects.keys().is_empty());
        assert_eq!(rows.stored_annotations(TENANT, EVENT, PUBLICATION), None);
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

    async fn files(
        rows: &MemoryPublicationRows,
        objects: &MemoryPublicationObjects,
        elections: &[Uuid],
    ) -> Result<Value> {
        let elections: Vec<String> = elections.iter().map(Uuid::to_string).collect();
        list_voter_files(
            rows,
            objects,
            &TENANT.to_string(),
            &EVENT.to_string(),
            &AREA.to_string(),
            &elections,
        )
        .await
    }

    /// The object and lifetime a fake presigned URL grants, without its signature.
    fn grant(url: &Value) -> &str {
        let url = url.as_str().unwrap();
        &url[..url.find("&signature=").unwrap()]
    }

    #[tokio::test]
    async fn voters_get_their_styles_with_the_live_election_policy_and_no_eml() {
        let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
        let mut response = files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
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
        let response = files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
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
        let response = files(
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
        let response = files(
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

        let response = files(&rows, &objects, &[OTHER_ELECTION]).await.unwrap();
        assert_eq!(response["files"].as_array().unwrap().len(), 1);
        assert_eq!(response["files"][0]["id"], json!(OTHER_STYLE));
        assert_eq!(
            files(&rows, &objects, &[]).await.unwrap(),
            json!({"event_id": EVENT, "status": {"is_published": true}, "files": []})
        );
    }

    #[tokio::test]
    async fn a_second_active_style_for_one_election_is_rejected() {
        let rows = voter_rows(vec![
            published(STYLE, ELECTION, PUBLICATION, Some(ROOT)),
            published(OTHER_STYLE, ELECTION, OTHER_PUBLICATION, Some(OTHER_ROOT)),
        ]);
        let error = files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
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
        let error = files(&rows, &MemoryPublicationObjects::default(), &[ELECTION])
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Publication requires S3 preparation; publish it again before serving voters"
        );
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
            let result = files(&rows, &MemoryPublicationObjects::default(), &[ELECTION]).await;
            assert_eq!(result.unwrap_err().to_string(), error, "{root}");
        }
    }

    #[tokio::test]
    async fn malformed_voter_identifiers_are_rejected() {
        let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
        let objects = MemoryPublicationObjects::default();
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
            let result = list_voter_files(&rows, &objects, tenant, event, area, &elections).await;
            let error = result.unwrap_err();
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
        let error = files(&rows, &MemoryPublicationObjects::unavailable(), &[ELECTION])
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Election event not found");
    }

    #[tokio::test]
    async fn a_url_that_cannot_be_signed_fails_the_whole_response() {
        let rows = voter_rows(vec![published(STYLE, ELECTION, PUBLICATION, Some(ROOT))]);
        let error = files(
            &rows,
            &MemoryPublicationObjects::failing_presign(),
            &[ELECTION],
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "presigning failed");
    }
}
