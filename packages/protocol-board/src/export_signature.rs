// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signature over what the election verifier reads: the verification
//! data and the result statement. Each export is signed with a one-use
//! Ed25519 key, generated for it and zeroized when dropped after signing
//! (`ed25519_dalek::SigningKey` is `ZeroizeOnDrop`). The protocol manager,
//! whose key the Configuration names, endorses that public key, so the
//! verifier authenticates the export through the anchor it already has (the
//! configuration hash) and needs no other.

use anyhow::{bail, ensure, Context as _, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cryptography::context::Context as _;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::signatures::{
    SignatureScheme, Signer as _, Verifier as _,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::encoding::{encode_signing_public_key, manager_verifying_key};
use crate::{BoardManager, Ctx, Rng, Scheme, VerifyingKey};

pub const SIGNED_EXPORT_FORMAT: &str = "sequent-signed-export-1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportKind {
    VerificationData,
    ResultStatement,
}

impl ExportKind {
    fn label(self) -> &'static str {
        match self {
            ExportKind::VerificationData => "verification-data",
            ExportKind::ResultStatement => "result-statement",
        }
    }
}

/// `content` keeps its exact bytes, which are what the signature covers.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedExport {
    pub format: String,
    pub kind: ExportKind,
    pub content: Box<RawValue>,
    /// The one-use export key's Ed25519 public key, base64 of its 32 bytes.
    pub signer: String,
    /// Base64 of the export key's 64-byte Ed25519 signature over the content.
    pub signature: String,
    /// The protocol manager's Ed25519 public key, base64 of its 32 bytes.
    pub endorsed_by: String,
    /// Base64 of the protocol manager's signature over the export key.
    pub endorsement: String,
}

fn framed(parts: &[&[u8]]) -> Vec<u8> {
    let mut message = SIGNED_EXPORT_FORMAT.as_bytes().to_vec();
    for part in parts {
        message.push(0);
        message.extend_from_slice(part);
    }
    message
}

/// What the export key signs: the format, the kind and the SHA-256 of the
/// content, so that a signature over one kind of export is not valid for
/// the other.
fn content_message(kind: ExportKind, content: &[u8]) -> Result<Vec<u8>> {
    Ok(framed(&[
        kind.label().as_bytes(),
        &voting_crypto::hash::hash_sha256(content)?,
    ]))
}

/// What the protocol manager signs: the format, `export-key`, the kind and
/// the export key's base64 public key.
fn endorsement_message(kind: ExportKind, signer: &str) -> Vec<u8> {
    framed(&[b"export-key", kind.label().as_bytes(), signer.as_bytes()])
}

fn parse_signature(
    kind: ExportKind,
    encoded: &str,
) -> Result<<Scheme as SignatureScheme<Rng>>::Signature> {
    let Ok(bytes) = STANDARD.decode(encoded) else {
        bail!("{} signature is not base64", kind.label());
    };
    <<Scheme as SignatureScheme<Rng>>::Signature>::deser(&bytes)
        .map_err(|_| anyhow::anyhow!("{} signature is malformed", kind.label()))
}

impl SignedExport {
    /// The envelope's file bytes: `content` (compact JSON) signed with a
    /// fresh one-use key that `manager` endorses.
    pub fn sign(
        kind: ExportKind,
        content: &[u8],
        manager: &BoardManager,
    ) -> Result<Vec<u8>> {
        let content = std::str::from_utf8(content)?;
        let raw = RawValue::from_string(content.to_string())
            .context("export content is not JSON")?;
        let export_key = Ctx::gen_signing_key();
        let signer = encode_signing_public_key(
            &<Scheme as SignatureScheme<Rng>>::verifying_key(&export_key),
        )?;
        let signature =
            export_key.sign(&content_message(kind, raw.get().as_bytes())?);
        // The export key is needed for this one signature only: dropping it
        // zeroizes it, here and on every early return above.
        drop(export_key);
        let endorsement = manager
            .signing_key
            .sign(&endorsement_message(kind, &signer));
        let envelope = SignedExport {
            format: SIGNED_EXPORT_FORMAT.to_string(),
            kind,
            content: raw,
            signer,
            signature: STANDARD.encode(signature.ser()),
            endorsed_by: encode_signing_public_key(&manager_verifying_key(
                manager,
            ))?,
            endorsement: STANDARD.encode(endorsement.ser()),
        };
        Ok(serde_json::to_vec(&envelope)?)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let envelope: SignedExport = serde_json::from_slice(bytes)?;
        ensure!(
            envelope.format == SIGNED_EXPORT_FORMAT,
            "unsupported export format"
        );
        Ok(envelope)
    }

    /// The content, once the endorsement verifies with the protocol manager
    /// key `manager` (base64, as [`TranscriptAnchors`] holds it) and the
    /// signature with the endorsed export key.
    ///
    /// [`TranscriptAnchors`]: crate::verification::TranscriptAnchors
    pub fn verify(&self, kind: ExportKind, manager: &str) -> Result<&str> {
        ensure!(self.kind == kind, "export is a {}", self.kind.label());
        ensure!(
            self.endorsed_by == manager,
            "{} is not endorsed by the protocol manager of the Configuration",
            kind.label()
        );
        let manager_key: VerifyingKey =
            <Scheme as SignatureScheme<Rng>>::verifier_from_base64_string(
                manager,
            )?;
        manager_key
            .verify(
                &endorsement_message(kind, &self.signer),
                &parse_signature(kind, &self.endorsement)?,
            )
            .map_err(|_| {
                anyhow::anyhow!("{} endorsement does not verify", kind.label())
            })?;
        let Ok(export_key) =
            <Scheme as SignatureScheme<Rng>>::verifier_from_base64_string(
                &self.signer,
            )
        else {
            bail!("{} export key is malformed", kind.label());
        };
        export_key
            .verify(
                &content_message(kind, self.content.get().as_bytes())?,
                &parse_signature(kind, &self.signature)?,
            )
            .map_err(|_| {
                anyhow::anyhow!("{} signature does not verify", kind.label())
            })?;
        Ok(self.content.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_manager;

    fn manager_key(manager: &BoardManager) -> String {
        encode_signing_public_key(&manager_verifying_key(manager)).unwrap()
    }

    #[test]
    fn a_signed_export_verifies_with_the_signer_key_and_keeps_its_content() {
        let manager = generate_manager();
        let content = br#"{"version":1,"dkg_messages":["AAEC"],"tally_messages":null}"#;
        let bytes =
            SignedExport::sign(ExportKind::VerificationData, content, &manager)
                .unwrap();
        let envelope = SignedExport::parse(&bytes).unwrap();
        let verified = envelope
            .verify(ExportKind::VerificationData, &manager_key(&manager))
            .unwrap();
        assert_eq!(verified.as_bytes(), content);
    }

    #[test]
    fn tampered_content_does_not_verify() {
        let manager = generate_manager();
        let bytes = SignedExport::sign(
            ExportKind::ResultStatement,
            br#"{"votes":1}"#,
            &manager,
        )
        .unwrap();
        let tampered = String::from_utf8(bytes)
            .unwrap()
            .replace(r#"{"votes":1}"#, r#"{"votes":2}"#);
        let envelope = SignedExport::parse(tampered.as_bytes()).unwrap();
        let error = envelope
            .verify(ExportKind::ResultStatement, &manager_key(&manager))
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "result-statement signature does not verify"
        );
    }

    #[test]
    fn another_key_or_kind_is_refused() {
        let manager = generate_manager();
        let other = generate_manager();
        let bytes =
            SignedExport::sign(ExportKind::ResultStatement, b"{}", &manager)
                .unwrap();
        let envelope = SignedExport::parse(&bytes).unwrap();
        assert_eq!(
            envelope
                .verify(ExportKind::ResultStatement, &manager_key(&other))
                .unwrap_err()
                .to_string(),
            "result-statement is not endorsed by the protocol manager of the Configuration"
        );
        assert_eq!(
            envelope
                .verify(ExportKind::VerificationData, &manager_key(&manager))
                .unwrap_err()
                .to_string(),
            "export is a result-statement"
        );
        // A forged envelope that names the right manager still fails.
        let mut forged: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap();
        forged["endorsed_by"] = manager_key(&other).into();
        let forged = SignedExport::parse(&serde_json::to_vec(&forged).unwrap())
            .unwrap();
        assert_eq!(
            forged
                .verify(ExportKind::ResultStatement, &manager_key(&other))
                .unwrap_err()
                .to_string(),
            "result-statement endorsement does not verify"
        );
        // So does another export key, endorsed for another export.
        let second =
            SignedExport::sign(ExportKind::ResultStatement, b"{}", &manager)
                .unwrap();
        let second: serde_json::Value = serde_json::from_slice(&second).unwrap();
        let mut swapped: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap();
        swapped["signer"] = second["signer"].clone();
        let swapped =
            SignedExport::parse(&serde_json::to_vec(&swapped).unwrap()).unwrap();
        assert_eq!(
            swapped
                .verify(ExportKind::ResultStatement, &manager_key(&manager))
                .unwrap_err()
                .to_string(),
            "result-statement endorsement does not verify"
        );
    }

    #[test]
    fn a_missing_signature_is_refused() {
        let missing = br#"{"format":"sequent-signed-export-1","kind":"result-statement","content":{},"signer":"x","endorsed_by":"x","endorsement":"x"}"#;
        assert!(SignedExport::parse(missing).is_err());
        let other_format = br#"{"format":"x","kind":"result-statement","content":{},"signer":"x","signature":"x","endorsed_by":"x","endorsement":"x"}"#;
        assert_eq!(
            SignedExport::parse(other_format).err().unwrap().to_string(),
            "unsupported export format"
        );
    }
}
