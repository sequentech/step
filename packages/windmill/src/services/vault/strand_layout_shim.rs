// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SHIM, removed once no secret stored by the strand-era vault is kept: reading
//! a stored secret in either of the two layouts the vault has written.
//!
//! The vault encrypts with `cryptography`'s ChaCha20-Poly1305 and stores the
//! envelope in `cryptography`'s canonical encoding. Until it left strand, it
//! stored the same envelope, with the same cipher, key and fields, serialized
//! by borsh:
//!
//! | Layout | Bytes |
//! |---|---|
//! | current, `cryptography` | ciphertext length as a big-endian `u64`, ciphertext, 12-byte nonce |
//! | strand era, borsh | ciphertext length as a little-endian `u32`, ciphertext, 12-byte nonce |
//!
//! Only the current layout is ever written. Both parsers consume the whole
//! value, and no value is valid in both layouts: a current envelope of `n`
//! bytes starts with the big-endian ciphertext length `n - 20`. Below 4 GiB of
//! ciphertext its high four bytes are zero, which the strand-era reading takes
//! for an empty ciphertext in a 16-byte envelope, while a current envelope
//! holds at least 20 bytes (8 of length, 12 of nonce); with 4 GiB of ciphertext
//! or more, the strand-era length `n - 16` does not fit in its four bytes. A
//! stored secret is therefore read in the one layout it parses in, and refused
//! when it parses in neither or, should a change of `cryptography`'s encoding
//! ever make the two overlap, in both.

use anyhow::{anyhow, bail, Result};
use cryptography::utils::serialization::Deserializable;
use cryptography::utils::symm::EncryptionData;

const STRAND_ERA_LENGTH_BYTES: usize = std::mem::size_of::<u32>();
const NONCE_BYTES: usize = 12;

/// The envelope of a stored secret, from the one layout it parses in.
pub(super) fn read_stored(bytes: &[u8]) -> Result<EncryptionData> {
    match (EncryptionData::deser(bytes), read_strand_era(bytes)) {
        (Ok(current), Err(_)) => Ok(current),
        (Err(_), Ok(strand_era)) => Ok(strand_era),
        (Err(current), Err(strand_era)) => Err(anyhow!(
            "a stored secret is in neither layout: current: {current}; strand era: {strand_era}"
        )),
        (Ok(_), Ok(_)) => bail!("a stored secret parses in both layouts"),
    }
}

/// The strand-era layout: borsh's encoding of the same envelope.
fn read_strand_era(bytes: &[u8]) -> Result<EncryptionData> {
    let Some((length, rest)) = bytes.split_first_chunk::<STRAND_ERA_LENGTH_BYTES>() else {
        bail!("{} bytes hold no length", bytes.len());
    };
    let length = usize::try_from(u32::from_le_bytes(*length))?;
    let Some((encrypted_bytes, nonce)) = rest.split_at_checked(length) else {
        bail!(
            "{length} bytes of ciphertext announced, {} bytes follow",
            rest.len()
        );
    };
    let nonce: [u8; NONCE_BYTES] = nonce.try_into().map_err(|_| {
        anyhow!(
            "{} bytes end the envelope where its {NONCE_BYTES}-byte nonce should",
            nonce.len()
        )
    })?;
    Ok(EncryptionData {
        encrypted_bytes: encrypted_bytes.to_vec(),
        nonce,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cryptography::utils::serialization::Serializable;
    use cryptography::utils::symm::{decrypt, encrypt, sk_from_bytes};
    use strand::serialization::StrandSerialize;

    const KEY: [u8; 32] = [7; 32];
    const SECRET: &[u8] = b"what the vault keeps";

    /// What the strand-era vault stored: strand's own encryption and borsh.
    fn stored_by_strand(plaintext: &[u8]) -> Vec<u8> {
        strand::symm::encrypt(KEY.into(), plaintext)
            .unwrap()
            .strand_serialize()
            .unwrap()
    }

    /// What the vault stores now.
    fn stored_now(plaintext: &[u8]) -> Vec<u8> {
        encrypt(sk_from_bytes(&KEY).unwrap(), plaintext)
            .unwrap()
            .ser()
    }

    fn read(stored: &[u8]) -> Vec<u8> {
        decrypt(&sk_from_bytes(&KEY).unwrap(), &read_stored(stored).unwrap()).unwrap()
    }

    #[test]
    fn a_secret_stored_by_the_strand_vault_is_read() {
        assert_eq!(read(&stored_by_strand(SECRET)), SECRET);
    }

    #[test]
    fn a_secret_stored_now_is_read() {
        assert_eq!(read(&stored_now(SECRET)), SECRET);
    }

    /// Ciphertexts of every length up to a few hundred bytes and around 64 KiB,
    /// for the real encoder of each layout to write.
    fn envelopes() -> Vec<(Vec<u8>, [u8; NONCE_BYTES])> {
        let mut envelopes = Vec::new();
        for length in (0..=300).chain([65_535, 65_536, 65_537]) {
            for contents in [
                vec![0u8; length],
                vec![0xFF; length],
                (0..length).map(|i| i as u8).collect(),
            ] {
                envelopes.push((contents, [0u8; NONCE_BYTES]));
            }
        }
        envelopes
    }

    /// The one property the shim rests on: no value is valid in both layouts.
    #[test]
    fn no_value_is_valid_in_both_layouts() {
        for (encrypted_bytes, nonce) in envelopes() {
            let current = EncryptionData {
                encrypted_bytes: encrypted_bytes.clone(),
                nonce,
            }
            .ser();
            let strand_era = strand::symm::EncryptionData {
                encrypted_bytes,
                nonce,
            }
            .strand_serialize()
            .unwrap();

            assert!(
                read_strand_era(&current).is_err(),
                "{} bytes",
                current.len()
            );
            assert!(
                EncryptionData::deser(&strand_era).is_err(),
                "{} bytes",
                strand_era.len()
            );
            assert!(read_stored(&current).is_ok());
            assert!(read_stored(&strand_era).is_ok());
        }
    }

    #[test]
    fn a_stored_secret_cut_short_or_extended_is_refused() {
        for stored in [stored_now(SECRET), stored_by_strand(SECRET)] {
            let extended = [stored.as_slice(), &[0]].concat();
            for bad in [
                &stored[..0],
                &stored[..STRAND_ERA_LENGTH_BYTES - 1],
                &stored[..STRAND_ERA_LENGTH_BYTES + 1],
                &stored[..stored.len() - 1],
                extended.as_slice(),
            ] {
                assert!(read_stored(bad).is_err(), "{} bytes", bad.len());
            }
        }
    }
}
