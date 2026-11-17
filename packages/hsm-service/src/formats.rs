// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Encodings shared by the providers and the API.

/// DER of the P-256 curve's OID, as `CKA_EC_PARAMS` holds it.
pub const P256_PARAMS: [u8; 10] = [0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
/// DER of the Ed25519 OID (1.3.101.112), as `CKA_EC_PARAMS` holds it.
pub const ED25519_PARAMS: [u8; 5] = [0x06, 0x03, 0x2b, 0x65, 0x70];

/// `CKA_EC_POINT` is a DER OCTET STRING on most tokens and the raw point on
/// some; both give the raw point.
pub fn unwrap_ec_point(value: &[u8], raw_length: usize) -> Option<Vec<u8>> {
    if value.len() == raw_length {
        return Some(value.to_vec());
    }
    if value.len() == raw_length + 2 && value[0] == 0x04 && usize::from(value[1]) == raw_length {
        return Some(value[2..].to_vec());
    }
    None
}

/// Raw `r || s` (64 bytes) to a DER ECDSA-Sig-Value.
pub fn ecdsa_raw_to_der(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.len() != 64 {
        return None;
    }
    fn integer(bytes: &[u8]) -> Vec<u8> {
        let mut value: Vec<u8> = bytes.iter().copied().skip_while(|b| *b == 0).collect();
        if value.is_empty() {
            value.push(0);
        }
        if value[0] & 0x80 != 0 {
            value.insert(0, 0);
        }
        let mut out = vec![0x02, value.len() as u8];
        out.extend(value);
        out
    }
    let mut body = integer(&raw[..32]);
    body.extend(integer(&raw[32..]));
    let mut out = vec![0x30, body.len() as u8];
    out.extend(body);
    Some(out)
}

/// SubjectPublicKeyInfo DER for a raw Ed25519 key.
pub fn ed25519_spki(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    out.extend_from_slice(raw);
    out
}

/// SubjectPublicKeyInfo DER for an uncompressed P-256 point.
pub fn p256_spki(point: &[u8]) -> Vec<u8> {
    let mut out = vec![
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ];
    out.extend_from_slice(point);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ec_points_unwrap_from_octet_strings_and_raw() {
        let raw = [7u8; 32];
        let mut wrapped = vec![0x04, 32];
        wrapped.extend(raw);
        assert_eq!(unwrap_ec_point(&wrapped, 32).unwrap(), raw);
        assert_eq!(unwrap_ec_point(&raw, 32).unwrap(), raw);
        assert!(unwrap_ec_point(&[0u8; 31], 32).is_none());
    }

    #[test]
    fn der_signatures_keep_integers_positive() {
        let mut raw = [0u8; 64];
        raw[0] = 0x80;
        raw[63] = 1;
        let der = ecdsa_raw_to_der(&raw).unwrap();
        assert_eq!(der[2], 0x02);
        assert_eq!(der[3], 33);
        assert_eq!(der[4], 0);
        assert!(ecdsa_raw_to_der(&raw[..63]).is_none());
    }

    #[test]
    fn spki_lengths() {
        assert_eq!(ed25519_spki(&[1u8; 32]).len(), 44);
        assert_eq!(p256_spki(&[4u8; 65]).len(), 91);
    }
}
