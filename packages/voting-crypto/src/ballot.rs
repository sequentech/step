// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ciphertext format shared by the voter, ballot box and tally.

use anyhow::{ensure, Context as _, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::groups::Ristretto255Group;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::Canonical;
use sha2::{Digest, Sha256};
use wbraid::trustee::ballot_encryption_context;

/// A single ciphertext carries the platform's 30-byte ballot codec payload.
pub const CIPHERTEXT_WIDTH: usize = 1;

/// One contest or a whole ballot, in canonical wbraid encoding.
#[derive(Clone, Debug, PartialEq, Canonical)]
pub struct BallotCiphertext<C: Context = RistrettoCtx> {
    /// The contests this ciphertext answers, in ballot style order.
    pub contest_ids: Vec<String>,
    /// The proof of well-formedness is part of the Naor–Yung ciphertext.
    pub ciphertext: naoryung::Ciphertext<C, CIPHERTEXT_WIDTH>,
}

impl<C: Context> BallotCiphertext<C> {
    /// Construct a stored ballot entry from an encrypted payload.
    pub fn new(contest_ids: Vec<String>, ciphertext: naoryung::Ciphertext<C, 1>) -> Self {
        Self {
            contest_ids,
            ciphertext,
        }
    }

    /// Padded standard base64 of the canonical contest IDs and ciphertext.
    pub fn encode(&self) -> String {
        STANDARD.encode(self.ser())
    }

    /// Read exactly one canonical entry.
    pub fn parse(encoded: &str) -> Result<Self> {
        let bytes = STANDARD
            .decode(encoded)
            .context("ballot ciphertext is not base64")?;
        Self::deser(&bytes).context("ballot ciphertext cannot be deserialized")
    }

    /// SHA-256 of the ciphertext's randomness commitment `u_b = g^r`. Each
    /// encryption draws fresh randomness, so two entries share a fingerprint only
    /// when one carries the other's ciphertext, whatever contests they name.
    pub fn fingerprint(&self) -> [u8; 32] {
        Sha256::digest(self.ciphertext.u_b.ser()).into()
    }

    /// Consume the entry for publication in a wbraid Ballots message.
    pub fn into_ciphertext(self) -> naoryung::Ciphertext<C, 1> {
        self.ciphertext
    }

    /// Check the proof under the same context the trustees use before mixing.
    pub fn verify(&self, configuration_id: u128, joint_key: &C::Element) -> Result<()> {
        let label = ballot_encryption_context::<C>(configuration_id, joint_key);
        let public_key =
            naoryung::PublicKey::augment(&elgamal::PublicKey::<C>::new(joint_key.clone()), &label)?;
        public_key.strip(self.ciphertext.clone(), &label)?;
        Ok(())
    }
}

/// Encrypt a codec payload, keeping randomness for the voter's audit opening.
pub fn encrypt_payload(
    configuration_id: u128,
    joint_key: &<RistrettoCtx as Context>::Element,
    payload: &[u8; 30],
) -> Result<(
    naoryung::Ciphertext<RistrettoCtx, 1>,
    <RistrettoCtx as Context>::Scalar,
)> {
    let randomness = RistrettoCtx::random_scalar();
    let ciphertext =
        encrypt_payload_with_randomness(configuration_id, joint_key, payload, &randomness)?;
    Ok((ciphertext, randomness))
}

/// Reconstruct an encrypted ballot from the audit opening.
pub fn encrypt_payload_with_randomness(
    configuration_id: u128,
    joint_key: &<RistrettoCtx as Context>::Element,
    payload: &[u8; 30],
    randomness: &<RistrettoCtx as Context>::Scalar,
) -> Result<naoryung::Ciphertext<RistrettoCtx, 1>> {
    let label = ballot_encryption_context::<RistrettoCtx>(configuration_id, joint_key);
    let public_key = naoryung::PublicKey::augment(
        &elgamal::PublicKey::<RistrettoCtx>::new(joint_key.clone()),
        &label,
    )?;
    let encoded = Ristretto255Group::encode_30_bytes(payload)?;
    Ok(public_key.encrypt_with_r(&[encoded], &[randomness.clone()], &label)?)
}

/// Check that an audit opening reconstructs the ciphertext's encrypted values.
/// The randomized proof is verified independently, rather than compared bytewise.
pub fn verify_opening(
    entry: &BallotCiphertext,
    configuration_id: u128,
    joint_key: &<RistrettoCtx as Context>::Element,
    payload: &[u8; 30],
    randomness: &<RistrettoCtx as Context>::Scalar,
) -> Result<()> {
    entry.verify(configuration_id, joint_key)?;
    let reconstructed =
        encrypt_payload_with_randomness(configuration_id, joint_key, payload, randomness)?;
    ensure!(
        entry.ciphertext.u_b == reconstructed.u_b
            && entry.ciphertext.v_b == reconstructed.v_b
            && entry.ciphertext.u_a == reconstructed.u_a,
        "audit opening does not match the encrypted ballot"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(contest_ids: &[&str]) -> BallotCiphertext {
        let joint_key = RistrettoCtx::random_element();
        let (ciphertext, _) = encrypt_payload(7, &joint_key, &[1; 30]).expect("encrypts");
        BallotCiphertext::new(
            contest_ids.iter().map(|id| id.to_string()).collect(),
            ciphertext,
        )
    }

    #[test]
    fn a_copied_ciphertext_keeps_its_fingerprint_under_any_contest() {
        let original = entry(&["contest"]);
        let copy = BallotCiphertext::new(vec!["other".to_string()], original.ciphertext.clone());

        assert_eq!(copy.fingerprint(), original.fingerprint());
        assert_ne!(entry(&["contest"]).fingerprint(), original.fingerprint());
    }
}
