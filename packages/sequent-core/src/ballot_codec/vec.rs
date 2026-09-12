// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const ENVELOPE_BYTES: usize = 30;
const MAX_PAYLOAD_BYTES: usize = ENVELOPE_BYTES - 1;

/// Pad an unprefixed payload with zeros; all 30 bytes are available to callers.
pub fn vec_to_30_array(data: &[u8]) -> Result<[u8; 30], String> {
    if data.len() > ENVELOPE_BYTES {
        return Err(format!(
            "Data too long, length {} is greater than 30",
            data.len()
        ));
    }
    let mut plaintext_array = [0; ENVELOPE_BYTES];
    plaintext_array[..data.len()].copy_from_slice(data);
    Ok(plaintext_array)
}

/// Prefix up to 29 payload bytes with their length, then pad the envelope.
pub fn encode_vec_to_array(data: &[u8]) -> Result<[u8; 30], String> {
    let plaintext_length = data.len();
    if plaintext_length > MAX_PAYLOAD_BYTES {
        return Err(format!(
            "Plaintext too long, length {} is greater than 29",
            plaintext_length
        ));
    }
    let mut plaintext_array = [0; ENVELOPE_BYTES];
    plaintext_array[0] = plaintext_length as u8;
    plaintext_array[1..=plaintext_length].copy_from_slice(data);
    Ok(plaintext_array)
}

/// Read a length-prefixed envelope, rejecting a length beyond its 29-byte payload.
///
/// Decoded ballots need not have been produced by our encoder. Validate the
/// prefix before slicing so malformed plaintext returns an error, not a panic.
pub fn decode_array_to_vec(code: &[u8; 30]) -> Result<Vec<u8>, String> {
    let plaintext_length = code[0] as usize;
    code[1..]
        .get(..plaintext_length)
        .map(|payload| payload.to_vec())
        .ok_or_else(|| {
            format!(
                "Invalid plaintext length {}, maximum is 29",
                plaintext_length
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;

    #[test]
    fn test_encode_vec_to_array_and_back() {
        let data: Vec<u8> = vec![33, 13, 155];
        let encoded = encode_vec_to_array(&data).unwrap();
        let decoded = decode_array_to_vec(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
