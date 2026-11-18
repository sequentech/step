// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Boards of an event whose electoral log an external signer signs, against
//! the migrated schema: the key is made by the installed signer and recorded
//! (also after a failed or rolled-back attempt, through the same path the
//! event import and creation take), an existing key is never replaced, a
//! keyless board is an error rather than one to skip, a record is used only
//! for the board it was made for, the setting can't
//! change once a key exists, and an export's key record is restored only for
//! the board and key it was made for. Software events keep today's
//! behaviour.
//!
//! The signer here is an in-memory stand-in: that the real signer is
//! idempotent is shown with the installation's own adapter, not here.
#[path = "support/schema.rs"]
mod schema;

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{anyhow, Result};
use serde_json::json;
use uuid::Uuid;
use voting_crypto::signing::{SigningKey, VerificationKey};
use windmill::services::electoral_log_board as boards;
use windmill::services::electoral_log_signers::{self as signers, BoardKey, BoardOrigin};

/// (event, board): the tests share one installed signer, so every switch
/// is per board of an event.
type Board = (String, String);

/// An in-memory stand-in for an installation's signer: one key per board,
/// made once, addressed by an opaque reference.
#[derive(Default)]
struct FakeSigners {
    keys: Mutex<HashMap<(String, String, String), (String, SigningKey)>>,
    /// Boards for which it answers with another board's origin.
    lie_about: Mutex<HashSet<Board>>,
    /// Boards whose next creation fails (once).
    fail_once: Mutex<HashSet<Board>>,
}

impl signers::ExternalLogSigners for FakeSigners {
    fn create_board_signer(&self, tenant: &str, event: &str, board: &str) -> Result<BoardKey> {
        let id: Board = (event.into(), board.into());
        if self.fail_once.lock().unwrap().remove(&id) {
            return Err(anyhow!("the signer is unreachable"));
        }
        let mut keys = self.keys.lock().unwrap();
        let (key_ref, key) = keys
            .entry((tenant.into(), event.into(), board.into()))
            .or_insert_with(|| {
                (
                    format!("ref-{}", Uuid::new_v4()),
                    SigningKey::generate().unwrap(),
                )
            })
            .clone();
        let mut origin = BoardOrigin {
            tenant_id: tenant.into(),
            election_event_id: event.into(),
            board_name: board.into(),
        };
        if self.lie_about.lock().unwrap().contains(&id) {
            origin.board_name = format!("{board}-other");
        }
        Ok(BoardKey {
            key_ref,
            public_key: VerificationKey::from_sk(&key)?.to_b64_string()?,
            key_version: 1,
            origin,
        })
    }

    fn board_signer(
        &self,
        tenant: &str,
        event: &str,
        board: &str,
        key: &BoardKey,
    ) -> Result<electoral_log::signer::SharedSigner> {
        let keys = self.keys.lock().unwrap();
        let (key_ref, signing) = keys
            .get(&(tenant.into(), event.into(), board.into()))
            .ok_or_else(|| anyhow!("no such key"))?;
        if *key_ref != key.key_ref || key.key_version != 1 {
            return Err(anyhow!("not this key (refused)"));
        }
        Ok(electoral_log::signer::software(signing.clone()))
    }
}

fn fake() -> Arc<FakeSigners> {
    static FAKE: OnceLock<Arc<FakeSigners>> = OnceLock::new();
    FAKE.get_or_init(|| {
        let fake = Arc::new(FakeSigners::default());
        signers::install(fake.clone()).unwrap();
        fake
    })
    .clone()
}

async fn tenant(tx: &deadpool_postgres::Transaction<'_>) -> Result<String> {
    let tenant = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1::text::uuid,$1)",
        &[&tenant],
    )
    .await?;
    Ok(tenant)
}

async fn event(
    tx: &deadpool_postgres::Transaction<'_>,
    tenant: &str,
    presentation: serde_json::Value,
) -> Result<String> {
    let event = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3)",
        &[&event, &tenant, &presentation],
    )
    .await?;
    Ok(event)
}

/// A tenant and an event, committed (so a failed attempt's rollback keeps them).
async fn committed_event(
    db: &mut deadpool_postgres::Object,
    presentation: serde_json::Value,
) -> Result<(String, String)> {
    let tx = db.transaction().await?;
    let tenant_id = tenant(&tx).await?;
    let event_id = event(&tx, &tenant_id, presentation).await?;
    tx.commit().await?;
    Ok((tenant_id, event_id))
}

fn external() -> serde_json::Value {
    json!({"electoral_log_signer_policy": "external"})
}

fn check_env() -> Result<()> {
    assert_eq!(std::env::var("SECRETS_BACKEND")?, "EnvVarMasterSecret");
    assert_eq!(std::env::var("MASTER_SECRET")?, "11".repeat(32));
    Ok(())
}

fn event_board(tenant_id: &str, event_id: &str) -> Result<String> {
    Ok(boards::get_event_board(
        tenant_id,
        event_id,
        &std::env::var("ENV_SLUG")?,
    ))
}

async fn external_record(
    tx: &deadpool_postgres::Transaction<'_>,
    tenant_id: &str,
    event_id: &str,
    board: &str,
) -> Result<BoardKey> {
    match boards::get_board_identity(tx, tenant_id, Some(event_id), board).await? {
        boards::BoardIdentity::External(key) => Ok(key),
        boards::BoardIdentity::Software(_) => Err(anyhow!("a software key")),
    }
}

#[tokio::test]
#[ignore = "isolated migrated PostgreSQL and synthetic EnvVarMasterSecret only"]
async fn a_failed_creation_is_repaired_by_the_next_audit_board_setup() -> Result<()> {
    check_env()?;
    let fake = fake();
    let pool = schema::pool().await;
    let mut db = pool.get().await?;
    let (tenant_id, event_id) = committed_event(&mut db, external()).await?;
    let board = event_board(&tenant_id, &event_id)?;

    // First setup: the board is made, the signer fails, nothing is recorded.
    fake.fail_once
        .lock()
        .unwrap()
        .insert((event_id.clone(), board.clone()));
    {
        let tx = db.transaction().await?;
        let error =
            windmill::services::import::import_election_event::ensure_election_audit_boards(
                &tx,
                &tenant_id,
                &event_id,
                &Vec::new(),
                false,
            )
            .await
            .err()
            .expect("the signer's failure fails the setup");
        assert!(format!("{error:#}").contains("unreachable"), "{error:#}");
        tx.rollback().await?;
    }
    {
        let tx = db.transaction().await?;
        assert!(!boards::board_has_key(&tx, &tenant_id, &event_id, &board).await?);
        let error = boards::get_board_signer_if_any(&tx, &tenant_id, &event_id, &board)
            .await
            .err()
            .expect("the keyless external board is refused, never skipped");
        assert!(error.to_string().contains("no key"), "{error:#}");
        tx.rollback().await?;
    }

    // Second setup: the board exists already; its key is made now.
    let tx = db.transaction().await?;
    windmill::services::import::import_election_event::ensure_election_audit_boards(
        &tx,
        &tenant_id,
        &event_id,
        &Vec::new(),
        false,
    )
    .await?;
    assert!(boards::board_has_key(&tx, &tenant_id, &event_id, &board).await?);
    let key = external_record(&tx, &tenant_id, &event_id, &board).await?;
    // A third setup changes nothing: the key is kept.
    windmill::services::import::import_election_event::ensure_election_audit_boards(
        &tx,
        &tenant_id,
        &event_id,
        &Vec::new(),
        false,
    )
    .await?;
    assert_eq!(
        external_record(&tx, &tenant_id, &event_id, &board).await?,
        key
    );
    tx.commit().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "isolated migrated PostgreSQL and synthetic EnvVarMasterSecret only"]
async fn an_external_board_is_made_once_signed_by_its_key_and_never_replaced() -> Result<()> {
    check_env()?;
    let fake = fake();
    let pool = schema::pool().await;
    let mut db = pool.get().await?;
    let (tenant_id, event_id) = committed_event(&mut db, external()).await?;
    let board = "board-a";

    // Made inside a transaction that rolls back: the signer's key exists,
    // the record doesn't.
    {
        let tx = db.transaction().await?;
        boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, board).await?;
        assert!(boards::board_has_key(&tx, &tenant_id, &event_id, board).await?);
        tx.rollback().await?;
    }
    let first_ref = fake.keys.lock().unwrap()
        [&(tenant_id.clone(), event_id.clone(), board.to_string())]
        .0
        .clone();

    let tx = db.transaction().await?;
    assert!(!boards::board_has_key(&tx, &tenant_id, &event_id, board).await?);
    // The repeated creation gets the same key and records it.
    boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, board).await?;
    let key = external_record(&tx, &tenant_id, &event_id, board).await?;
    assert_eq!(key.key_ref, first_ref, "the retry made no other key");
    assert_eq!(
        key.origin,
        BoardOrigin {
            tenant_id: tenant_id.clone(),
            election_event_id: event_id.clone(),
            board_name: board.into()
        }
    );

    // An existing record is never replaced.
    let error = boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, board)
        .await
        .err()
        .expect("a board with a key is refused");
    assert!(error.to_string().contains("never replaced"), "{error:#}");
    assert_eq!(
        external_record(&tx, &tenant_id, &event_id, board).await?,
        key
    );

    // It signs with the recorded key, and no software key exists.
    let signer = boards::get_board_signer(&tx, &tenant_id, Some(&event_id), board).await?;
    let signature = signer.sign(b"entry")?;
    key.verification_key()?.verify(&signature, b"entry")?;
    assert!(
        boards::get_board_signing_key(&tx, &tenant_id, Some(&event_id), board)
            .await
            .is_err()
    );
    assert!(
        boards::get_board_signer_if_any(&tx, &tenant_id, &event_id, board)
            .await?
            .is_some()
    );

    // A signer answering with another board's key is refused.
    fake.lie_about
        .lock()
        .unwrap()
        .insert((event_id.clone(), "board-b".into()));
    let error = boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, "board-b")
        .await
        .err()
        .expect("a key made for another board is refused");
    assert!(format!("{error:#}").contains("another board"), "{error:#}");

    // A record moved to another board's place in the vault is refused when
    // used, for signing and for verification alike.
    tx.execute(
        "UPDATE sequent_backend.secret SET key = 'boards/board-c/external-signer' \
         WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid \
         AND key = 'boards/board-a/external-signer'",
        &[&tenant_id, &event_id],
    )
    .await?;
    for error in [
        boards::get_board_signer(&tx, &tenant_id, Some(&event_id), "board-c")
            .await
            .err(),
        boards::get_board_public_key(&tx, &tenant_id, Some(&event_id), "board-c")
            .await
            .err(),
    ] {
        let error = error.expect("a moved record is refused");
        assert!(format!("{error:#}").contains("another board"), "{error:#}");
    }
    tx.execute(
        "UPDATE sequent_backend.secret SET key = 'boards/board-a/external-signer' \
         WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid \
         AND key = 'boards/board-c/external-signer'",
        &[&tenant_id, &event_id],
    )
    .await?;
    tx.commit().await?;

    // The database refuses changing the setting now that a key exists
    // (the migration's trigger, on the migrated schema).
    let tx = db.transaction().await?;
    let error = tx
        .execute(
            "UPDATE sequent_backend.election_event SET presentation = '{}'::jsonb WHERE id = $1::text::uuid",
            &[&event_id],
        )
        .await
        .err()
        .expect("the setting can't change once a board has a key");
    assert!(format!("{error:?}").contains("can't change"), "{error:?}");
    tx.rollback().await?;
    let tx = db.transaction().await?;
    tx.execute(
        "UPDATE sequent_backend.election_event SET presentation = $2 WHERE id = $1::text::uuid",
        &[
            &event_id,
            &json!({"electoral_log_signer_policy": "external", "unrelated": true}),
        ],
    )
    .await?;
    tx.rollback().await?;
    Ok(())
}

/// The export's record, changed by `change`.
fn altered(exported: &str, change: impl FnOnce(&mut BoardKey)) -> Result<String> {
    let record = exported
        .strip_prefix(boards::EXTERNAL_KEY_EXPORT_PREFIX)
        .ok_or_else(|| anyhow!("not an external record"))?;
    let mut key: BoardKey = serde_json::from_str(record)?;
    change(&mut key);
    Ok(format!(
        "{}{}",
        boards::EXTERNAL_KEY_EXPORT_PREFIX,
        serde_json::to_string(&key)?
    ))
}

#[tokio::test]
#[ignore = "isolated migrated PostgreSQL and synthetic EnvVarMasterSecret only"]
async fn an_exported_external_key_restores_only_to_its_own_board_and_key() -> Result<()> {
    check_env()?;
    fake();
    let pool = schema::pool().await;
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    let tenant_id = tenant(&tx).await?;
    let event_id = event(&tx, &tenant_id, external()).await?;
    let board = "board-x";
    boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, board).await?;
    let exported = boards::export_board_key(&tx, &tenant_id, &event_id, board).await?;
    assert!(exported.starts_with(boards::EXTERNAL_KEY_EXPORT_PREFIX));
    assert!(
        !exported.contains("signing_key"),
        "no private key is exported"
    );

    // A copy of the event under other ids can't reuse the key.
    let copy = event(&tx, &tenant_id, external()).await?;
    let error = boards::import_board_key(&tx, &tenant_id, &copy, board, &exported)
        .await
        .err()
        .expect("a remapped copy is refused");
    assert!(format!("{error:#}").contains("another board"), "{error:#}");
    // Nor under another board name.
    let error = boards::import_board_key(&tx, &tenant_id, &event_id, "board-y", &exported)
        .await
        .err()
        .expect("another board name is refused");
    assert!(format!("{error:#}").contains("another board"), "{error:#}");
    // A software event refuses an external record, and an external event a
    // software key.
    let software_event = event(&tx, &tenant_id, json!({})).await?;
    assert!(
        boards::import_board_key(&tx, &tenant_id, &software_event, board, &exported)
            .await
            .is_err()
    );
    assert!(
        boards::import_board_key(&tx, &tenant_id, &copy, board, "signing_key = \"AAAA\"\n")
            .await
            .is_err()
    );

    tx.execute(
        "DELETE FROM sequent_backend.secret WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid",
        &[&tenant_id, &event_id],
    )
    .await?;
    // A record for its own board, but not the key the signer holds: another
    // public key, another version, another reference: all refused.
    let other_key = VerificationKey::from_sk(&SigningKey::generate()?)?.to_b64_string()?;
    for (what, record) in [
        (
            "public key",
            altered(&exported, |k| k.public_key = other_key.clone())?,
        ),
        ("version", altered(&exported, |k| k.key_version = 2)?),
        (
            "reference",
            altered(&exported, |k| k.key_ref = "ref-other".into())?,
        ),
    ] {
        assert!(
            boards::import_board_key(&tx, &tenant_id, &event_id, board, &record)
                .await
                .is_err(),
            "another {what} is refused"
        );
        assert!(!boards::board_has_key(&tx, &tenant_id, &event_id, board).await?);
    }

    // Restored to its own board as exported, it is accepted.
    boards::import_board_key(&tx, &tenant_id, &event_id, board, &exported).await?;
    assert!(boards::board_has_key(&tx, &tenant_id, &event_id, board).await?);
    boards::get_board_signer(&tx, &tenant_id, Some(&event_id), board).await?;
    tx.rollback().await?;
    Ok(())
}

/// The keys file of an event export, holding the event board's key.
fn keys_file(event_key: &str) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new()?;
    {
        let mut writer = csv::Writer::from_writer(&mut file);
        writer.write_record(["election_id", "key"])?;
        writer.write_record(["", event_key])?;
        writer.flush()?;
    }
    file.flush()?;
    Ok(tempfile::NamedTempFile::from_parts(
        file.reopen()?,
        file.into_temp_path(),
    ))
}

#[tokio::test]
#[ignore = "isolated migrated PostgreSQL and synthetic EnvVarMasterSecret only"]
async fn an_event_import_under_new_ids_refuses_the_external_key() -> Result<()> {
    check_env()?;
    fake();
    let pool = schema::pool().await;
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    let tenant_id = tenant(&tx).await?;
    let source = event(&tx, &tenant_id, external()).await?;
    let source_board = event_board(&tenant_id, &source)?;
    boards::create_protocol_manager_keys(&tx, &tenant_id, &source, &source_board).await?;
    let exported = boards::export_board_key(&tx, &tenant_id, &source, &source_board).await?;

    // The import path, with the event under new ids (a remapped copy).
    let copy = event(&tx, &tenant_id, external()).await?;
    let error = windmill::services::import::import_bulletin_boards::import_protocol_manager_keys(
        &tx,
        &tenant_id,
        &copy,
        keys_file(&exported)?,
        HashMap::from([(source.clone(), copy.clone())]),
        &[],
    )
    .await
    .err()
    .expect("the copy's import is refused");
    assert!(format!("{error:#}").contains("another board"), "{error:#}");
    let copy_board = event_board(&tenant_id, &copy)?;
    assert!(!boards::board_has_key(&tx, &tenant_id, &copy, &copy_board).await?);

    // The same file for the event it was made for is accepted.
    tx.execute(
        "DELETE FROM sequent_backend.secret WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid",
        &[&tenant_id, &source],
    )
    .await?;
    windmill::services::import::import_bulletin_boards::import_protocol_manager_keys(
        &tx,
        &tenant_id,
        &source,
        keys_file(&exported)?,
        HashMap::new(),
        &[],
    )
    .await?;
    assert!(boards::board_has_key(&tx, &tenant_id, &source, &source_board).await?);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "isolated migrated PostgreSQL and synthetic EnvVarMasterSecret only"]
async fn software_events_keep_todays_keys_and_skip_keyless_boards() -> Result<()> {
    check_env()?;
    fake();
    let pool = schema::pool().await;
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;
    let tenant_id = tenant(&tx).await?;
    let event_id = event(&tx, &tenant_id, json!({})).await?;
    // A keyless software board is skipped, as today.
    assert!(
        boards::get_board_signer_if_any(&tx, &tenant_id, &event_id, "b")
            .await?
            .is_none()
    );
    boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, "b").await?;
    let key = boards::get_board_signing_key(&tx, &tenant_id, Some(&event_id), "b").await?;
    let exported = boards::export_board_key(&tx, &tenant_id, &event_id, "b").await?;
    assert_eq!(
        boards::parse_signing_identity(&exported)?.to_b64_string()?,
        key.to_b64_string()?
    );
    // The software key made nothing in the external signer.
    assert!(!fake()
        .keys
        .lock()
        .unwrap()
        .keys()
        .any(|(_, event, _)| *event == event_id));
    // Nor is a software key replaced.
    assert!(
        boards::create_protocol_manager_keys(&tx, &tenant_id, &event_id, "b")
            .await
            .is_err()
    );
    tx.rollback().await?;
    Ok(())
}
