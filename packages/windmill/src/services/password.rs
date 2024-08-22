// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::{thread_rng, Rng};
use tracing::instrument;

#[instrument]
pub fn generate_random_bytes(bytes_length: usize) -> Vec<u8> {
    // Initialize the random number generator
    let mut rng = thread_rng();

    // Generate a random Vec<u8> of the specified length
    let random_bytes: Vec<u8> = (0..bytes_length)
        .map(|_| rng.gen())
        .collect();

    random_bytes
}
