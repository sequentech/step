// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Public-only backfill, publication snapshots, and preserved archive identity.
#[path = "support/schema.rs"]
mod schema;
use anyhow::Result;
use electoral_log::messages::message::SigningData;
use electoral_log::messages::newtypes::ElectoralLogCheckpointReason;
use electoral_log::ports::ElectoralLogStore;
use sequent_core::types::hasura::core::ElectionEvent;
use serde_json::json;
use uuid::Uuid;
use voting_crypto::signing::{SigningKey, VerificationKey};
use windmill::services::{ballot_box_key, electoral_log_board, public_ballot_keys as keys};

async fn event(
    tx: &deadpool_postgres::Transaction<'_>,
    tenant: &str,
) -> Result<(ElectionEvent, String)> {
    let event = Uuid::new_v4().to_string();
    let board = electoral_log_board::get_event_board(tenant, &event, &std::env::var("ENV_SLUG")?);
    tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,bulletin_board_reference,presentation) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3,$4)",&[&event,&tenant,&json!({"id":0,"database_name":board,"is_archived":false}),&json!({"receipts":{"policy":"signed-by-ballot-box"}})]).await?;
    let store = electoral_log_board::get_event_databases()
        .await?
        .create_event(tenant, &event)
        .await?;
    store.create_board(&board).await?;
    store.create_ballot_box(&event).await?;
    Ok((
        windmill::postgres::election_event::get_election_event_by_id(tx, tenant, &event).await?,
        board,
    ))
}

#[tokio::test]
#[ignore = "isolated migrated PostgreSQL and synthetic EnvVarMasterSecret only"]
async fn public_backfill_needs_no_private_secret_and_archives_preserve_source_identity(
) -> Result<()> {
    assert_eq!(std::env::var("SECRETS_BACKEND")?, "EnvVarMasterSecret");
    assert_eq!(std::env::var("MASTER_SECRET")?, "11".repeat(32));
    let pool = schema::pool().await;
    let mut db = pool.get().await?;
    let database: String = db.query_one("SELECT current_database()", &[]).await?.get(0);
    assert!(database.starts_with("windmill_schema_"));
    let tx = db.transaction().await?;
    let tenant = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1::text::uuid,$1)",
        &[&tenant],
    )
    .await?;
    let (event, board) = event(&tx, &tenant).await?;
    // These fixture keys are never saved in the vault. Recovery has public sources only.
    let signing = SigningKey::generate()?;
    let receipt = ballot_box_key::published_key(&SigningKey::generate()?)?;
    windmill::postgres::trusted_write(&tx).await?;
    let election = Uuid::new_v4();
    let area = Uuid::new_v4();
    let publication = Uuid::new_v4();
    tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id) VALUES($1,$2::text::uuid,$3::text::uuid)",&[&election,&tenant,&event.id]).await?;
    tx.execute("INSERT INTO sequent_backend.area(id,tenant_id,election_event_id,name) VALUES($1,$2::text::uuid,$3::text::uuid,'Synthetic public key fixture')",&[&area,&tenant,&event.id]).await?;
    tx.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,is_generated,published_at) VALUES($1,$2::text::uuid,$3::text::uuid,true,now())",&[&publication,&tenant,&event.id]).await?;
    tx.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES($1,$2::text::uuid,$3::text::uuid,$4,$5,$6,$7)",&[&Uuid::new_v4(),&tenant,&event.id,&election,&area,&publication,&json!({"ballot_box_key":receipt}).to_string()]).await?;
    let store = electoral_log_board::get_event_store(&event.id).await?;
    let checkpoint = store.journal().checkpoint(&board).await?;
    let signed = windmill::services::electoral_log::sign_checkpoint(
        &SigningData::new(signing.clone(), "system", signing.clone()),
        &checkpoint,
        ElectoralLogCheckpointReason::VotingClosed,
    )?;
    windmill::postgres::electoral_log_checkpoint::insert_electoral_log_checkpoint(
        &tx, &tenant, &event.id, &signed,
    )
    .await?;
    assert!(ballot_box_key::get_ballot_box_signing_key(&tx, &event)
        .await?
        .is_none());
    assert_eq!(
        keys::backfill(&tx, &event, keys::BackfillSource::PublishedOnly).await?,
        1
    );
    let retained = keys::list(&tx, &tenant, &event.id).await?;
    assert_eq!(retained.len(), 1);
    assert_eq!(
        retained[0].system_public_key,
        VerificationKey::from_sk(&signing)?.to_b64_string()?
    );
    assert_eq!(retained[0].ballot_box_public_key, receipt.public_key);
    assert_eq!(
        keys::backfill(&tx, &event, keys::BackfillSource::PublishedOnly).await?,
        1
    );
    let mut wrong = retained[0].clone();
    wrong.system_public_key =
        VerificationKey::from_sk(&SigningKey::generate()?)?.to_b64_string()?;
    assert!(keys::retain(&tx, &tenant, &event.id, &wrong)
        .await
        .unwrap_err()
        .to_string()
        .contains("different public verification"));

    let (records, manifest) =
        windmill::services::electoral_log_transfer::export_electoral_log(&tx, &tenant, &event.id)
            .await?;
    assert_eq!(manifest.ballot_verification_keys, retained);
    let mut legacy = serde_json::to_value(&manifest)?;
    legacy
        .as_object_mut()
        .unwrap()
        .remove("ballot_verification_keys");
    assert!(
        serde_json::from_value::<electoral_log::adapters::transfer::ExportManifest>(legacy)?
            .ballot_verification_keys
            .is_empty()
    );
    let (clone, clone_board) = self::event(&tx, &tenant).await?;
    electoral_log_board::create_protocol_manager_keys(&tx, &tenant, &clone.id, &clone_board)
        .await?;
    windmill::services::electoral_log_transfer::import_electoral_log(
        &tx,
        &tenant,
        &clone.id,
        &clone_board,
        &serde_json::to_vec(&manifest)?,
        std::io::BufReader::new(records.reopen()?),
    )
    .await?;
    let imported = keys::list(&tx, &tenant, &clone.id).await?;
    assert_eq!(imported, retained);
    assert!(
        imported.iter().all(|key| key.election_event_id != clone.id),
        "a clone must not adopt historical authority as its own"
    );

    let (compat, compat_board) = self::event(&tx, &tenant).await?;
    electoral_log_board::create_protocol_manager_keys(&tx, &tenant, &compat.id, &compat_board)
        .await?;
    let compat_receipt = ballot_box_key::ballot_box_key_for_publication(&tx, &compat)
        .await?
        .unwrap();
    assert!(
        keys::backfill(&tx, &compat, keys::BackfillSource::PublishedOnly)
            .await
            .unwrap_err()
            .to_string()
            .contains("No retained published receipt key")
    );
    assert_eq!(
        keys::backfill(&tx, &compat, keys::BackfillSource::ExistingSigningKey).await?,
        1
    );
    keys::for_publication(&tx, &compat, &compat_receipt).await?;
    assert_eq!(keys::list(&tx, &tenant, &compat.id).await?.len(), 1);
    let mut invalid = retained[0].clone();
    invalid.board_name = compat_board;
    assert!(keys::validate(&invalid).is_err());
    let mut invalid = retained[0].clone();
    invalid.ballot_box_key_id = "wrong".into();
    assert!(keys::validate(&invalid).is_err());
    for event in [event, clone, compat] {
        electoral_log_board::get_event_databases()
            .await?
            .drop_event(&tenant, &event.id)
            .await?;
    }
    Ok(())
}
