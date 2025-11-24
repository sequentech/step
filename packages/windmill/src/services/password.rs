// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::{thread_rng, Rng};
use tracing::{info, instrument};

#[instrument]
pub fn generate_random_string_with_charset(bytes_length: usize, charset: &str) -> String {
    // Initialize the random number generator
    let mut rng = thread_rng();

    // Convert the charset to a vector of characters
    let charset_vec: Vec<char> = charset.chars().collect();

    // Generate a random password of the specified length
    let password: String = (0..bytes_length)
        .map(|_| {
            let idx = rng.gen_range(0..charset_vec.len());
            charset_vec[idx]
        })
        .collect();

    info!("password: {}", password);

    password
}
