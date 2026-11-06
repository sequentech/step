// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{ballot::Contest, ballot_codec::ContestCodecContext};
use anyhow::Result;

pub trait BasesCodec {
    // get bases (no write-ins)
    fn get_bases(&self) -> Result<Vec<u64>>;
}

impl BasesCodec for Contest {
    fn get_bases(&self) -> Result<Vec<u64>> {
        let context = ContestCodecContext::new(self)
            .map_err(|message| anyhow::anyhow!("{}", message))?;

        context
            .single_contest_bases()
            .map_err(|message| anyhow::anyhow!("{}", message))
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::bases_fixture;
    use crate::fixtures::ballot_codec::get_configurable_contest;
    use crate::fixtures::ballot_codec::get_fixtures;
    use crate::types::ceremonies::CountingAlgType;

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

    #[test]
    fn test_get_bases_rejects_negative_max_votes() {
        for max_votes in [-1, -2, i64::MIN] {
            let contest = get_configurable_contest(
                max_votes,
                3,
                CountingAlgType::InstantRunoff,
                false,
                None,
                false,
            );
            assert!(contest.get_bases().is_err(), "max_votes {max_votes}");
        }
    }

    #[test]
    fn test_get_bases_accepts_zero_max_votes() {
        let contest = get_configurable_contest(
            0,
            3,
            CountingAlgType::InstantRunoff,
            false,
            None,
            false,
        );
        assert_eq!(contest.get_bases().unwrap(), vec![2, 1, 1, 1]);
    }

    #[test]
    fn test_get_bases_rejects_cumulative_checkboxes_overflow() {
        let mut contest = get_configurable_contest(
            1,
            3,
            CountingAlgType::Cumulative,
            false,
            None,
            false,
        );
        contest
            .presentation
            .get_or_insert_with(Default::default)
            .cumulative_number_of_checkboxes = Some(u64::MAX);
        assert!(contest.get_bases().is_err());
    }
}
