// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signed notes, in the format of `golang.org/x/mod/sumdb/note`, with Ed25519
//! keys: the statements step signs with the operator's `authority` key.

use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

/// A signature line starts with an em dash and a space.
pub const SIGNATURE_PREFIX: &str = "\u{2014} ";
const PRIVATE_KEY_PREFIX: &str = "PRIVATE+KEY+";
const ALG_ED25519: u8 = 1;
const SEED_SIZE: usize = 32;
const KEY_HASH_SIZE: usize = 4;

/// The key ID: the first 4 bytes of `SHA-256(name || "\n" || alg || key)`.
pub fn key_hash(name: &str, alg: u8, key: &[u8]) -> [u8; KEY_HASH_SIZE] {
    let mut h = Sha256::new();
    h.update(name.as_bytes());
    h.update(b"\n");
    h.update([alg]);
    h.update(key);
    let sum = h.finalize();
    [sum[0], sum[1], sum[2], sum[3]]
}

fn check_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && !name.contains('+') && !name.chars().any(char::is_whitespace),
        "a key name is non-empty, with no spaces and no +"
    );
    Ok(())
}

fn key_id(text: &str) -> Result<[u8; KEY_HASH_SIZE]> {
    ensure!(text.len() == 2 * KEY_HASH_SIZE, "a key ID has 8 hex digits");
    hex::decode(text)
        .context("a key ID has 8 hex digits")?
        .try_into()
        .map_err(|_| anyhow!("a key ID has 8 hex digits"))
}

/// A verifier key, `<name>+<key ID hex>+<base64 of the type byte and key>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vkey {
    pub name: String,
    pub key_hash: [u8; KEY_HASH_SIZE],
    pub alg: u8,
    pub key: Vec<u8>,
}

impl Vkey {
    pub fn parse(text: &str) -> Result<Self> {
        let mut parts = text.splitn(3, '+');
        let (Some(name), Some(id), Some(data)) = (parts.next(), parts.next(), parts.next()) else {
            bail!("a vkey is <name>+<key ID>+<key>");
        };
        check_name(name)?;
        let key_hash = key_id(id)?;
        let data = STANDARD.decode(data).context("a vkey's key is base64")?;
        let (&alg, key) = data.split_first().ok_or_else(|| anyhow!("empty vkey"))?;
        ensure!(
            self::key_hash(name, alg, key) == key_hash,
            "the vkey's key ID doesn't match its key"
        );
        Ok(Vkey {
            name: name.to_string(),
            key_hash,
            alg,
            key: key.to_vec(),
        })
    }
}

/// An Ed25519 note signer.
pub struct NoteSigner {
    name: String,
    key_hash: [u8; KEY_HASH_SIZE],
    key: SigningKey,
}

impl NoteSigner {
    pub fn from_seed(name: &str, seed: &[u8; SEED_SIZE]) -> Result<Self> {
        check_name(name)?;
        let key = SigningKey::from_bytes(seed);
        let key_hash = key_hash(name, ALG_ED25519, key.verifying_key().as_bytes());
        Ok(NoteSigner {
            name: name.to_string(),
            key_hash,
            key,
        })
    }

    /// Reads a `PRIVATE+KEY+<name>+<key ID>+<base64>` key, checking its key ID.
    pub fn from_private_key(text: &str) -> Result<Self> {
        let rest = text
            .trim()
            .strip_prefix(PRIVATE_KEY_PREFIX)
            .ok_or_else(|| anyhow!("a private key starts with PRIVATE+KEY+"))?;
        let mut parts = rest.splitn(3, '+');
        let (Some(name), Some(id), Some(data)) = (parts.next(), parts.next(), parts.next()) else {
            bail!("a private key is PRIVATE+KEY+<name>+<key ID>+<key>");
        };
        let expected = key_id(id)?;
        let data = STANDARD
            .decode(data)
            .context("a private key's key is base64")?;
        let Some((&ALG_ED25519, seed)) = data.split_first() else {
            bail!("the private key isn't an Ed25519 key");
        };
        let seed: [u8; SEED_SIZE] = seed
            .try_into()
            .map_err(|_| anyhow!("an Ed25519 private key has a 32-byte seed"))?;
        let signer = NoteSigner::from_seed(name, &seed)?;
        ensure!(
            signer.key_hash == expected,
            "the private key's key ID doesn't match its key"
        );
        Ok(signer)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn vkey(&self) -> String {
        let mut data = vec![ALG_ED25519];
        data.extend_from_slice(self.key.verifying_key().as_bytes());
        format!(
            "{}+{}+{}",
            self.name,
            hex::encode(self.key_hash),
            STANDARD.encode(data)
        )
    }

    /// The note: the text, a blank line and this key's signature line.
    pub fn sign(&self, text: &str) -> Result<String> {
        check_text(text)?;
        let signature = self.key.sign(text.as_bytes());
        let mut sig = self.key_hash.to_vec();
        sig.extend_from_slice(&signature.to_bytes());
        Ok(format!(
            "{text}\n{SIGNATURE_PREFIX}{} {}\n",
            self.name,
            STANDARD.encode(sig)
        ))
    }
}

/// A note text ends with a newline, has no blank line and no control
/// characters other than newline.
pub fn check_text(text: &str) -> Result<()> {
    ensure!(text.ends_with('\n'), "a note text ends with a newline");
    ensure!(!text.contains("\n\n"), "a note text has no blank line");
    ensure!(
        !text.chars().any(|c| c.is_control() && c != '\n'),
        "a note text has no control characters"
    );
    Ok(())
}

/// The text and the signature lines of a signed note.
pub fn split_note(note: &str) -> Result<(&str, Vec<&str>)> {
    let at = note
        .find("\n\n")
        .ok_or_else(|| anyhow!("a signed note has a blank line before its signatures"))?;
    let (text, signatures) = (&note[..=at], &note[at + 2..]);
    check_text(text)?;
    ensure!(
        signatures.ends_with('\n'),
        "a signed note ends with a newline"
    );
    let lines: Vec<&str> = signatures[..signatures.len() - 1].split('\n').collect();
    for line in &lines {
        let rest = line
            .strip_prefix(SIGNATURE_PREFIX)
            .ok_or_else(|| anyhow!("a signature line starts with an em dash"))?;
        let (name, sig) = rest
            .split_once(' ')
            .ok_or_else(|| anyhow!("a signature line is <name> <base64>"))?;
        check_name(name)?;
        let sig = STANDARD.decode(sig).context("a signature is base64")?;
        ensure!(
            sig.len() > KEY_HASH_SIZE,
            "a signature has a key ID and bytes"
        );
    }
    Ok((text, lines))
}

/// Checks that `note` has the shape of a signed note.
pub fn check_note(note: &str) -> Result<()> {
    split_note(note).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    const SIGNATURE_SIZE: usize = 64;

    const PETER_KEY: &str =
        "PRIVATE+KEY+PeterNeumann+c74f20a3+AYEKFALVFGyNhPJEMzD1QIDr+Y7hfZx09iUvxdXHKDFz";
    const PETER_VKEY: &str = "PeterNeumann+c74f20a3+ARpc2QcUPDhMQegwxbzhKqiBfsVkmqq/LDE4izWy10TW";
    const PETER_TEXT: &str = "If you think cryptography is the answer to your problem,\n\
                              then you don't know what your problem is.\n";
    const PETER_SIG: &str = "\u{2014} PeterNeumann x08go/ZJkuBS9UG/SffcvIAQxVBtiFupLLr8pAcElZInNIuGUgYN1FFYC2pZSNXgKvqfqdngotpRZb6KE6RyyBwJnAM=\n";

    #[test]
    fn signs_as_the_sumdb_note_vector() {
        let signer = NoteSigner::from_private_key(PETER_KEY).unwrap();
        assert_eq!(signer.name(), "PeterNeumann");
        assert_eq!(signer.vkey(), PETER_VKEY);
        let note = signer.sign(PETER_TEXT).unwrap();
        assert_eq!(note, format!("{PETER_TEXT}\n{PETER_SIG}"));
        let (text, sigs) = split_note(&note).unwrap();
        assert_eq!(text, PETER_TEXT);
        assert_eq!(sigs.len(), 1);
    }

    #[test]
    fn signatures_verify_with_the_vkey() {
        let signer = NoteSigner::from_seed("board.example", &[7; 32]).unwrap();
        let vkey = Vkey::parse(&signer.vkey()).unwrap();
        assert_eq!(vkey.alg, ALG_ED25519);
        let note = signer.sign("vitrine/x\n").unwrap();
        let line = split_note(&note).unwrap().1[0];
        let b64 = line.rsplit(' ').next().unwrap();
        let raw = STANDARD.decode(b64).unwrap();
        assert_eq!(raw[..4], vkey.key_hash);
        let key = VerifyingKey::from_bytes(&vkey.key.clone().try_into().unwrap()).unwrap();
        let sig = Signature::from_slice(&raw[4..4 + SIGNATURE_SIZE]).unwrap();
        assert!(key.verify(b"vitrine/x\n", &sig).is_ok());
    }

    #[test]
    fn rejects_malformed_keys_and_texts() {
        assert!(NoteSigner::from_private_key(&PETER_KEY.replace("c74f20a3", "c74f20a4")).is_err());
        assert!(NoteSigner::from_private_key(&PETER_KEY.replace("PRIVATE", "PUBLIC")).is_err());
        assert!(Vkey::parse(&PETER_VKEY.replace("c74f20a3", "00000000")).is_err());
        assert!(Vkey::parse("no-key").is_err());
        let signer = NoteSigner::from_private_key(PETER_KEY).unwrap();
        assert!(signer.sign("no newline").is_err());
        assert!(signer.sign("blank\n\nline\n").is_err());
        assert!(signer.sign("tab\there\n").is_err());
        assert!(check_note("hello").is_err());
        assert!(check_note("text\n\nnot a signature\n").is_err());
    }
}
