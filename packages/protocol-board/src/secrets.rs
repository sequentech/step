// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A trustee's two secrets and the keys file that holds them.
//!
//! The file carries the secrets only. Both public halves are derived from
//! them, so a file cannot pair a secret with a public key that is not its
//! own, and what [`TrusteeSecrets::public_keys`] gives is what an
//! administrator registers for the trustee.

use std::fmt;

use anyhow::{anyhow, Context as _, Result};
use cryptography::context::Context as _;
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::traits::groups::GroupElement as _;
use cryptography::utils::signatures::SignatureScheme;
use serde::{Deserialize, Serialize};
use wbraid::messages::artifact::Configuration;

use crate::encoding::{
    encode_share_encryption_public_key, encode_share_encryption_secret,
    encode_signing_key, encode_signing_public_key,
    parse_share_encryption_secret, parse_signing_key,
};
use crate::{Ctx, Rng, Scheme, Signer, VerifyingKey};

/// The keys file's fields, as errors name them.
const SIGNING_KEY_FIELD: &str = "signing_key";
const SHARE_ENCRYPTION_KEY_FIELD: &str = "share_encryption_key";

/// The keys file as it is written down.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeysFile {
    signing_key: String,
    share_encryption_key: String,
}

/// What a trustee signs its board messages with, and the key pair its peers
/// encrypt its DKG shares to.
#[derive(Clone)]
pub struct TrusteeSecrets {
    signing_key: Signer,
    share_encryption: KeyPair<Ctx>,
}

/// A trustee's public keys, in the forms the `trustee` row stores them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrusteePublicKeys {
    pub signing_public_key: String,
    pub share_encryption_public_key: String,
}

impl TrusteeSecrets {
    /// Fresh secrets for a new trustee.
    pub fn generate() -> TrusteeSecrets {
        TrusteeSecrets {
            signing_key: Ctx::gen_signing_key(),
            share_encryption: KeyPair::generate(),
        }
    }

    /// Read a keys file.
    pub fn parse(contents: &str) -> Result<TrusteeSecrets> {
        // The error's own rendering quotes the offending line of the file,
        // which may hold a secret: only its message is kept.
        let file: KeysFile = toml::from_str(contents).map_err(|err| {
            anyhow!("Failed parsing the trustee keys file: {}", err.message())
        })?;
        let signing_key = parse_signing_key(&file.signing_key)
            .with_context(|| keys_file_field(SIGNING_KEY_FIELD))?;
        let skey = parse_share_encryption_secret(&file.share_encryption_key)
            .with_context(|| keys_file_field(SHARE_ENCRYPTION_KEY_FIELD))?;
        let pkey = Ctx::generator().exp(&skey);
        Ok(TrusteeSecrets {
            signing_key,
            share_encryption: KeyPair::new(skey, pkey),
        })
    }

    /// The keys file of these secrets.
    pub fn to_toml(&self) -> Result<String> {
        let file = KeysFile {
            signing_key: encode_signing_key(&self.signing_key)
                .with_context(|| keys_file_field(SIGNING_KEY_FIELD))?,
            share_encryption_key: encode_share_encryption_secret(
                &self.share_encryption.skey,
            ),
        };
        toml::to_string(&file)
            .context("Failed encoding into the trustee keys file")
    }

    /// The public parts as an administrator registers them.
    pub fn public_keys(&self) -> Result<TrusteePublicKeys> {
        Ok(TrusteePublicKeys {
            signing_public_key: encode_signing_public_key(
                &self.verifying_key(),
            )?,
            share_encryption_public_key: encode_share_encryption_public_key(
                &self.share_encryption.pkey.y,
            ),
        })
    }

    /// Whether `configuration` names this trustee among its trustees.
    pub fn is_listed_in(&self, configuration: &Configuration<Ctx>) -> bool {
        configuration.trustees.contains(&self.verifying_key())
    }

    pub fn into_parts(self) -> (Signer, KeyPair<Ctx>) {
        (self.signing_key, self.share_encryption)
    }

    fn verifying_key(&self) -> VerifyingKey {
        <Scheme as SignatureScheme<Rng>>::verifying_key(&self.signing_key)
    }
}

impl fmt::Debug for TrusteeSecrets {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrusteeSecrets")
            .finish_non_exhaustive()
    }
}

fn keys_file_field(name: &str) -> String {
    format!("the trustee keys file's {name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
    use base64::Engine as _;

    /// A generated keys file, with its two values as written.
    fn written() -> (String, String, String) {
        let file = TrusteeSecrets::generate().to_toml().unwrap();
        let table: toml::Table = toml::from_str(&file).unwrap();
        let value = |name: &str| table[name].as_str().unwrap().to_string();
        let signing_key = value(SIGNING_KEY_FIELD);
        let share_encryption_key = value(SHARE_ENCRYPTION_KEY_FIELD);
        (file, signing_key, share_encryption_key)
    }

    fn keys_file(signing_key: &str, share_encryption_key: &str) -> String {
        format!(
            "{SIGNING_KEY_FIELD} = \"{signing_key}\"\n\
             {SHARE_ENCRYPTION_KEY_FIELD} = \"{share_encryption_key}\"\n"
        )
    }

    fn refusal(contents: &str) -> String {
        match TrusteeSecrets::parse(contents) {
            Ok(_) => panic!("the keys file was accepted:\n{contents}"),
            Err(err) => format!("{err:#}"),
        }
    }

    #[test]
    fn a_keys_file_reads_back_as_the_secrets_it_was_written_from() {
        let secrets = TrusteeSecrets::generate();
        let file = secrets.to_toml().unwrap();
        let read = TrusteeSecrets::parse(&file).unwrap();

        assert_eq!(read.to_toml().unwrap(), file);
        assert_eq!(read.public_keys().unwrap(), secrets.public_keys().unwrap());
        // The share-encryption public key was derived, not stored.
        assert_eq!(read.into_parts(), secrets.into_parts());
    }

    #[test]
    fn a_keys_file_holds_two_padded_32_byte_secrets_and_nothing_else() {
        let (file, _, _) = written();
        let table: toml::Table = toml::from_str(&file).unwrap();

        let mut fields = table.keys().map(String::as_str).collect::<Vec<_>>();
        fields.sort_unstable();
        assert_eq!(fields, [SHARE_ENCRYPTION_KEY_FIELD, SIGNING_KEY_FIELD]);
        for value in table.values() {
            let encoded = value.as_str().unwrap();
            assert_eq!(STANDARD.decode(encoded).unwrap().len(), 32, "{value}");
        }
    }

    #[test]
    fn a_keys_file_that_cannot_be_used_is_refused_with_the_field_named() {
        let (_, signing_key, share_encryption_key) = written();
        let seed = STANDARD.decode(&signing_key).unwrap();
        let scalar = STANDARD.decode(&share_encryption_key).unwrap();
        let mut longer_scalar = scalar.clone();
        longer_scalar.push(0);

        for bad in [
            STANDARD_NO_PAD.encode(&seed),
            STANDARD.encode(&seed[..31]),
            "not base64!".to_string(),
        ] {
            let error = refusal(&keys_file(&bad, &share_encryption_key));
            assert!(error.contains(SIGNING_KEY_FIELD), "{error}");
            assert!(!error.contains(&bad), "{error}");
        }

        for bad in [
            STANDARD_NO_PAD.encode(&scalar),
            STANDARD.encode(&scalar[..31]),
            STANDARD.encode(longer_scalar),
            // Above the group order: not the canonical form of any scalar.
            STANDARD.encode([0xffu8; 32]),
            "not base64!".to_string(),
        ] {
            let error = refusal(&keys_file(&signing_key, &bad));
            assert!(error.contains(SHARE_ENCRYPTION_KEY_FIELD), "{error}");
            assert!(!error.contains(&bad), "{error}");
        }

        let missing = format!("{SIGNING_KEY_FIELD} = \"{signing_key}\"\n");
        let error = refusal(&missing);
        assert!(error.contains(SHARE_ENCRYPTION_KEY_FIELD), "{error}");

        // The old trustee's keys file carried a public half as well.
        let old_field = "signing_key_pk";
        let unknown = format!(
            "{}{old_field} = \"{}\"\n",
            keys_file(&signing_key, &share_encryption_key),
            STANDARD.encode([1u8; 32])
        );
        let error = refusal(&unknown);
        assert!(error.contains(old_field), "{error}");
        assert!(!error.contains(&signing_key), "{error}");
    }

    #[test]
    fn a_keys_file_that_is_not_toml_is_refused_without_quoting_it() {
        let (_, signing_key, _) = written();
        let unterminated = format!("{SIGNING_KEY_FIELD} = \"{signing_key}\n");
        let error = refusal(&unterminated);
        assert!(!error.contains(&signing_key), "{error}");
    }

    #[test]
    fn debugging_the_secrets_prints_none_of_them() {
        assert_eq!(
            format!("{:?}", TrusteeSecrets::generate()),
            "TrusteeSecrets { .. }"
        );
    }
}
