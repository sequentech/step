// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use chrono::Utc;
use rand::Rng;
use chrono::Datelike;

pub fn generate_transaction_id() -> u32 {
    let now = Utc::now();
    let year = now.year() as u32 - 2024u32; // Get last two digits of the year
    let day = now.ordinal() as u32; // Get the day of the year (1 to 366)

    // Calculate the first part of the number: year * day
    let first_part = year * day as u32;

    // Generate the random part to fill up to 8 digits
    let mut rng = rand::thread_rng();
    let random_part: u32 = rng.gen_range(1..=11415);

    // Combine the two parts
    let final_number = first_part * random_part;

    final_number
}
