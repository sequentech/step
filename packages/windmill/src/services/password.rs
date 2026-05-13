// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Password generation helpers.

use rand::{thread_rng, Rng};
use tracing::{info, instrument};

/// Generate a random string with a specified charset.
///
/// # Panics
///
/// Panics only if the random index into `charset` is out of range, which cannot
/// happen when `charset` is non-empty and `gen_range` is bounded to its length.
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
            *charset_vec
                .get(idx)
                .expect("charset index is always in range from gen_range")
        })
        .collect();

    info!("password: {}", password);

    password
}
