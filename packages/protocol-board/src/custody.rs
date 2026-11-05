// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Local trustee custody proofs. Only a public request and its signature leave
//! the trustee's machine; the two secrets remain in its keys file.

use crate::TrusteeSecrets;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};

/// A platform-scoped request to confirm a trustee's local keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyRequest {
    pub tenant_id: String,
    pub election_event_id: String,
    pub keys_ceremony_id: String,
    pub tally_session_id: Option<String>,
    pub trustee_name: String,
    pub signing_public_key: String,
    pub share_encryption_public_key: String,
}

/// A signature over a custody request, with no secret key material.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyProof {
    pub request: CustodyRequest,
    pub signature: String,
}

impl CustodyRequest {
    fn signing_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = b"sequent-wbraid-trustee-custody-v1\0".to_vec();
        bytes.extend(serde_json::to_vec(self)?);
        Ok(bytes)
    }

    /// Confirm that the local keys match both registered public keys, then sign.
    pub fn attest(&self, secrets: &TrusteeSecrets) -> Result<String> {
        let keys = secrets.public_keys()?;
        ensure!(
            keys.signing_public_key == self.signing_public_key
                && keys.share_encryption_public_key
                    == self.share_encryption_public_key,
            "local trustee keys do not match the custody request"
        );
        let (signer, _) = secrets.clone().into_parts();
        let sk = SigningKey::from_b64_string(
            &crate::encoding::encode_signing_key(&signer)?,
        )?;
        let proof = CustodyProof {
            request: self.clone(),
            signature: sk.sign(&self.signing_bytes()?)?.to_b64_string()?,
        };
        Ok(serde_json::to_string_pretty(&proof)?)
    }

    /// Check a proof against the platform's trusted request.
    pub fn verifies(&self, proof: &str) -> Result<bool> {
        let proof: CustodyProof = serde_json::from_str(proof)?;
        if proof.request != *self {
            return Ok(false);
        }
        let key = VerificationKey::from_b64_string(&self.signing_public_key)?;
        let signature = Signature::from_b64_string(&proof.signature)?;
        Ok(key.verify(&signature, &self.signing_bytes()?).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custody_requires_the_registered_pair_and_cannot_be_replayed_in_another_session(
    ) {
        let secrets = TrusteeSecrets::generate();
        let keys = secrets.public_keys().unwrap();
        let mut request = CustodyRequest {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            keys_ceremony_id: "ceremony".into(),
            tally_session_id: None,
            trustee_name: "trustee".into(),
            signing_public_key: keys.signing_public_key,
            share_encryption_public_key: keys.share_encryption_public_key,
        };
        let proof = request.attest(&secrets).unwrap();
        assert!(request.verifies(&proof).unwrap());
        assert!(request.attest(&TrusteeSecrets::generate()).is_err());
        request.tally_session_id = Some("tally".into());
        assert!(!request.verifies(&proof).unwrap());
        let proof = request.attest(&secrets).unwrap();
        request.tally_session_id = Some("other-tally".into());
        assert!(!request.verifies(&proof).unwrap());
    }
}
