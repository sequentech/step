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

        context.single_contest_bases().map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::bases_fixture;
    use crate::fixtures::ballot_codec::get_fixtures;

    #[test]
    fn candidate_bases_reject_invalid_values_without_wrapping_or_panicking() {
        use crate::types::ceremonies::CountingAlgType;
        let mut contest = get_fixtures().remove(0).contest;
        contest.counting_algorithm = Some(CountingAlgType::Borda);
        contest.max_votes = -1;
        assert!(contest.get_bases().is_err());
        contest.max_votes = i64::MAX;
        assert!(contest.get_bases().unwrap().contains(&(1u64 << 63)));
        contest.counting_algorithm = Some(CountingAlgType::Cumulative);
        contest
            .presentation
            .as_mut()
            .unwrap()
            .cumulative_number_of_checkboxes = Some(u64::MAX);
        assert!(contest.get_bases().is_err());
        contest
            .presentation
            .as_mut()
            .unwrap()
            .cumulative_number_of_checkboxes = Some(u64::MAX - 1);
        assert!(contest.get_bases().unwrap().contains(&u64::MAX));
    }

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
