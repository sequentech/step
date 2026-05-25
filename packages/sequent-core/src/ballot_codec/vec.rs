// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Similar to `ballot_codec::encode_vec_to_array` but it doesn't add the size.
///
/// # Errors
/// Returns an error if the input data is longer than 30 bytes.
pub fn vec_to_30_array(data: &[u8]) -> Result<[u8; 30], String> {
    if data.len() > 30 {
        return Err(format!(
            "Data too long, length {} is greater than 30",
            data.len()
        ));
    }
    let mut plaintext_array = [0u8; 30];
    if let Some(slice) = plaintext_array.get_mut(..data.len()) {
        slice.copy_from_slice(data);
    } else {
        return Err("Internal error: failed to copy data".to_string());
    }
    Ok(plaintext_array)
}

/// Encode an input byte slice into an array of 30 bytes.
/// The first byte will indicate the size of the input bytes.
/// Then follows the input bytes, and the remaining are zeroed bytes.
///
/// # Errors
/// Returns an error if the input data is longer than 29 bytes.
pub fn encode_vec_to_array(data: &[u8]) -> Result<[u8; 30], String> {
    let plaintext_length = data.len();
    if plaintext_length > 29 {
        return Err(format!(
            "Plaintext too long, length {plaintext_length} is greater than 29. Data: {data:?}"
        ));
    }
    let mut plaintext_array = [0u8; 30];
    plaintext_array[0] = u8::try_from(plaintext_length)
        .map_err(|e| format!("Error converting plaintext length to u8: {e}"))?;
    let end = plaintext_length
        .checked_add(1)
        .ok_or("Overflow in plaintext length addition")?;
    let slice = plaintext_array
        .get_mut(1..end)
        .ok_or_else(|| "Internal error: failed to copy data".to_string())?;

    slice.copy_from_slice(data);
    Ok(plaintext_array)
}

/**
 * Decode an array of 30 bytes into a vector of bytes.
 * This is the inverse of `encode_vec_to_array` and in that way
 * the first byte indicates the size of the data.
 */
#[must_use]
pub fn decode_array_to_vec(code: &[u8; 30]) -> Vec<u8> {
    let plaintext_length = code[0] as usize;
    let mut plaintext_bytes = Vec::new();
    for i in 0..plaintext_length {
        if let Some(idx) = i.checked_add(1) {
            if let Some(&value) = code.get(idx) {
                plaintext_bytes.push(value);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    plaintext_bytes
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;

    #[test]
    fn test_encode_vec_to_array_and_back() {
        let data: Vec<u8> = vec![33, 13, 155];
        let encoded = encode_vec_to_array(&data).unwrap();
        let decoded = decode_array_to_vec(&encoded);
        assert_eq!(data, decoded);
    }
}
