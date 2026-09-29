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
//! | a decrypted plaintext element | the 30 bytes the element encodes |
//!
//! The stored form of a cast vote's ciphertext is the ballot module's.

use std::fmt;

use anyhow::{bail, Context as _, Result};
use base64::engine::general_purpose::{
    GeneralPurpose, STANDARD, STANDARD_NO_PAD,
};
use base64::Engine as _;
use cryptography::context::Context as _;
use cryptography::groups::Ristretto255Group;
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
pub struct HashHex {
    hash: Hash,
    hex: String,
}

impl HashHex {
    pub(crate) fn of(hash: &Hash) -> HashHex {
        HashHex {
            hash: *hash,
            hex: hex::encode(hash.as_slice()),
        }
    }

    /// A stored hash, in exactly the form [`HashHex::as_str`] gives.
    pub fn parse(encoded: &str) -> Result<HashHex> {
        let mut hash = Hash::default();
        hex::decode_to_slice(encoded, &mut hash[..])
            .with_context(|| format!("{encoded:?} is not a protocol hash"))?;
        let parsed = HashHex::of(&hash);
        if parsed.as_str() != encoded {
            bail!("{encoded:?} is not a protocol hash in lowercase");
        }
        Ok(parsed)
    }

    pub fn as_str(&self) -> &str {
        &self.hex
    }

    /// The hash itself, as braid's message heads carry it.
    pub(crate) fn to_hash(&self) -> Hash {
        self.hash
    }

    /// The leading characters, for a log line or an error message that should
    /// stay readable. Never should be used for comparison.
    pub(crate) fn short(&self) -> &str {
        &self.hex[..16]
    }
}

impl fmt::Display for HashHex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.hex)
    }
}

/// The 30 bytes a plaintext element encodes: what a voter's client packed into
/// one ballot, as the trustees decrypted it.
pub type ElementPayload = [u8; 30];

/// The payload of a decrypted plaintext element.
pub(crate) fn decode_plaintext_element(
    element: &Element,
) -> Result<ElementPayload> {
    Ristretto255Group::decode_30_bytes(element)
        .context("a plaintext element that does not decode to 30 bytes")
}

/// A brand-new protocol manager identity for one board.
pub fn generate_manager() -> BoardManager {
    BoardManager::new(Ctx::gen_signing_key())
}

/// What the protocol manager key is called wherever an error names it.
const MANAGER_KEY: &str = "the protocol manager key";

/// The manager's signing key as the vault keeps it.
pub fn encode_manager_key(manager: &BoardManager) -> Result<String> {
    encode_signing_key(&manager.signing_key).context(MANAGER_KEY)
}

/// The inverse of [`encode_manager_key`].
pub fn parse_manager_key(encoded: &str) -> Result<BoardManager> {
    Ok(BoardManager::new(
        parse_signing_key(encoded).context(MANAGER_KEY)?,
    ))
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

    #[test]
    fn a_stored_hash_reads_back_as_the_hash_it_was_written_from() {
        let hash = hash_bytes(b"board");
        let stored = HashHex::of(&hash).as_str().to_string();
        let parsed = HashHex::parse(&stored).unwrap();
        assert_eq!(parsed, HashHex::of(&hash));
        assert_eq!(parsed.to_hash(), hash);
        assert_eq!(parsed.as_str(), stored);
    }

    #[test]
    fn only_the_stored_form_of_a_hash_parses() {
        let stored = HashHex::of(&hash_bytes(b"board")).as_str().to_string();
        for refused in [
            String::new(),
            stored.to_uppercase(),
            stored[1..].to_string(),
            stored[2..].to_string(),
            format!("{stored}00"),
            format!("0x{}", &stored[2..]),
            format!("{} ", &stored[1..]),
            "zz".repeat(64),
        ] {
            assert!(HashHex::parse(&refused).is_err(), "{refused:?}");
        }
    }

    #[test]
    fn a_manager_key_reads_back_as_the_manager_that_signs() {
        let manager = generate_manager();
        let stored = encode_manager_key(&manager).unwrap();
        let parsed = parse_manager_key(&stored).unwrap();
        assert_eq!(
            manager_verifying_key(&parsed),
            manager_verifying_key(&manager)
        );

        let error =
            format!("{:#}", parse_manager_key("not base64!").err().unwrap());
        assert!(error.contains(MANAGER_KEY), "{error}");
    }

    #[test]
    fn a_plaintext_element_decodes_to_the_bytes_it_encodes() {
        let mut payload: ElementPayload = [0u8; 30];
        payload[..18].copy_from_slice(b"testing encryption");
        for payload in [payload, [0xde; 30], [0u8; 30], [0xff; 30]] {
            let element = Ristretto255Group::encode_30_bytes(&payload).unwrap();
            assert_eq!(decode_plaintext_element(&element).unwrap(), payload);
        }
    }
}
