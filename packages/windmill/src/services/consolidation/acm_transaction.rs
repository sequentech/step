// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use chrono::Utc;
use chrono::{Datelike, Timelike};
use rand::Rng;

// the random part comes from dividing 9999999999999 / (24*365*24*3600)
const RANDOM_PART: u64 = 13212;

// generate a 13 digit number like 1721184531864
pub fn generate_transaction_id() -> u64 {
    let now = Utc::now();
    let year = now.year() as u64 - 2023u64; // Get last two digits of the year
    let day = now.ordinal() as u64; // Get the day of the year (1 to 366)
    let hour = now.hour() as u64 + 1u64;
    let second = now.second() as u64 + 1u64;

    // Calculate the first part of the number: year * day * hour * second
    let first_part = year * day * hour * second;

    // Generate the random part to fill up to 8 digits
    let mut rng = rand::thread_rng();
    let random_part: u64 = rng.gen_range(1..=RANDOM_PART);

    // Combine the two parts
    let final_number = first_part * random_part;

    final_number
}
