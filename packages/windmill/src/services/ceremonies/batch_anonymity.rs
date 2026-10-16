// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Whether the batches a tally session is about to post for one contest area
//! keep each decrypted vote among others, and what `BatchAnonymityPolicy` does
//! when they do not.

use anyhow::{anyhow, Result};
use sequent_core::ballot::BatchAnonymityPolicy;
use sequent_core::types::keycloak::MIN_WEIGHT_BATCH_ANONYMITY;
use std::fmt;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use tracing::{event, Level};

/// A batch another tally session already posted for the same election, area
/// and contest.
#[derive(Debug, Clone)]
pub struct PostedBatch {
    pub tally_session_id: String,
    pub offset: u32,
    pub ciphertexts: Vec<Ciphertext<RistrettoCtx>>,
}

/// Why a batch would not keep its decrypted votes among others.
#[derive(Debug, PartialEq, Eq)]
pub enum BatchAnonymityIssue {
    TooFewVoters {
        offset: u32,
        voters: usize,
    },
    DiffersFromSession {
        tally_session_id: String,
        offset: u32,
    },
}

impl fmt::Display for BatchAnonymityIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVoters { offset, voters } => write!(
                f,
                "batch offset {offset} holds the ballots of {voters} voter(s), fewer than \
                 {MIN_WEIGHT_BATCH_ANONYMITY}"
            ),
            Self::DiffersFromSession {
                tally_session_id,
                offset,
            } => write!(
                f,
                "batch offset {offset} differs from the ballots tally session \
                 {tally_session_id} already posted for the same election, area and contest"
            ),
        }
    }
}

/// A non-empty batch holding the ballots of fewer voters than the minimum.
pub fn small_batch(offset: u32, voters: usize) -> Option<BatchAnonymityIssue> {
    (voters > 0 && voters < MIN_WEIGHT_BATCH_ANONYMITY)
        .then_some(BatchAnonymityIssue::TooFewVoters { offset, voters })
}

/// Compares what this run built with every other session that posted
/// ballots for the same contest area, batch offset by batch offset. A batch
/// missing on either side counts as empty, so a voter moving into or out of a
/// batch is caught too. A session that posted only empty batches decrypted
/// nothing and is not compared.
pub fn session_divergences(
    built: &[Vec<Ciphertext<RistrettoCtx>>],
    posted_elsewhere: &[PostedBatch],
) -> Vec<BatchAnonymityIssue> {
    let mut sessions: Vec<&str> = posted_elsewhere
        .iter()
        .filter(|posted| !posted.ciphertexts.is_empty())
        .map(|posted| posted.tally_session_id.as_str())
        .collect();
    sessions.sort_unstable();
    sessions.dedup();

    let offsets = posted_elsewhere
        .iter()
        .map(|posted| posted.offset as usize + 1)
        .chain(std::iter::once(built.len()))
        .max()
        .unwrap_or_default();

    let mut issues = Vec::new();
    for tally_session_id in sessions {
        for offset in 0..offsets {
            let posted = posted_elsewhere
                .iter()
                .find(|posted| {
                    posted.tally_session_id == tally_session_id && posted.offset as usize == offset
                })
                .map(|posted| posted.ciphertexts.as_slice())
                .unwrap_or_default();
            let built = built
                .get(offset)
                .map(|built| built.as_slice())
                .unwrap_or_default();
            if posted != built {
                issues.push(BatchAnonymityIssue::DiffersFromSession {
                    tally_session_id: tally_session_id.to_string(),
                    offset: offset as u32,
                });
            }
        }
    }
    issues
}

/// Logs the issues under `WARN` and refuses the contest area under `REFUSE`.
pub fn enforce_batch_anonymity(
    policy: &BatchAnonymityPolicy,
    election_id: &str,
    area_id: &str,
    issues: &[BatchAnonymityIssue],
) -> Result<()> {
    if issues.is_empty() {
        return Ok(());
    }
    let details = issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    match policy {
        BatchAnonymityPolicy::WARN => {
            event!(
                Level::WARN,
                "Ballots of election {election_id} area {area_id}: {details}"
            );
            Ok(())
        }
        BatchAnonymityPolicy::REFUSE => Err(anyhow!(
            "Refusing to extract the ballots of election {election_id} area {area_id} under \
             the refuse batch anonymity policy: {details}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::keycloak::VOTE_WEIGHT_BATCHES;
    use strand::context::Ctx;

    fn pool(size: usize) -> Vec<Ciphertext<RistrettoCtx>> {
        let ctx = RistrettoCtx::default();
        let mut rng = ctx.get_rng();
        (0..size)
            .map(|_| Ciphertext {
                mhr: ctx.rnd(&mut rng),
                gr: ctx.rnd(&mut rng),
            })
            .collect()
    }

    fn unweighted(batch: &[Ciphertext<RistrettoCtx>]) -> Vec<Vec<Ciphertext<RistrettoCtx>>> {
        let mut batches = vec![Vec::new(); VOTE_WEIGHT_BATCHES as usize];
        batches[0] = batch.to_vec();
        batches
    }

    fn posted(
        tally_session_id: &str,
        offset: u32,
        ciphertexts: &[Ciphertext<RistrettoCtx>],
    ) -> PostedBatch {
        PostedBatch {
            tally_session_id: tally_session_id.to_string(),
            offset,
            ciphertexts: ciphertexts.to_vec(),
        }
    }

    #[test]
    fn flags_a_batch_below_the_minimum_number_of_voters() {
        assert_eq!(
            small_batch(0, 1),
            Some(BatchAnonymityIssue::TooFewVoters {
                offset: 0,
                voters: 1
            })
        );
        assert_eq!(
            small_batch(3, MIN_WEIGHT_BATCH_ANONYMITY - 1),
            Some(BatchAnonymityIssue::TooFewVoters {
                offset: 3,
                voters: MIN_WEIGHT_BATCH_ANONYMITY - 1
            })
        );
    }

    #[test]
    fn accepts_an_empty_batch_and_one_at_the_minimum() {
        assert_eq!(small_batch(0, 0), None);
        assert_eq!(small_batch(0, MIN_WEIGHT_BATCH_ANONYMITY), None);
    }

    #[test]
    fn refuses_a_single_voter_batch_under_refuse_policy() {
        let issues: Vec<_> = small_batch(0, 1).into_iter().collect();
        assert!(enforce_batch_anonymity(
            &BatchAnonymityPolicy::REFUSE,
            "election",
            "area",
            &issues
        )
        .is_err());
    }

    #[test]
    fn allows_a_single_voter_batch_under_warn_policy() {
        let issues: Vec<_> = small_batch(0, 1).into_iter().collect();
        assert!(
            enforce_batch_anonymity(&BatchAnonymityPolicy::WARN, "election", "area", &issues)
                .is_ok()
        );
    }

    #[test]
    fn allows_a_batch_with_no_issues_under_refuse_policy() {
        assert!(
            enforce_batch_anonymity(&BatchAnonymityPolicy::REFUSE, "election", "area", &[]).is_ok()
        );
    }

    #[test]
    fn flags_a_batch_that_lost_a_voter_since_another_session() {
        let pool = pool(6);
        let earlier = [posted("earlier", 0, &pool)];
        assert_eq!(
            session_divergences(&unweighted(&pool[..5]), &earlier),
            vec![BatchAnonymityIssue::DiffersFromSession {
                tally_session_id: "earlier".to_string(),
                offset: 0,
            }]
        );
    }

    #[test]
    fn flags_a_batch_that_gained_a_voter_since_another_session() {
        let pool = pool(6);
        let earlier = [posted("earlier", 0, &pool[..5])];
        assert_eq!(session_divergences(&unweighted(&pool), &earlier).len(), 1);
    }

    #[test]
    fn accepts_a_batch_identical_to_another_session() {
        let pool = pool(6);
        let earlier = [posted("earlier", 0, &pool)];
        assert!(session_divergences(&unweighted(&pool), &earlier).is_empty());
    }

    #[test]
    fn ignores_an_empty_batch_posted_by_another_session() {
        let pool = pool(6);
        let earlier = [posted("initialization", 0, &[])];
        assert!(session_divergences(&unweighted(&pool), &earlier).is_empty());
    }

    #[test]
    fn compares_weighted_batches_offset_by_offset() {
        let pool = pool(6);
        let mut built = vec![Vec::new(); VOTE_WEIGHT_BATCHES as usize];
        built[0] = pool[..5].to_vec();
        built[1] = pool[1..6].to_vec();
        let same = [
            posted("earlier", 0, &pool[..5]),
            posted("earlier", 1, &pool[1..6]),
        ];
        assert!(session_divergences(&built, &same).is_empty());

        let swapped = [
            posted("earlier", 0, &pool[1..6]),
            posted("earlier", 1, &pool[..5]),
        ];
        assert_eq!(session_divergences(&built, &swapped).len(), 2);
    }

    #[test]
    fn flags_a_batch_another_session_posted_that_this_run_leaves_empty() {
        let pool = pool(6);
        let earlier = [
            posted("earlier", 0, &pool),
            posted("earlier", 2, &pool[..5]),
        ];
        assert_eq!(
            session_divergences(&unweighted(&pool), &earlier),
            vec![BatchAnonymityIssue::DiffersFromSession {
                tally_session_id: "earlier".to_string(),
                offset: 2,
            }]
        );
    }

    #[test]
    fn flags_a_batch_this_run_fills_that_another_session_left_empty() {
        let pool = pool(6);
        let earlier = [posted("earlier", 0, &pool[..5])];
        let mut built = unweighted(&pool[..5]);
        built[1] = pool[1..6].to_vec();
        assert_eq!(
            session_divergences(&built, &earlier),
            vec![BatchAnonymityIssue::DiffersFromSession {
                tally_session_id: "earlier".to_string(),
                offset: 1,
            }]
        );
    }

    #[test]
    fn compares_with_every_other_session() {
        let pool = pool(6);
        let earlier = [posted("first", 0, &pool), posted("second", 0, &pool[..5])];
        assert_eq!(
            session_divergences(&unweighted(&pool), &earlier),
            vec![BatchAnonymityIssue::DiffersFromSession {
                tally_session_id: "second".to_string(),
                offset: 0,
            }]
        );
    }

    #[test]
    fn refuses_a_batch_differing_from_another_session_under_refuse_policy() {
        let pool = pool(6);
        let earlier = [posted("earlier", 0, &pool)];
        let issues = session_divergences(&unweighted(&pool[..5]), &earlier);
        assert!(enforce_batch_anonymity(
            &BatchAnonymityPolicy::REFUSE,
            "election",
            "area",
            &issues
        )
        .is_err());
        assert!(
            enforce_batch_anonymity(&BatchAnonymityPolicy::WARN, "election", "area", &issues)
                .is_ok()
        );
    }
}
