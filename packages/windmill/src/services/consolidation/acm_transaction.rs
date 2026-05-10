// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic pseudo-transaction ids for Miru EML headers.

use chrono::Utc;
use chrono::{Datelike, Timelike};
use rand::Rng;

/// The random part comes from dividing 9999999999999 / (24*365*24*3600)
const RANDOM_PART: u64 = 13212;

/// Builds a 13-digit number like 1721184531864
///
/// # Panics
///
/// Panics only if arithmetic around year/hour/second components or the final product overflows
/// (should not occur for real wall-clock times).
pub fn generate_transaction_id() -> u64 {
    let now = Utc::now();
    let year = (now.year() as u64)
        .checked_sub(2023u64)
        .expect("year component underflow"); // Last two digits of the year (offset base)
    let day = now.ordinal() as u64; // Day of the year (1 to 366)
    let hour = (now.hour() as u64)
        .checked_add(1)
        .expect("hour component overflow");
    let second = (now.second() as u64)
        .checked_add(1)
        .expect("second component overflow");

    let first_part = year
        .checked_mul(day)
        .and_then(|v| v.checked_mul(hour))
        .and_then(|v| v.checked_mul(second))
        .expect("transaction id first part overflow");

    let mut rng = rand::thread_rng();
    let random_part: u64 = rng.gen_range(1..=RANDOM_PART);

    first_part
        .checked_mul(random_part)
        .expect("transaction id overflow")
}
