// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::{thread_rng, Rng};
use tracing::instrument;

// Define the character set: ASCII letters, numbers, and common symbols
const PASSWORD_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                           abcdefghijklmnopqrstuvwxyz\
                           0123456789\
                           !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

#[instrument]
pub fn generate_random_password(bytes_length: usize) -> String {
    // Initialize the random number generator
    let mut rng = thread_rng();

    // Generate a random password of the specified length
    let password: String = (0..bytes_length)
        .map(|_| {
            let idx = rng.gen_range(0..PASSWORD_CHARSET.len());
            PASSWORD_CHARSET[idx] as char
        })
        .collect();

    password
}
