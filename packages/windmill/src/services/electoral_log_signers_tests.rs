// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

#[test]
fn events_without_the_setting_are_software() {
    assert_eq!(policy_of(None).unwrap(), ElectoralLogSignerPolicy::SOFTWARE);
    assert_eq!(policy_of(Some(&json!({}))).unwrap(), ElectoralLogSignerPolicy::SOFTWARE);
    assert_eq!(
        policy_of(Some(&json!({"electoral_log_signer_policy": null}))).unwrap(),
        ElectoralLogSignerPolicy::SOFTWARE
    );
}

#[test]
fn the_stored_setting_is_read_and_unknown_values_are_refused() {
    assert_eq!(
        policy_of(Some(&json!({"electoral_log_signer_policy": "external"}))).unwrap(),
        ElectoralLogSignerPolicy::EXTERNAL
    );
    assert!(policy_of(Some(&json!({"electoral_log_signer_policy": "hsm"}))).is_err());
}

// sequent-core's `electoral_log_signer_policy_tests` shows a Borsh round
// trip loses the setting: it is read from the stored JSON here only.

#[test]
fn without_an_installed_signer_external_events_are_refused() {
    // Nothing is installed in this test binary.
    let error = installed().err().unwrap().to_string();
    assert!(error.contains("refused"), "{error}");
}

#[test]
fn a_presentation_stored_as_a_string_is_read_through_and_malformed_ones_refused() {
    let as_string = Value::String(r#"{"electoral_log_signer_policy":"external"}"#.to_string());
    assert_eq!(policy_of(Some(&as_string)).unwrap(), ElectoralLogSignerPolicy::EXTERNAL);
    assert!(policy_of(Some(&Value::String("not json".to_string()))).is_err());
    assert!(policy_of(Some(&json!([1, 2]))).is_err());
    assert!(policy_of(Some(&json!(7))).is_err());
}

#[test]
fn the_setting_cannot_change_once_a_board_key_exists() {
    let key = BoardKey { public_key: "AA".into(), key_version: 1 };
    use ElectoralLogSignerPolicy::*;
    assert!(check_unchanged(SOFTWARE, None, true).is_ok(), "today's events");
    assert!(check_unchanged(EXTERNAL, Some(&key), false).is_ok());
    assert!(check_unchanged(EXTERNAL, None, true).is_err(), "switched to external after a software key");
    assert!(check_unchanged(SOFTWARE, Some(&key), false).is_err(), "switched back to software");
    assert!(check_unchanged(EXTERNAL, Some(&key), true).is_err(), "both keys");
}

/// A signer whose key is "held elsewhere" (a key behind a wrapper).
struct Elsewhere(voting_crypto::signing::SigningKey);

impl electoral_log::signer::MessageSigner for Elsewhere {
    fn public_key(&self) -> Result<VerificationKey> {
        VerificationKey::from_sk(&self.0)
    }
    fn sign(&self, bytes: &[u8]) -> Result<voting_crypto::signing::Signature> {
        self.0.sign(bytes)
    }
}

#[test]
fn a_checkpoint_signed_through_a_plugged_in_signer_is_byte_identical() {
    use electoral_log::messages::message::SigningData;
    let key = voting_crypto::signing::SigningKey::generate().unwrap();
    let checkpoint = electoral_log::proofs::Checkpoint {
        log_name: "board".to_string(),
        log_uid: electoral_log::proofs::Uuid::from_u128(7),
        tree_size: 3,
        root: vec![1u8; 32],
    };
    let reason = electoral_log::messages::newtypes::ElectoralLogCheckpointReason::Periodic;
    let software = SigningData::new(key.clone(), "", key.clone());
    let external: electoral_log::signer::SharedSigner = Arc::new(Elsewhere(key));
    let plugged = SigningData::with_signers(external.clone(), "", external);
    let a = crate::services::electoral_log::sign_checkpoint(&software, &checkpoint, reason).unwrap();
    let b = crate::services::electoral_log::sign_checkpoint(&plugged, &checkpoint, reason).unwrap();
    assert_eq!(a.signature, b.signature);
    assert_eq!(a.signer_pk, b.signer_pk);
}
