// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use braid::util::{decode_base64, ensure_directory, hash_from_vec, ProtocolContext, ProtocolError};

#[test]
fn hash_and_unpadded_base64_reject_noncanonical_lengths_and_encodings() {
    assert_eq!(hash_from_vec(&[42; 64]).unwrap(), [42; 64]);
    for len in [0, 63, 65] {
        assert!(hash_from_vec(&vec![42; len]).is_err());
    }
    assert_eq!(decode_base64(&"AP8Q".into()).unwrap(), [0, 255, 16]);
    assert_eq!(decode_base64(&"/w".into()).unwrap(), [255]);
    assert!(decode_base64(&String::new()).unwrap().is_empty());
    for invalid in ["/w==", "_w", "A", "AP8Q\n", "/x"] {
        assert!(decode_base64(&invalid.into()).is_err(), "{invalid:?}");
    }
}

#[test]
fn directory_creation_is_idempotent_but_files_and_missing_parents_are_errors() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("board");
    ensure_directory(folder.clone()).unwrap();
    std::fs::write(folder.join("sentinel"), b"keep me").unwrap();
    ensure_directory(folder.clone()).unwrap();
    assert_eq!(std::fs::read(folder.join("sentinel")).unwrap(), b"keep me");
    let error = ensure_directory(folder.join("sentinel")).unwrap_err();
    assert!(error.to_string().starts_with("Path is not a folder:"));
    assert!(ensure_directory(root.path().join("missing/child")).is_err());
    assert!(!root.path().join("missing").exists());
}

#[test]
fn context_preserves_success_and_the_original_typed_failure() {
    let good: Result<i32, ProtocolError> = Ok(7);
    assert_eq!(good.add_context("outer").unwrap(), 7);
    let bad: Result<(), ProtocolError> = Err(ProtocolError::BoardError("disk offline".into()));
    let error = bad.add_context("read").add_context("restart").unwrap_err();
    assert_eq!(error.to_string(), "restart: read: disk offline");
    let ProtocolError::WrappedError(outer, inner) = error else {
        panic!("missing outer context")
    };
    assert_eq!(outer, "restart");
    let ProtocolError::WrappedError(inner_context, original) = *inner else {
        panic!("missing inner context")
    };
    assert_eq!(inner_context, "read");
    assert!(matches!(*original, ProtocolError::BoardError(ref text) if text == "disk offline"));
    let good: Result<i32, strand::util::StrandError> = Ok(9);
    assert_eq!(good.add_context("decode").unwrap(), 9);
    let bad = hash_from_vec(&[0; 63]).add_context("decode").unwrap_err();
    assert!(
        matches!(bad, ProtocolError::WrappedError(ref context, ref error)
        if context == "decode" && matches!(**error, ProtocolError::StrandError(_)))
    );
}
