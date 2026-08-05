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

        Ok(context.single_contest_bases())
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
