// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

pub fn validate_schedule(
    minutes: u16,
    timeout: u16,
) -> Result<(), &'static str> {
    if !(1..=60).contains(&minutes) {
        return Err("Self-test interval must be between 1 and 60 minutes");
    }
    if timeout < 10
        || timeout > 600
        || u32::from(timeout) + 30 > u32::from(minutes) * 60
    {
        return Err("Self-test timeout must be 10–600 seconds and leave 30 seconds before the next interval");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_disabled_sparse_and_overlapping_checks() {
        for (minutes, timeout) in
            [(0, 60), (61, 60), (15, 0), (15, 601), (1, 31)]
        {
            assert!(validate_schedule(minutes, timeout).is_err());
        }
    }
    #[test]
    fn accepts_secure_bounded_intervals() {
        for (minutes, timeout) in [(1, 10), (1, 30), (15, 60), (60, 600)] {
            assert!(validate_schedule(minutes, timeout).is_ok());
        }
    }
}
