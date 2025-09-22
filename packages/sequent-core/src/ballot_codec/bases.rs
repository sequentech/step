// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use anyhow::Result;
use std::convert::TryInto;

pub trait BasesCodec {
    // get bases (no write-ins)
    fn get_bases(&self) -> Result<Vec<u64>>;
}

impl BasesCodec for Contest {
    fn get_bases(&self) -> Result<Vec<u64>> {
        // Calculate the base for candidates. It depends on the
        // `contest.counting_algorithm`:
        // - plurality-at-large: base 2 (value can be either 0 o 1)
        // - preferential (*bordas*): contest.max + 1
        // - cummulative: contest.extra_options.cumulative_number_of_checkboxes
        //   + 1

        let candidate_base: u64 = match self.get_counting_algorithm().as_str() {
            "plurality-at-large" => 2,
            "cumulative" => self.cumulative_number_of_checkboxes() + 1u64,
            _ => (self.max_votes + 1i64).try_into().unwrap(),
        };

        let num_valid_candidates: usize = self
            .candidates
            .iter()
            .filter(|candidate| !candidate.is_explicit_invalid())
            .count();

        // Set the initial bases and raw ballot, populate bases using the valid
        // candidates list
        let mut bases: Vec<u64> = vec![2];
        for _i in 0..num_valid_candidates {
            bases.push(candidate_base);
        }

        // Add bases for null terminators.
        if self.allow_writeins() {
            let char_map = self.get_char_map();
            let write_in_base = char_map.base();
            for candidate in self.candidates.iter() {
                if candidate.is_write_in() {
                    bases.push(write_in_base);
                }
            }
        }

        Ok(bases)
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::bases_fixture;
    use crate::fixtures::ballot_codec::get_fixtures;

    #[test]
    fn test_contest_bases() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);

            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("contest_bases").cloned()
                });

            if expected_error.is_some() {
                assert_ne!(
                    &fixture.contest.get_bases().unwrap(),
                    &fixture.raw_ballot.bases
                );
            } else {
                assert_eq!(
                    &fixture.contest.get_bases().unwrap(),
                    &fixture.raw_ballot.bases
                );
            }
        }
    }

    #[test]
    fn test_bases() {
        let fixtures = bases_fixture();
        for fixture in fixtures.iter() {
            let bases = fixture.contest.get_bases().unwrap();
            assert_eq!(bases, fixture.bases);
        }
    }
}
