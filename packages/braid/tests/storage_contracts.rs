// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "native")]

use b4::{
    messages::{
        artifact::Configuration, message::Message, newtypes::CiphertextsHash,
        protocol_manager::ProtocolManager,
    },
    HttpB3Message,
};
use braid::{
    native::board::{NoOpStorage, SqliteStorage},
    protocol::board::local_storage::LocalBoardStorage,
};
use std::{fs, marker::PhantomData};
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::StrandSerialize,
    signature::{StrandSignaturePk, StrandSignatureSk},
};

fn message(id: i64, batch: u64, mix: usize) -> HttpB3Message {
    // Fixed synthetic Ed25519 seed, so duplicate identities are reproducible.
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0, 0x30, 5, 6, 3, 0x2b, 0x65, 0x70, 4, 0x22, 4, 0x20,
    ];
    der.extend([7; 32]);
    let signer = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::from_der(&der).unwrap());
    let pk = StrandSignaturePk::from_sk(&signer.signing_key).unwrap();
    der[16..].fill(8);
    let other = StrandSignaturePk::from_sk(&StrandSignatureSk::from_der(&der).unwrap()).unwrap();
    let cfg = Configuration::<RistrettoCtx>::new(42, pk.clone(), vec![pk, other], 2, PhantomData);
    let signed = Message::mix_signed_msg(
        &cfg,
        batch,
        CiphertextsHash([1; 64]),
        CiphertextsHash([2; 64]),
        mix,
        &signer,
    )
    .unwrap();
    HttpB3Message {
        id,
        message: signed.strand_serialize().unwrap(),
        version: b4::get_schema_version(),
        sender_pk: "untrusted-envelope-key".into(),
        statement_kind: "untrusted-envelope-kind".into(),
        batch: 0,
        mix_number: 0,
    }
}

#[test]
fn local_order_survives_restart_and_external_ids_cannot_reorder_messages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.sqlite");
    let store = SqliteStorage::new(path.clone(), None);
    assert_eq!(store.get_last_external_id().unwrap(), -1);
    let empty = store.get_storage_info().unwrap();
    assert_eq!(
        (
            empty.total_messages,
            empty.max_internal_id,
            empty.max_external_id
        ),
        (0, -1, -1)
    );
    store
        .store_messages(
            &[message(900, 1, 1), message(2, 2, 1), message(400, 2, 2)],
            false,
        )
        .unwrap();
    drop(store);
    let restarted = SqliteStorage::new(path, None);
    let rows = restarted.retrieve_messages(-1).unwrap();
    assert_eq!(
        rows.iter()
            .map(|(m, id)| (
                *id,
                m.statement.get_batch_number(),
                m.statement.get_mix_number()
            ))
            .collect::<Vec<_>>(),
        vec![(1, 1, 1), (2, 2, 1), (3, 2, 2)]
    );
    assert_eq!(
        restarted
            .retrieve_messages(1)
            .unwrap()
            .iter()
            .map(|(_, id)| *id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert!(restarted.retrieve_messages(3).unwrap().is_empty());
    assert_eq!(restarted.get_last_external_id().unwrap(), 900);
    let info = restarted.get_storage_info().unwrap();
    assert_eq!(
        (
            info.total_messages,
            info.max_internal_id,
            info.max_external_id
        ),
        (3, 3, 900)
    );
}

#[test]
fn duplicate_identity_and_external_id_reject_the_whole_batch() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStorage::new(dir.path().join("board.sqlite"), None);
    store.store_messages(&[message(10, 1, 1)], false).unwrap();
    for duplicate in [message(10, 2, 1), message(20, 1, 1)] {
        let err = store
            .store_messages(&[message(30, 3, 1), duplicate], false)
            .unwrap_err();
        assert!(
            err.to_string().contains("UNIQUE constraint failed"),
            "{err}"
        );
        assert_eq!(store.retrieve_messages(-1).unwrap().len(), 1);
        assert_eq!(store.get_last_external_id().unwrap(), 10);
    }
    // A full refresh may ignore duplicates, but must still retain new messages.
    store
        .store_messages(&[message(10, 1, 1), message(40, 4, 1)], true)
        .unwrap();
    let rows = store.retrieve_messages(-1).unwrap();
    assert_eq!(
        rows.iter()
            .map(|(m, _)| m.statement.get_batch_number())
            .collect::<Vec<_>>(),
        vec![1, 4]
    );
}

#[test]
fn invalid_schema_wire_and_integer_overflow_roll_back_prior_rows() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStorage::new(dir.path().join("board.sqlite"), None);
    let mut schema = message(2, 2, 1);
    schema.version = "unsupported".into();
    let mut malformed = message(2, 2, 1);
    malformed.message = vec![0];
    for bad in [
        schema,
        malformed,
        message(2, i32::MAX as u64 + 1, 1),
        message(2, 2, i32::MAX as usize + 1),
    ] {
        assert!(store
            .store_messages(&[message(1, 1, 1), bad], false)
            .is_err());
        assert!(store.retrieve_messages(-1).unwrap().is_empty());
    }
    store.store_messages(&[message(1, 1, 1)], false).unwrap();
    assert_eq!(store.retrieve_messages(-1).unwrap().len(), 1);
}

#[test]
fn blob_storage_reads_persisted_bytes_and_reports_missing_or_corrupt_files() {
    let dir = tempfile::tempdir().unwrap();
    let blobs = dir.path().join("nested/blobs");
    let path = dir.path().join("board.sqlite");
    let store = SqliteStorage::new(path.clone(), Some(blobs.clone()));
    let original = message(9, 3, 2);
    store.store_messages(&[original.clone()], false).unwrap();
    let row = &store.retrieve_messages(-1).unwrap()[0];
    assert_eq!(row.0.strand_serialize().unwrap(), original.message);
    assert_eq!(row.1, 1);
    let blob = fs::read_dir(&blobs)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read(&blob).unwrap(), original.message);
    let connection = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT length(message) FROM MESSAGES", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    fs::write(&blob, [0]).unwrap();
    assert!(store.retrieve_messages(-1).is_err());
    fs::remove_file(&blob).unwrap();
    assert!(store
        .retrieve_messages(-1)
        .unwrap_err()
        .to_string()
        .contains("Blob file not found"));
}

#[test]
fn inline_storage_reports_corrupt_rows_and_database_open_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.sqlite");
    let store = SqliteStorage::new(path.clone(), None);
    store.store_messages(&[message(1, 1, 1)], false).unwrap();
    assert_eq!(store.retrieve_messages(-1).unwrap().len(), 1);
    rusqlite::Connection::open(path)
        .unwrap()
        .execute("UPDATE MESSAGES SET message=x'00'", [])
        .unwrap();
    assert!(store.retrieve_messages(-1).is_err());
    let invalid = SqliteStorage::new(dir.path().to_path_buf(), None);
    assert!(invalid.retrieve_messages(-1).is_err());
    assert!(invalid.store_messages(&[], false).is_err());
}

#[test]
fn transient_storage_replaces_consumes_and_clears_even_invalid_batches() {
    let store = NoOpStorage::new();
    store.store_messages(&[message(99, 1, 1)], false).unwrap();
    store.store_messages(&[message(2, 2, 1)], false).unwrap();
    let rows = store.retrieve_messages(1000).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, 2); // This backend intentionally has no persistent local sequence.
    assert_eq!(rows[0].0.statement.get_batch_number(), 2);
    assert!(store.retrieve_messages(-1).unwrap().is_empty());
    let mut bad = message(3, 3, 1);
    bad.message = vec![0];
    store.store_messages(&[bad], false).unwrap();
    assert!(store.retrieve_messages(-1).is_err());
    assert!(store.retrieve_messages(-1).unwrap().is_empty());
    assert_eq!(store.get_last_external_id().unwrap(), -1);
}
