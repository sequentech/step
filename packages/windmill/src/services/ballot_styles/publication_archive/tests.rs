// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use deadpool_postgres::Pool;

#[test]
fn rejects_unknown_archive_versions() {
    assert!(read_archive(br#"{"version":99,"publications":[]}"#).is_err());
}

#[test]
fn remaps_embedded_eml_and_object_keys_without_changing_other_values() {
    let old = Uuid::new_v4().to_string();
    let new = Uuid::new_v4().to_string();
    let ids = HashMap::from([(old.clone(), new.clone())]);
    let mut value = json!({old.clone(): {"ballot_eml":format!(r#"{{"id":"{old}","votes":7}}"#),"key":"untouched","enabled":true}});
    remap(&mut value, &ids);
    assert_eq!(
        value[&new]["ballot_eml"],
        format!(r#"{{"id":"{new}","votes":7}}"#)
    );
    assert_eq!(value[&new]["key"], "untouched");
    assert_eq!(value[&new]["enabled"], true);
    assert!(value.get(old).is_none());
}

/// Called by the disposable PostgreSQL/S3 integration, using its real uploaded files.
pub(crate) async fn round_trip(
    pool: &Pool,
    tenant: &str,
    event: &str,
    source_publication: &str,
) -> Result<()> {
    let (archive, paths) = {
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        let exported = export_publication_archive(&tx, tenant, event).await?;
        tx.commit().await?;
        exported
    };
    let manifest = serde_json::to_vec(&archive)?;
    let entries = paths
        .iter()
        .map(|(name, path)| Ok((name.clone(), std::fs::read(path)?)))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        !entries.is_empty(),
        "Export omitted private publication files"
    );
    let target_tenant = Uuid::new_v4();
    let target_event = Uuid::new_v4();
    let t = target_tenant.to_string();
    let ev = target_event.to_string();
    let mut ids = HashMap::from([
        (tenant.to_owned(), t.clone()),
        (event.to_owned(), ev.clone()),
    ]);
    let result: Result<()> = async {
        let mut db = pool.get().await?;
        db.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,presentation,description,status) VALUES ($1,$2,'{}','imported','{}')", &[&target_event,&target_tenant]).await?;
        db.execute("UPDATE sequent_backend.election_event SET status=$2 WHERE id=$1", &[&target_event, &json!({"is_published":false,"voting_status":"NOT_STARTED","test_marker":7})]).await?;
        for row in db.query("SELECT id,to_jsonb(e) FROM sequent_backend.election e WHERE tenant_id=$1 AND election_event_id=$2", &[&Uuid::parse_str(tenant)?,&Uuid::parse_str(event)?]).await? {
            let original: Uuid = row.get(0);
            let mut data: Value = row.get(1);
            let id = Uuid::new_v4();
            ids.insert(original.to_string(),id.to_string());
            data["id"] = json!(id); data["tenant_id"] = json!(target_tenant); data["election_event_id"] = json!(target_event);
            db.execute("INSERT INTO sequent_backend.election SELECT * FROM jsonb_populate_record(NULL::sequent_backend.election,$1)", &[&data]).await?;
        }
        for row in db.query("SELECT DISTINCT area_id FROM sequent_backend.ballot_style WHERE tenant_id=$1 AND election_event_id=$2", &[&Uuid::parse_str(tenant)?,&Uuid::parse_str(event)?]).await? {
            let id: Uuid = row.get(0); ids.insert(id.to_string(),Uuid::new_v4().to_string());
        }
        drop(db);
        {
            let mut db = pool.get().await?;
            let tx = db.transaction().await?;
            ensure!(import_publication_archive(&tx,&t,&ev,&manifest,&entries[1..],&ids).await.is_err(), "Missing S3 file was silently ignored");
            ensure!(tx.query_one("SELECT count(*) FROM sequent_backend.ballot_publication WHERE election_event_id=$1", &[&target_event]).await?.get::<_,i64>(0)==0, "Incomplete archive created publication rows");
            ensure!(import_publication_archive(&tx,&Uuid::new_v4().to_string(),&ev,&manifest,&entries,&ids).await.is_err(), "Foreign tenant mapping accepted");
            tx.rollback().await?;
        }
        {
            let mut db = pool.get().await?;
            let tx = db.transaction().await?;
            import_publication_archive(&tx,&t,&ev,&manifest,&entries,&ids).await?;
            tx.commit().await?;
        }
        let mut db = pool.get().await?;
        let row = db.query_one("SELECT id,published_at IS NOT NULL,annotations->>'ballot_files_v1' FROM sequent_backend.ballot_publication WHERE election_event_id=$1 AND is_generated", &[&target_event]).await?;
        let status: Value = db.query_one("SELECT status FROM sequent_backend.election_event WHERE id=$1", &[&target_event]).await?.get(0);
        ensure!(status == json!({"is_published":true,"voting_status":"NOT_STARTED","test_marker":7}), "Publication import did not preserve event status while restoring its published flag");
        let imported_id: Uuid = row.get(0);
        ensure!(row.get::<_,bool>(1), "Publication lost its published status");
        ensure!(imported_id.to_string()!=source_publication, "Publication id was not remapped");
        let root: String = row.get(2);
        ensure!(root.starts_with(&format!("tenant-{t}/event-{ev}/publication-{imported_id}/")), "Imported files point outside the imported event");
        let styles = db.query("SELECT id,ballot_eml,area_id,election_id FROM sequent_backend.ballot_style WHERE ballot_publication_id=$1", &[&imported_id]).await?;
        let client = get_shared_s3_client(S3Endpoint::Server).await?;
        let bucket = get_private_bucket()?;
        let event_file = get_object_into_temp_file(&bucket,&format!("{root}/event.json"),"event",".json").await?;
        let event_data: Value = serde_json::from_slice(&std::fs::read(event_file.path())?)?;
        ensure!(event_data["id"]==ev, "Event file id was not remapped");
        for row in styles {
            let id: Uuid = row.get(0);
            let eml: String = row.get(1);
            let data: Value = serde_json::from_str(&eml)?;
            ensure!(data.get("election_event_id").is_none_or(|id| *id==ev) && data.get("tenant_id").is_none_or(|id| *id==t), "Stored ballot EML was not remapped");
            let file = get_object_into_temp_file(&bucket,&format!("{root}/style-{id}.json"),"style",".json").await?;
            let wire: Value = serde_json::from_slice(&std::fs::read(file.path())?)?;
            let reconstructed = wire["ballot_eml"].as_str().map(str::to_owned).unwrap_or_else(||format!("{}{}{}",wire["ballot_eml_prefix"].as_str().unwrap(),event_data["ballot_eml_presentation"].as_str().unwrap(),wire["ballot_eml_suffix"].as_str().unwrap()));
            ensure!(reconstructed==eml, "S3 ballot differs from imported database ballot");
            let area: Uuid = row.get(2);
            let election: Uuid = row.get(3);
            let tx = db.transaction().await?;
            let references = super::super::publication_files::load_voter_files(&tx,&t,&ev,&area.to_string(),&[election.to_string()]).await?;
            tx.rollback().await?;
            let response = super::super::publication_files::presign_voter_files(references).await?;
            ensure!(response["files"].as_array().is_some_and(|files| !files.is_empty()), "Imported publication has no authorized voter files");
        }
        drop(db);
        let source = archive.publications.iter().find(|p| p.publication.id==source_publication).context("Source publication missing")?;
        let legacy = json!([{"ballot_publication_id":source_publication,"ballot_styles": source.ballot_styles.iter().map(|s| -> Result<Value> {
            let mut eml: Value = serde_json::from_str(s.ballot_eml.as_deref().unwrap())?;
            eml["id"]=json!(s.id); eml["tenant_id"]=json!(s.tenant_id); eml["election_event_id"]=json!(s.election_event_id); eml["election_id"]=json!(s.election_id); eml["area_id"]=json!(s.area_id);
            Ok(eml)
        }).collect::<Result<Vec<_>>>()?}]);
        {
            let mut db = pool.get().await?;
            let tx = db.transaction().await?;
            tx.execute("UPDATE sequent_backend.election_event SET status=jsonb_set(status,'{is_published}','false') WHERE id=$1", &[&target_event]).await?;
            import_publication_archive(&tx,&t,&ev,&serde_json::to_vec(&legacy)?,&[],&ids).await?;
            let status: Value = tx.query_one("SELECT status FROM sequent_backend.election_event WHERE id=$1", &[&target_event]).await?.get(0);
            ensure!(status["is_published"] == false, "Legacy draft import incorrectly published the event");
            tx.commit().await?;
        }
        ensure!(pool.get().await?.query_one("SELECT count(*) FROM sequent_backend.ballot_publication WHERE election_event_id=$1 AND is_generated AND published_at IS NULL AND annotations ? 'ballot_files_v1'", &[&target_event]).await?.get::<_,i64>(0)==1, "Legacy archive did not restore a generated draft with S3 files");
        drop(client);
        Ok(())
    }.await;
    let cleanup = sequent_core::services::s3::delete_files_from_s3(
        get_private_bucket()?,
        format!("tenant-{t}/event-{ev}/"),
        S3Endpoint::Server,
    )
    .await;
    result?;
    cleanup?;
    Ok(())
}
