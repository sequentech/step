// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::LocalBoard;
use b3::{
    grpc::GrpcB3Message,
    messages::{
        artifact::Configuration, message::Message, newtypes::CiphertextsHash,
        protocol_manager::ProtocolManager,
    },
};
use std::{fs, marker::PhantomData};
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::{StrandSignaturePk, StrandSignatureSk},
};
fn message(id: i64, batch: usize, mix: usize) -> GrpcB3Message {
    let mut der = vec![
        0x30, 0x2e, 2, 1, 0, 0x30, 5, 6, 3, 0x2b, 0x65, 0x70, 4, 0x22, 4, 0x20,
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
        CiphertextsHash([id as u8; 64]),
        CiphertextsHash([2; 64]),
        mix,
        &signer,
    )
    .unwrap();
    // The v10 envelope carries no statement metadata; identity comes from the signed bytes.
    GrpcB3Message {
        id,
        message: signed.strand_serialize().unwrap(),
        version: "1".into(),
    }
}
#[test]
fn local_order_survives_restart_independently_of_remote_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.sqlite");
    let mut board = LocalBoard::<RistrettoCtx>::new(Some(path.clone()), None);
    assert_eq!(board.get_last_external_id().unwrap(), -1);
    board
        .update_store(
            &vec![message(900, 1, 1), message(2, 2, 1), message(400, 2, 2)],
            false,
        )
        .unwrap();
    drop(board);
    let mut restarted = LocalBoard::<RistrettoCtx>::new(Some(path), None);
    let rows = restarted
        .store_and_return_messages(&vec![], -1, false)
        .unwrap();
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
            .store_and_return_messages(&vec![], 1, false)
            .unwrap()
            .iter()
            .map(|(_, id)| *id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert!(restarted
        .store_and_return_messages(&vec![], 3, false)
        .unwrap()
        .is_empty());
    assert_eq!(restarted.get_last_external_id().unwrap(), 900);
}
#[test]
fn duplicate_constraints_roll_back_the_batch_but_refresh_keeps_new_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = LocalBoard::<RistrettoCtx>::new(Some(dir.path().join("board.sqlite")), None);
    board.update_store(&vec![message(10, 1, 1)], false).unwrap();
    for duplicate in [message(10, 2, 1), message(20, 1, 1)] {
        assert!(board
            .update_store(&vec![message(30, 3, 1), duplicate], false)
            .unwrap_err()
            .to_string()
            .contains("UNIQUE constraint failed"));
        assert_eq!(
            board
                .store_and_return_messages(&vec![], -1, false)
                .unwrap()
                .len(),
            1
        );
    }
    board
        .update_store(&vec![message(10, 1, 1), message(40, 4, 1)], true)
        .unwrap();
    assert_eq!(
        board
            .store_and_return_messages(&vec![], -1, false)
            .unwrap()
            .iter()
            .map(|(m, _)| m.statement.get_batch_number())
            .collect::<Vec<_>>(),
        vec![1, 4]
    );
}
#[test]
fn schema_wire_and_integer_errors_do_not_commit_preceding_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = LocalBoard::<RistrettoCtx>::new(Some(dir.path().join("board.sqlite")), None);
    let mut schema = message(2, 2, 1);
    schema.version = "unknown".into();
    let mut wire = message(2, 2, 1);
    wire.message = vec![0];
    for bad in [
        schema,
        wire,
        message(2, i32::MAX as usize + 1, 1),
        message(2, 2, i32::MAX as usize + 1),
    ] {
        assert!(board
            .update_store(&vec![message(1, 1, 1), bad], false)
            .is_err());
        assert!(board
            .store_and_return_messages(&vec![], -1, false)
            .unwrap()
            .is_empty());
    }
    assert_eq!(
        board
            .store_and_return_messages(&vec![message(1, 1, 1)], -1, false)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn blob_reads_preserve_bytes_and_return_errors_for_corrupt_or_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let blobs = dir.path().join("nested/blobs");
    let mut board =
        LocalBoard::<RistrettoCtx>::new(Some(dir.path().join("board.sqlite")), Some(blobs.clone()));
    let original = message(9, 3, 2);
    let rows = board
        .store_and_return_messages(&vec![original.clone()], -1, false)
        .unwrap();
    assert_eq!(rows[0].0.strand_serialize().unwrap(), original.message);
    assert_eq!(rows[0].1, 1);
    let blob = fs::read_dir(blobs).unwrap().next().unwrap().unwrap().path();
    assert_eq!(fs::read(&blob).unwrap(), original.message);
    fs::write(&blob, [0]).unwrap();
    let corrupt = board
        .store_and_return_messages(&vec![], -1, false)
        .unwrap_err();
    assert!(matches!(
        corrupt.downcast_ref::<strand::util::StrandError>(),
        Some(strand::util::StrandError::SerializationError(_))
    ));
    fs::remove_file(blob).unwrap();
    let missing = board
        .store_and_return_messages(&vec![], -1, false)
        .unwrap_err();
    assert_eq!(
        missing.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn artifact_lookup_propagates_corrupt_and_missing_blob_errors() {
    let dir = tempfile::tempdir().unwrap();
    let blobs = dir.path().join("blobs");
    let board =
        LocalBoard::<RistrettoCtx>::new(Some(dir.path().join("board.sqlite")), Some(blobs.clone()));
    let mut original = message(9, 3, 2);
    // This storage accessor preserves opaque artifact bytes; signature verification
    // belongs to the trustee, not this persistence method.
    let mut decoded = Message::strand_deserialize(&original.message).unwrap();
    decoded.artifact = Some(vec![1, 2, 3]);
    original.message = decoded.strand_serialize().unwrap();
    board.update_store(&vec![original], false).unwrap();
    assert_eq!(board.get_artifact_from_store(1).unwrap(), [1, 2, 3]);
    let path = fs::read_dir(&blobs)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(&path, [0]).unwrap();
    let corrupt = board.get_artifact_from_store(1).unwrap_err();
    assert!(matches!(
        corrupt.downcast_ref::<strand::util::StrandError>(),
        Some(strand::util::StrandError::SerializationError(_))
    ));
    fs::remove_file(path).unwrap();
    let missing = board.get_artifact_from_store(1).unwrap_err();
    assert_eq!(
        missing.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn failed_blob_batches_remove_new_files_and_retry_uses_fresh_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let blobs = dir.path().join("blobs");
    let mut store =
        LocalBoard::<RistrettoCtx>::new(Some(dir.path().join("board.sqlite")), Some(blobs.clone()));
    store.update_store(&vec![message(10, 1, 1)], false).unwrap();
    let mut malformed = message(40, 4, 1);
    malformed.version = "unknown".into();
    for bad in [malformed, message(10, 4, 1)] {
        assert!(store
            .update_store(&vec![message(30, 3, 1), bad], false)
            .is_err());
        assert_eq!(
            store
                .store_and_return_messages(&vec![], -1, false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fs::read_dir(&blobs).unwrap().count(),
            1,
            "failed batch left unreferenced blobs"
        );
    }
    let retry = message(31, 3, 1);
    store.update_store(&vec![retry.clone()], false).unwrap();
    assert_eq!(
        store.store_and_return_messages(&vec![], -1, false).unwrap()[1]
            .0
            .strand_serialize()
            .unwrap(),
        retry.message
    );
    assert_eq!(fs::read_dir(&blobs).unwrap().count(), 2);
}
