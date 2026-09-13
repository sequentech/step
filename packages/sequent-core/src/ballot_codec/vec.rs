// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// similar to ballot_codec::encode_vec_to_array but it doesn't add the size.
pub fn vec_to_30_array(data: &Vec<u8>) -> Result<[u8; 30], String> {
    if data.len() > 30 {
        return Err(format!(
            "Data too long, lenght {} is greater than 29",
            data.len()
        ));
    }
    let mut plaintext_array = [0u8; 30];
    for i in 0..data.len() {
        plaintext_array[i] = data[i];
    }
    Ok(plaintext_array)
}

/**
 *  Encode an input vector of bytes into an array of 30 bytes.
 *  The first byte will indicate the size of the input bytes.
 * . Then follows the input bytes, and the remaining are zeroed bytes.
 */
pub fn encode_vec_to_array(data: &Vec<u8>) -> Result<[u8; 30], String> {
    let plaintext_length = data.len();
    if plaintext_length > 29 {
        return Err(format!(
            "Plaintext too long, length {} is greater than 29. Data: {:?}",
            plaintext_length, data
        ));
    }
    let mut plaintext_array = [0u8; 30];
    plaintext_array[0] = plaintext_length as u8;
    for i in 0..plaintext_length {
        plaintext_array[i + 1] = data[i];
    }
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
    #[deny(clippy::unwrap_used, clippy::panic)]
    fn test_encode_vec_to_array_and_back(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data: Vec<u8> = vec![33, 13, 155];
        let encoded = encode_vec_to_array(&data)?;
        let decoded = decode_array_to_vec(&encoded)?;
        assert_eq!(data, decoded);
        Ok(())
    }
}
