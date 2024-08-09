// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::{Rng, thread_rng};

pub fn generate_random_password(bytes_length: usize) -> String {
    // Define the character set: ASCII letters, numbers, and common symbols
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                           abcdefghijklmnopqrstuvwxyz\
                           0123456789\
                           !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

    // Initialize the random number generator
    let mut rng = thread_rng();

    // Generate a random password of the specified length
    let password: String = (0..bytes_length)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect();

    password
}