// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Every form in which a key or a hash of the crypto core is written down.
//!
//! Keys and hashes leave this crate for the database, the vault and a
//! trustee's keys file, and each of them has to be read back by something that
//! is not this process: an admin portal rendering a ceremony, the trustee
//! joining a board, a voter's browser encrypting a ballot. Whenever two places
//! decide a base64 alphabet or a hash casing on their own, they eventually
//! disagree, so all of it is decided here and nowhere else.
//!
//! | Value | Form |
//! |---|---|
//! | trustee signing public key | base64 (padded) of the 32 raw Ed25519 bytes |
//! | trustee share-encryption public key | base64 (padded) of the 32 canonical element bytes |
//! | joint public key | base64 **without padding** of the 32 canonical element bytes |
//! | a protocol hash | 128 lowercase hexadecimal characters |
//! | the board's manager key | base64 (padded) of the 32-byte Ed25519 seed |
//! | trustee signing key | base64 (padded) of the 32-byte Ed25519 seed |
//! | trustee share-encryption secret key | base64 (padded) of the 32 canonical scalar bytes |

use std::fmt;

use anyhow::{Context as _, Result};
use base64::engine::general_purpose::{
    GeneralPurpose, STANDARD, STANDARD_NO_PAD,
};
use base64::Engine as _;
use cryptography::context::Context as _;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::signatures::SignatureScheme;
use wbraid::messages::newtypes::Hash;

use crate::{
    BoardManager, Ctx, Element, Rng, Scalar, Scheme, Signer, VerifyingKey,
};

/// Key material: the encoding the crypto core's own key strings use
/// (`SignatureScheme::verifier_to_base64_string`).
const KEY_MATERIAL_BASE64: GeneralPurpose = STANDARD;

/// The joint public key. Unpadded, which is the voter path already uses.
const JOINT_PUBLIC_KEY_BASE64: GeneralPurpose = STANDARD_NO_PAD;

/// What a trustee's keys are called wherever an error names them.
pub(crate) const SIGNING_PUBLIC_KEY: &str = "signing public key";
pub(crate) const SHARE_ENCRYPTION_PUBLIC_KEY: &str =
    "share encryption public key";
const SIGNING_KEY: &str = "signing key";
const SHARE_ENCRYPTION_SECRET_KEY: &str = "share encryption secret key";

/// The Ed25519 key a trustee signs its board messages with.
pub(crate) fn parse_signing_public_key(encoded: &str) -> Result<VerifyingKey> {
    <Scheme as SignatureScheme<Rng>>::verifier_from_base64_string(encoded)
        .context(SIGNING_PUBLIC_KEY)
}

/// The inverse of [`parse_signing_public_key`].
pub(crate) fn encode_signing_public_key(key: &VerifyingKey) -> Result<String> {
    <Scheme as SignatureScheme<Rng>>::verifier_to_base64_string(key)
        .context(SIGNING_PUBLIC_KEY)
}

/// The ElGamal key the other trustees encrypt a trustee's DKG shares to.
pub(crate) fn parse_share_encryption_public_key(
    encoded: &str,
) -> Result<Element> {
    let bytes = KEY_MATERIAL_BASE64
        .decode(encoded)
        .context(SHARE_ENCRYPTION_PUBLIC_KEY)?;
    Element::deser(&bytes).context(SHARE_ENCRYPTION_PUBLIC_KEY)
}

/// The inverse of [`parse_share_encryption_public_key`].
pub(crate) fn encode_share_encryption_public_key(key: &Element) -> String {
    KEY_MATERIAL_BASE64.encode(key.ser())
}

/// An Ed25519 signing key, the board manager's or a trustee's.
pub(crate) fn parse_signing_key(encoded: &str) -> Result<Signer> {
    <Scheme as SignatureScheme<Rng>>::signer_from_base64_string(encoded)
        .context(SIGNING_KEY)
}

/// The inverse of [`parse_signing_key`].
pub(crate) fn encode_signing_key(key: &Signer) -> Result<String> {
    <Scheme as SignatureScheme<Rng>>::signer_to_base64_string(key)
        .context(SIGNING_KEY)
}

/// The scalar that decrypts the DKG shares dealt to a trustee.
pub(crate) fn parse_share_encryption_secret(encoded: &str) -> Result<Scalar> {
    let bytes = KEY_MATERIAL_BASE64
        .decode(encoded)
        .context(SHARE_ENCRYPTION_SECRET_KEY)?;
    Scalar::deser(&bytes).context(SHARE_ENCRYPTION_SECRET_KEY)
}

/// The inverse of [`parse_share_encryption_secret`].
pub(crate) fn encode_share_encryption_secret(key: &Scalar) -> String {
    KEY_MATERIAL_BASE64.encode(key.ser())
}

/// The joint public key `y` as the ceremony status and the ballot styles carry
/// it.
pub(crate) fn encode_joint_public_key(y: &Element) -> String {
    JOINT_PUBLIC_KEY_BASE64.encode(y.ser())
}

/// A protocol hash in the form the platform stores and compares it: 128
/// lowercase hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashHex(String);

impl HashHex {
    pub(crate) fn of(hash: &Hash) -> HashHex {
        HashHex(hex::encode(hash.as_slice()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The leading characters, for a log line or an error message that should
    /// stay readable. Never should be used for comparison.
    pub(crate) fn short(&self) -> &str {
        &self.0[..16]
    }
}

impl fmt::Display for HashHex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A brand-new protocol manager identity for one board.
pub fn generate_manager() -> BoardManager {
    BoardManager::new(Ctx::gen_signing_key())
}

/// The manager's signing key as the vault keeps it.
pub fn encode_manager_key(manager: &BoardManager) -> Result<String> {
    encode_signing_key(&manager.signing_key).context("the protocol manager key")
}

/// The verifying key of a protocol manager, which is what a `Configuration`
/// names.
pub(crate) fn manager_verifying_key(manager: &BoardManager) -> VerifyingKey {
    <Scheme as SignatureScheme<Rng>>::verifying_key(&manager.signing_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wbraid::messages::newtypes::hash_bytes;

    #[test]
    fn signing_public_key_round_trips_and_rejects_the_old_der_form() {
        let key = manager_verifying_key(&generate_manager());
        let encoded =
            <Scheme as SignatureScheme<Rng>>::verifier_to_base64_string(&key)
                .unwrap();
        assert_eq!(parse_signing_public_key(&encoded).unwrap(), key);
        assert_eq!(encode_signing_public_key(&key).unwrap(), encoded);

        // What the old core stored: a DER SubjectPublicKeyInfo wrapper.
        let old_der =
            "MCowBQYDK2VwAyEAy1vJM4P85hJ1WAPZpRX3/QsOT2usIAuVy4/+t5VHHDs=";
        assert!(parse_signing_public_key(old_der).is_err());
    }

    #[test]
    fn share_encryption_public_key_reads_padded_base64() {
        let key = Ctx::random_element();
        let encoded = STANDARD.encode(key.ser());
        assert_eq!(parse_share_encryption_public_key(&encoded).unwrap(), key);
        assert_eq!(encode_share_encryption_public_key(&key), encoded);
    }

    #[test]
    fn the_manager_and_the_trustees_write_signing_keys_alike() {
        let manager = generate_manager();
        let encoded = encode_manager_key(&manager).unwrap();
        assert_eq!(parse_signing_key(&encoded).unwrap(), manager.signing_key);
        assert_eq!(encode_signing_key(&manager.signing_key).unwrap(), encoded);

        let seed = STANDARD.decode(encoded).unwrap();
        assert_eq!(seed.len(), 32);
        assert!(parse_signing_key(&STANDARD_NO_PAD.encode(&seed)).is_err());
        assert!(parse_signing_key(&STANDARD.encode(&seed[..31])).is_err());
        assert!(parse_signing_key("not base64!").is_err());
    }

    #[test]
    fn share_encryption_secret_round_trips_and_refuses_what_is_not_a_scalar() {
        let scalar = Ctx::random_scalar();
        let encoded = encode_share_encryption_secret(&scalar);
        assert_eq!(parse_share_encryption_secret(&encoded).unwrap(), scalar);
        assert_eq!(STANDARD.decode(encoded).unwrap(), scalar.ser());

        let bytes = scalar.ser();
        let mut longer = bytes.clone();
        longer.push(0);
        for refused in [
            STANDARD_NO_PAD.encode(&bytes),
            STANDARD.encode(&bytes[..31]),
            STANDARD.encode(longer),
            // Above the group order: not the canonical form of any scalar.
            STANDARD.encode([0xffu8; 32]),
            "not base64!".to_string(),
        ] {
            assert!(
                parse_share_encryption_secret(&refused).is_err(),
                "{refused}"
            );
        }
    }

    #[test]
    fn share_encryption_public_key_rejects_garbage_and_wrong_lengths() {
        assert!(parse_share_encryption_public_key("not base64!").is_err());
        assert!(
            parse_share_encryption_public_key(&STANDARD.encode([1u8; 31]))
                .is_err()
        );
    }

    #[test]
    fn joint_public_key_reads_back_the_way_the_voter_path_decodes_it() {
        // sequent-core's encrypt.rs decodes the joint key as unpadded standard
        // base64 of the element's canonical bytes.
        let y = Ctx::random_element();
        let bytes =
            STANDARD_NO_PAD.decode(encode_joint_public_key(&y)).unwrap();
        assert_eq!(Element::deser(&bytes).unwrap(), y);
    }

    #[test]
    fn hash_hex_is_128_lowercase_characters() {
        let hex = HashHex::of(&hash_bytes(b"board"));
        assert_eq!(hex.as_str().len(), 128);
        assert_eq!(hex.as_str(), hex.as_str().to_lowercase());
        assert_ne!(hex, HashHex::of(&hash_bytes(b"another board")));
        assert!(hex.as_str().starts_with(hex.short()));
    }
}
