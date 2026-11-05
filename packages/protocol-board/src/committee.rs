// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustees of one ceremony, with the keys braid needs from them.
//! Whether a committee itself is acceptable by the protocol is not
//! our concern; it is handled by `Configuration::new`.

use anyhow::{anyhow, Context as _, Result};

use crate::encoding::{
    parse_share_encryption_public_key, parse_signing_public_key,
    SHARE_ENCRYPTION_PUBLIC_KEY, SIGNING_PUBLIC_KEY,
};
use crate::{Element, VerifyingKey};

/// One trustee as the database would hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTrusteeRecord {
    pub name: String,
    /// The Ed25519 key the trustee signs board messages with.
    pub signing_public_key: Option<String>,
    /// The ElGamal key its peers encrypt its DKG shares to.
    pub share_encryption_public_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommitteeMember {
    pub(crate) name: String,
    pub(crate) signing_key: VerifyingKey,
    pub(crate) share_encryption_key: Element,
}

/// The trustees of one ceremony, in `Configuration` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committee {
    members: Vec<CommitteeMember>,
}

impl Committee {
    /// The committee of a ceremony, in the order given: it becomes the
    /// `Configuration` order, which is what the protocol numbers trustees by.
    pub fn new(trustees: &[RawTrusteeRecord]) -> Result<Committee> {
        let members = trustees
            .iter()
            .map(|trustee| {
                Ok(CommitteeMember {
                    name: trustee.name.clone(),
                    signing_key: read_key(
                        &trustee.name,
                        SIGNING_PUBLIC_KEY,
                        trustee.signing_public_key.as_deref(),
                        parse_signing_public_key,
                    )?,
                    share_encryption_key: read_key(
                        &trustee.name,
                        SHARE_ENCRYPTION_PUBLIC_KEY,
                        trustee.share_encryption_public_key.as_deref(),
                        parse_share_encryption_public_key,
                    )?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Committee { members })
    }

    pub(crate) fn members(&self) -> &[CommitteeMember] {
        &self.members
    }

    pub(crate) fn signing_keys(&self) -> Vec<VerifyingKey> {
        self.members
            .iter()
            .map(|member| member.signing_key.clone())
            .collect()
    }

    pub(crate) fn share_encryption_keys(&self) -> Vec<Element> {
        self.members
            .iter()
            .map(|member| member.share_encryption_key.clone())
            .collect()
    }
}

fn read_key<T>(
    trustee: &str,
    key: &str,
    stored: Option<&str>,
    parse: fn(&str) -> Result<T>,
) -> Result<T> {
    let stored = stored
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("trustee {trustee} has no {key}"))?;
    parse(stored)
        .with_context(|| format!("failed to parse {key} of trustee {trustee}"))
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use crate::{Ctx, Rng, Scheme};
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    use cryptography::context::Context as _;
    use cryptography::cryptosystem::elgamal::KeyPair;
    use cryptography::utils::serialization::Serializable;
    use cryptography::utils::signatures::SignatureScheme;

    /// A trustee's row together with the secrets behind it, so that a test can
    /// both build a committee and run the real braid trustee that belongs to
    /// it.
    #[derive(Clone)]
    pub(crate) struct TrusteeFixture {
        pub(crate) input: RawTrusteeRecord,
        pub(crate) signing_key: <Scheme as SignatureScheme<Rng>>::Signer,
        pub(crate) share_encryption: KeyPair<Ctx>,
    }

    /// A trustee whose row carries its keys the way the new core writes them
    /// down, independently of the parsing under test.
    pub(crate) fn trustee(name: &str) -> TrusteeFixture {
        let signing_key = Ctx::gen_signing_key();
        let share_encryption = KeyPair::<Ctx>::generate();
        let verifying_key =
            <Scheme as SignatureScheme<Rng>>::verifying_key(&signing_key);
        TrusteeFixture {
            input: RawTrusteeRecord {
                name: name.to_string(),
                signing_public_key: Some(
                    <Scheme as SignatureScheme<Rng>>::verifier_to_base64_string(
                        &verifying_key,
                    )
                    .unwrap(),
                ),
                share_encryption_public_key: Some(
                    STANDARD.encode(share_encryption.pkey.y.ser()),
                ),
            },
            signing_key,
            share_encryption,
        }
    }

    /// `count` trustees named `trustee1`, `trustee2`, and so on.
    pub(crate) fn trustees(count: usize) -> Vec<TrusteeFixture> {
        (1..=count)
            .map(|n| trustee(&format!("trustee{n}")))
            .collect()
    }

    pub(crate) fn inputs(fixtures: &[TrusteeFixture]) -> Vec<RawTrusteeRecord> {
        fixtures
            .iter()
            .map(|fixture| fixture.input.clone())
            .collect()
    }

    pub(crate) fn committee_of(fixtures: &[TrusteeFixture]) -> Committee {
        Committee::new(&inputs(fixtures)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn a_trustee_whose_key_cannot_be_used_is_named_with_the_key() {
        // What the old core stored: a DER SubjectPublicKeyInfo wrapper.
        let old_der =
            "MCowBQYDK2VwAyEAy1vJM4P85hJ1WAPZpRX3/QsOT2usIAuVy4/+t5VHHDs=";
        for stored in [None, Some(""), Some(old_der)] {
            let missing = stored.is_none_or(str::is_empty);

            let mut rows = inputs(&trustees(2));
            rows[1].signing_public_key = stored.map(str::to_string);
            let error = format!("{:#}", Committee::new(&rows).unwrap_err());
            assert!(error.contains("trustee trustee2"), "{error}");
            assert!(error.contains(SIGNING_PUBLIC_KEY), "{error}");
            assert_eq!(error.contains("has no"), missing, "{error}");

            let mut rows = inputs(&trustees(2));
            rows[0].share_encryption_public_key = stored.map(str::to_string);
            let error = format!("{:#}", Committee::new(&rows).unwrap_err());
            assert!(error.contains("trustee trustee1"), "{error}");
            assert!(error.contains(SHARE_ENCRYPTION_PUBLIC_KEY), "{error}");
            assert_eq!(error.contains("has no"), missing, "{error}");
        }
    }
}
