// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use cryptography::utils::{
    serialization::Serializable,
    symm::{decrypt, encrypt, sk_from_bytes},
};
use voting_crypto::envelope::{read_encrypted_value, StoredEnvelope};
use voting_crypto::RecordSerialize;

#[test]
fn persisted_application_frames_decrypt_with_wbraid_without_rewriting_bytes() {
    let key = sk_from_bytes(&[7u8; 32]).unwrap();
    let ciphertext = encrypt(key.clone(), b"stored application secret").unwrap();
    let canonical = ciphertext.ser();
    let stored = StoredEnvelope::from(ciphertext.clone())
        .record_serialize()
        .unwrap();
    // Independent fixed-width length fields pin the two persisted contracts.
    assert_eq!(
        &canonical[..8],
        &(ciphertext.encrypted_bytes.len() as u64).to_be_bytes()
    );
    assert_eq!(
        &stored[..4],
        &(ciphertext.encrypted_bytes.len() as u32).to_le_bytes()
    );
    for bytes in [stored, canonical] {
        let decoded = read_encrypted_value(&bytes).unwrap();
        assert_eq!(
            decrypt(&key, &decoded).unwrap(),
            b"stored application secret"
        );
        for prefix in 0..bytes.len() {
            assert!(read_encrypted_value(&bytes[..prefix]).is_err());
        }
        let mut extended = bytes.clone();
        extended.push(0);
        assert!(read_encrypted_value(&extended).is_err());
        let mut tampered = decoded;
        tampered.encrypted_bytes[0] ^= 1;
        assert!(decrypt(&key, &tampered).is_err());
    }
}
