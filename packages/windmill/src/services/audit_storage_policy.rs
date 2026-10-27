// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Integer-only audit-storage threshold and observation freshness policy.

/// Maximum lifetime of a successful measurement; monitors sample every 30 seconds.
pub const MAX_AGE_SECONDS: i64 = 90;
/// Used and available bytes on the filesystem containing the actual audit database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Space {
    /// Bytes occupied on the database filesystem.
    pub used: u64,
    /// Bytes available to the database, excluding reserved blocks.
    pub available: u64,
}

impl Space {
    /// Reject an impossible or empty filesystem instead of reporting a healthy sample.
    pub fn new(used: u64, available: u64) -> Result<Self, &'static str> {
        let capacity = used
            .checked_add(available)
            .ok_or("storage capacity overflow")?;
        if capacity == 0 {
            return Err("storage capacity is zero");
        }
        Ok(Self { used, available })
    }

    /// FAU_STG.4 applies strictly above 80%; integer cross multiplication avoids rounding.
    pub fn exceeded(self) -> bool {
        u128::from(self.used) * 100 > (u128::from(self.used) + u128::from(self.available)) * 80
    }
}

/// Parse one GNU df observation requested with --output=size,used,avail -B1.
pub fn parse_df(output: &str) -> Result<Space, &'static str> {
    let mut lines = output.lines();
    lines.next().ok_or("missing storage header")?;
    let values = lines.next().ok_or("missing storage measurement")?;
    if lines.any(|line| !line.trim().is_empty()) {
        return Err("multiple storage measurements");
    }
    let numbers: Vec<u64> = values
        .split_whitespace()
        .map(|value| value.parse().map_err(|_| "invalid storage number"))
        .collect::<Result<_, _>>()?;
    let [total, used, available] = numbers.as_slice() else {
        return Err("invalid storage measurement shape");
    };
    let space = Space::new(*used, *available)?;
    if *total
        < used
            .checked_add(*available)
            .ok_or("storage capacity overflow")?
    {
        return Err("storage measurement exceeds total capacity");
    }
    Ok(space)
}

/// Future-dated and expired measurements both refuse voter access.
pub fn fresh(age_seconds: i64) -> bool {
    (0..MAX_AGE_SECONDS).contains(&age_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_is_strictly_above_eighty_percent() {
        assert!(!Space::new(80, 20).unwrap().exceeded());
        assert!(Space::new(81, 19).unwrap().exceeded());
        assert!(!Space::new(0, 100).unwrap().exceeded());
        assert!(Space::new(100, 0).unwrap().exceeded());
    }

    #[test]
    fn empty_or_overflowing_capacity_is_not_healthy() {
        assert!(Space::new(0, 0).is_err());
        assert!(Space::new(u64::MAX, 1).is_err());
        assert!(Space::new(u64::MAX, 0).unwrap().exceeded());
    }

    #[test]
    fn df_uses_storage_available_to_the_database() {
        let result = parse_df("1B-blocks Used Avail\n1000 81 19\n").unwrap();
        assert!(result.exceeded());
        assert_eq!(result, Space::new(81, 19).unwrap());
    }

    #[test]
    fn malformed_or_ambiguous_measurements_are_rejected() {
        for value in [
            "",
            "header",
            "h\n0 0 0\n",
            "h\n1 -1 2\n",
            "h\n100 80 21\n",
            "h\n100 80 20 extra\n",
            "h\n100 80 20\n100 80 20",
        ] {
            assert!(parse_df(value).is_err(), "{value:?}");
        }
    }

    #[test]
    fn fresh_observation_is_mandatory() {
        assert!(fresh(0));
        assert!(fresh(89));
        assert!(!fresh(90));
        assert!(!fresh(-1));
    }
}
