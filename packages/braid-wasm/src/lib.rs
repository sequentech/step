//! WASM bindings for Braid trustee operations.
//! This focuses on key generation, PBKDF2 commitments, local key storage, and
//! helper utilities for browser-based trustees.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::collections::HashMap;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use anyhow::Result;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

use js_sys::Uint8Array;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn set_hooks() {
    console_error_panic_hook::set_once();
}

/// Internal key identifier used as an opaque handle in JS.
type KeyId = u32;

thread_local! {
    static KEYS: RefCell<HashMap<KeyId, StrandSignatureSk>> = RefCell::new(HashMap::new());
    static NEXT_KEY_ID: RefCell<KeyId> = RefCell::new(1);
}

fn store_key(sk: StrandSignatureSk) -> KeyId {
    KEYS.with(|cell| {
        NEXT_KEY_ID.with(|next_cell| {
            let mut map = cell.borrow_mut();
            let mut next = next_cell.borrow_mut();
            let id = *next;
            *next = next.saturating_add(1).max(1);
            map.insert(id, sk);
            id
        })
    })
}

fn with_key<F, R>(key_id: KeyId, f: F) -> Result<R, String>
where
    F: FnOnce(&StrandSignatureSk) -> Result<R, String>,
{
    KEYS.with(|cell| {
        let map = cell.borrow();
        match map.get(&key_id) {
            Some(sk) => f(sk),
            None => Err(format!("Unknown key_id {key_id}")),
        }
    })
}

/// PBKDF2 commitment of a private key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCommitment {
    /// Base64-encoded salt.
    pub salt_b64: String,
    /// PBKDF2 iteration count.
    pub iterations: u32,
    /// Base64-encoded PBKDF2 output.
    pub hash_b64: String,
}

/// Response of `generate_trustee_keypair_js`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedKeypair {
    pub election_id: String,
    pub trustee_id: String,
    /// base64-encoded SPKI DER public key (StrandSignaturePk::to_der_b64_string).
    pub public_key_b64: String,
    pub commitment: KeyCommitment,
    /// Opaque handle to the private signing key stored in memory for this
    /// session.
    pub key_id: KeyId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    pub salt_b64: String,
    pub iterations: u32,
}

/// On-disk/browser-exported key file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFile {
    pub version: u32,
    pub election_id: String,
    pub trustee_id: String,
    /// base64-encoded SPKI DER public key.
    pub public_key_b64: String,
    /// KDF parameters for file encryption (PBKDF2 over passphrase).
    pub kdf: KdfParams,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

/// Result returned when importing a key file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedKey {
    pub election_id: String,
    pub trustee_id: String,
    pub public_key_b64: String,
    pub key_id: KeyId,
}

/// Metadata describing a large protocol artifact stored out-of-band in S3/Minio
/// rather than inline in B3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    /// S3 bucket name where the artifact is stored.
    pub bucket: String,
    /// Object key within the bucket.
    pub key: String,
    /// Hex-encoded SHA-256 hash of the artifact contents.
    pub sha256_hex: String,
    /// Size of the artifact in bytes.
    pub size: u64,
    /// MIME type of the artifact (e.g. application/octet-stream).
    pub content_type: String,
    /// Logical kind of artifact (e.g. BALLOTS, MIX, DECRYPTION_FACTORS, PLAINTEXTS).
    pub kind: String,
}

fn derive_commitment_random(private_key_der: &[u8], iterations: u32) -> Result<KeyCommitment> {
    const SALT_LEN: usize = 16;
    const DK_LEN: usize = 32;

    let mut salt = [0u8; SALT_LEN];
    getrandom::getrandom(&mut salt)?;

    let mut dk = [0u8; DK_LEN];
    pbkdf2_hmac::<Sha256>(private_key_der, &salt, iterations, &mut dk);

    let salt_b64 = base64::encode(salt);
    let hash_b64 = base64::encode(dk);

    Ok(KeyCommitment {
        salt_b64,
        iterations,
        hash_b64,
    })
}

fn generate_signing_keypair() -> Result<(StrandSignatureSk, StrandSignaturePk)> {
    let sk = StrandSignatureSk::gen().map_err(|e| anyhow::anyhow!("Error generating signing key: {e}"))?;
    let pk = StrandSignaturePk::from_sk(&sk)
        .map_err(|e| anyhow::anyhow!("Error deriving public key from secret key: {e}"))?;
    Ok((sk, pk))
}

fn encrypt_key_to_file(
    sk_der: &[u8],
    election_id: &str,
    trustee_id: &str,
    public_key_b64: &str,
    passphrase: &str,
    iterations: u32,
) -> Result<KeyFile> {
    const SALT_LEN: usize = 16;
    const KEY_LEN: usize = 32;
    const NONCE_LEN: usize = 12;

    let mut salt = [0u8; SALT_LEN];
    getrandom::getrandom(&mut salt)?;

    let mut enc_key_bytes = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, iterations, &mut enc_key_bytes);

    let key = Key::<Aes256Gcm>::from_slice(&enc_key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, sk_der)
        .map_err(|e| anyhow::anyhow!("AES-GCM encryption failed: {e}"))?;

    enc_key_bytes.zeroize();

    let kdf = KdfParams {
        salt_b64: base64::encode(salt),
        iterations,
    };

    Ok(KeyFile {
        version: 1,
        election_id: election_id.to_string(),
        trustee_id: trustee_id.to_string(),
        public_key_b64: public_key_b64.to_string(),
        kdf,
        nonce_b64: base64::encode(nonce_bytes),
        ciphertext_b64: base64::encode(ciphertext),
    })
}

fn decrypt_key_from_file(file: &KeyFile, passphrase: &str) -> Result<Vec<u8>> {
    const KEY_LEN: usize = 32;

    let salt = base64::decode(&file.kdf.salt_b64)?;
    let mut enc_key_bytes = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, file.kdf.iterations, &mut enc_key_bytes);

    let key = Key::<Aes256Gcm>::from_slice(&enc_key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes = base64::decode(&file.nonce_b64)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = base64::decode(&file.ciphertext_b64)?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("AES-GCM decryption failed: {e}"))?;

    enc_key_bytes.zeroize();

    Ok(plaintext)
}

/// Export the in-memory private key identified by `key_id` into an encrypted
/// key file JSON object.
#[wasm_bindgen]
pub fn export_private_key_file_js(
    key_id: KeyId,
    election_id: String,
    trustee_id: String,
    public_key_b64: String,
    passphrase: String,
    iterations: u32,
) -> Result<JsValue, JsValue> {
    let file = with_key(key_id, |sk| {
        let sk_der = sk
            .to_der()
            .map_err(|e| format!("Error serializing private key (DER): {e}"))?;
        encrypt_key_to_file(
            &sk_der,
            &election_id,
            &trustee_id,
            &public_key_b64,
            &passphrase,
            iterations,
        )
        .map_err(|e| format!("Error encrypting key file: {e}"))
    })?;

    serde_wasm_bindgen::to_value(&file)
        .map_err(|e| JsValue::from_str(&format!("Error serializing KeyFile: {e}")))
}

/// Import a key file JSON object and restore the private key in memory.
#[wasm_bindgen]
pub fn import_private_key_file_js(file: JsValue, passphrase: String) -> Result<JsValue, JsValue> {
    let file: KeyFile = serde_wasm_bindgen::from_value(file)
        .map_err(|e| JsValue::from_str(&format!("Error parsing key file JSON: {e}")))?;

    let sk_der = decrypt_key_from_file(&file, &passphrase)
        .map_err(|e| JsValue::from_str(&format!("Error decrypting key file: {e}")))?;

    let sk = StrandSignatureSk::from_der(&sk_der).map_err(|e| {
        JsValue::from_str(&format!("Error deserializing private key from DER: {e}"))
    })?;

    let key_id = store_key(sk);

    let imported = ImportedKey {
        election_id: file.election_id,
        trustee_id: file.trustee_id,
        public_key_b64: file.public_key_b64,
        key_id,
    };

    serde_wasm_bindgen::to_value(&imported)
        .map_err(|e| JsValue::from_str(&format!("Error serializing ImportedKey: {e}")))
}

/// Compute the SHA-256 hash (hex) of an arbitrary byte array provided from JS.
#[wasm_bindgen]
pub fn sha256_hex_js(data: &Uint8Array) -> String {
    let mut bytes = vec![0u8; data.length() as usize];
    data.copy_to(&mut bytes[..]);
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    hex::encode(digest)
}

/// Build an ArtifactEnvelope from raw bytes and S3 metadata. This is intended
/// to be used after uploading an artifact to S3 via a presigned URL.
#[wasm_bindgen]
pub fn make_artifact_envelope_js(
    bucket: String,
    key: String,
    content_type: String,
    kind: String,
    data: &Uint8Array,
) -> Result<JsValue, JsValue> {
    let size = data.length() as u64;
    let sha256_hex = sha256_hex_js(data);

    let env = ArtifactEnvelope {
        bucket,
        key,
        sha256_hex,
        size,
        content_type,
        kind,
    };

    serde_wasm_bindgen::to_value(&env)
        .map_err(|e| JsValue::from_str(&format!("Error serializing ArtifactEnvelope: {e}")))
}


/// A minimal representation of a signed board message to be sent from the
/// browser trustee to the backend via GraphQL/HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSignedMessage {
    /// Logical board name (e.g. "dkg", "mix", "decrypt").
    pub board: String,
    /// Arbitrary protocol payload encoded as hex (typically a Borsh-encoded
    /// statement or batch structure).
    pub payload_hex: String,
    /// Optional reference to a large artifact stored in S3/Minio.
    pub artifact: Option<ArtifactEnvelope>,
    /// Base64-encoded Strand signature over `payload_hex` decoded as bytes.
    pub signature_b64: String,
    /// Base64-encoded SPKI DER public key of the signer.
    pub public_key_b64: String,
}

/// Sign an arbitrary payload for posting to the bulletin board and package it
/// together with optional artifact metadata.
///
/// The actual network I/O is performed by the JS/TypeScript side; this helper
/// focuses on computing the signature and producing a serialisable envelope.
#[wasm_bindgen]
pub fn build_signed_board_message_js(
    key_id: KeyId,
    board: String,
    payload: &Uint8Array,
    artifact: JsValue,
    public_key_b64: String,
) -> Result<JsValue, JsValue> {
    // Extract payload bytes from JS.
    let mut bytes = vec![0u8; payload.length() as usize];
    payload.copy_to(&mut bytes[..]);

    // Sign with the in-memory private key.
    let signature_b64 = with_key(key_id, |sk| {
        let sig = sk
            .sign(&bytes)
            .map_err(|e| format!("Error signing payload: {e}"))?;
        sig.to_b64_string()
            .map_err(|e| format!("Error serialising signature as base64: {e}"))
    })?;

    // Decode optional ArtifactEnvelope from JS.
    let artifact: Option<ArtifactEnvelope> = if artifact.is_undefined() || artifact.is_null() {
        None
    } else {
        Some(
            serde_wasm_bindgen::from_value(artifact)
                .map_err(|e| JsValue::from_str(&format!("Error parsing ArtifactEnvelope: {e}")))?,
        )
    };

    let payload_hex = hex::encode(&bytes);

    let msg = BoardSignedMessage {
        board,
        payload_hex,
        artifact,
        signature_b64,
        public_key_b64,
    };

    serde_wasm_bindgen::to_value(&msg)
        .map_err(|e| JsValue::from_str(&format!("Error serialising BoardSignedMessage: {e}")))
}

/// Derive a deterministic PBKDF2 commitment for a private key using a
/// per-(election, trustee) salt. This allows us to recompute the commitment
/// when importing a key file without needing any server-side state.
fn derive_commitment(
    election_id: &str,
    trustee_id: &str,
    private_key_der: &[u8],
    iterations: u32,
) -> Result<KeyCommitment> {
    const SALT_LEN: usize = 16;
    const DK_LEN: usize = 32;

    // Derive a stable salt from the (election_id, trustee_id) pair.
    let mut hasher = Sha256::new();
    hasher.update(election_id.as_bytes());
    hasher.update(b":");
    hasher.update(trustee_id.as_bytes());
    let digest = hasher.finalize();

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&digest[..SALT_LEN]);

    let mut dk = [0u8; DK_LEN];
    pbkdf2_hmac::<Sha256>(private_key_der, &salt, iterations, &mut dk);

    Ok(KeyCommitment {
        salt_b64: base64::encode(salt),
        iterations,
        hash_b64: base64::encode(dk),
    })
}

#[wasm_bindgen]
pub fn generate_trustee_keypair_js(
    election_id: String,
    trustee_id: String,
    iterations: u32,
) -> Result<JsValue, JsValue> {
    let (sk, pk) = match generate_signing_keypair() {
        Ok(pair) => pair,
        Err(e) => {
            return Err(JsValue::from_str(&format!("Key generation error: {e}")));
        }
    };

    let sk_der = match sk.to_der() {
        Ok(der) => der,
        Err(e) => {
            return Err(JsValue::from_str(&format!("Error serializing private key (DER): {e}")));
        }
    };

    let public_key_b64 = match pk.to_der_b64_string() {
        Ok(s) => s,
        Err(e) => {
            return Err(JsValue::from_str(&format!("Error serializing public key (DER base64): {e}")));
        }
    };

    let commitment = match derive_commitment(&election_id, &trustee_id, &sk_der, iterations) {
        Ok(c) => c,
        Err(e) => {
            return Err(JsValue::from_str(&format!("Error deriving key commitment: {e}")));
        }
    };

    let key_id = store_key(sk);

    let result = GeneratedKeypair {
        election_id,
        trustee_id,
        public_key_b64,
        commitment,
        key_id,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Error serializing GeneratedKeypair: {e}")))
}

/// Recompute the deterministic PBKDF2 commitment for an in-memory key
/// identified by `key_id`.
#[wasm_bindgen]
pub fn recompute_key_commitment_js(
    key_id: KeyId,
    election_id: String,
    trustee_id: String,
    iterations: u32,
) -> Result<JsValue, JsValue> {
    let commitment = with_key(key_id, |sk| {
        let sk_der = sk
            .to_der()
            .map_err(|e| format!("Error serializing private key (DER): {e}"))?;
        derive_commitment(&election_id, &trustee_id, &sk_der, iterations)
            .map_err(|e| format!("Error deriving key commitment: {e}"))
    })?;

    serde_wasm_bindgen::to_value(&commitment)
        .map_err(|e| JsValue::from_str(&format!("Error serializing KeyCommitment: {e}")))
}
