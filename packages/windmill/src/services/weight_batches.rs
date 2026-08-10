// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Batch layout for `VOTERS_WEIGHTED_VOTING`.
//!
//! A voter's weight is applied by placing their ciphertext in the batch for
//! each bit that weight sets, so a contest area owns `VOTE_WEIGHT_BATCHES`
//! consecutive board batches starting at its `session_id`, and the tally
//! multiplies the batch at offset `bit` by `2^bit`. Summing those multipliers
//! over the set bits reconstructs the weight, which is what makes the count
//! exact. Every other policy fills only the first batch, and the helpers here
//! collapse to the single-batch behaviour that predates weighting.

use anyhow::{anyhow, Result};
use b3::messages::{artifact::Plaintexts, message::Message};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::{TallySessionContest, TallySessionContestAnnotations};
use sequent_core::types::keycloak::{
    weight_bit_multiplier, weight_has_bit, MAX_TOTAL_VOTE_WEIGHT, VOTE_WEIGHT_BATCHES,
};
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tracing::{event, Level};

/// The batch offsets one copy of this voter's ciphertext goes into.
///
/// Errors rather than truncating on a weight too large to represent: masking it
/// would drop the part that does not fit and under-count that voter silently.
/// `merge_join_csv` already rejects those, so this is a backstop against the
/// two limits drifting apart.
pub fn weight_batch_offsets(weight: u64) -> Result<impl Iterator<Item = u32>> {
    if weight >= 1u64 << VOTE_WEIGHT_BATCHES {
        return Err(anyhow!(
            "Vote weight {weight} does not fit in {VOTE_WEIGHT_BATCHES} batches"
        ));
    }
    Ok((0..VOTE_WEIGHT_BATCHES).filter(move |bit| weight_has_bit(weight, *bit)))
}

/// The batches the area actually posted, each with the multiplier the tally
/// owes it.
///
/// Falls back to the single unweighted batch when no mask was recorded, which
/// covers every other policy and any row written before weighting existed.
/// Errors rather than falling back when the annotations are present but
/// unreadable: treating a corrupt weighted row as unweighted would count one
/// batch at multiplier 1, discard every other batch, and report the result as
/// complete.
pub fn contest_weight_batches(
    tally_session_contest: &TallySessionContest,
) -> Result<Vec<(i64, u64)>> {
    let base = tally_session_contest.session_id as i64;
    let Some(annotations) = tally_session_contest.annotations.clone() else {
        return Ok(vec![(base, 1)]);
    };
    let annotations: TallySessionContestAnnotations =
        deserialize_value(annotations).map_err(|error| {
            anyhow!(
                "Could not read annotations for tally session contest {}: {error:?}",
                tally_session_contest.id
            )
        })?;
    let Some(mask) = annotations.weight_bit_mask else {
        return Ok(vec![(base, 1)]);
    };
    (0..VOTE_WEIGHT_BATCHES)
        .filter(|bit| mask & (1u32 << bit) != 0)
        .map(|bit| {
            let multiplier = weight_bit_multiplier(bit).ok_or_else(|| {
                anyhow!("Weight batch offset {bit} is outside the batches a contest area owns")
            })?;
            Ok((base + bit as i64, multiplier))
        })
        .collect()
}

/// The area's decrypted ballots, each repeated by the multiplier its batch
/// carries, in one vector for the contest to count.
///
/// `None` while any expected batch is still unmixed: counting the batches that
/// have arrived would silently drop the weight of the ones that have not, and
/// publish a result that looks complete.
pub fn collect_weighted_plaintexts(
    tally_session_contest: &TallySessionContest,
    relevant_plaintexts: &[&Message],
) -> Result<Option<Vec<<RistrettoCtx as Ctx>::P>>> {
    let batches = contest_weight_batches(tally_session_contest)?;
    let mut found: Vec<(Vec<<RistrettoCtx as Ctx>::P>, u64)> = Vec::with_capacity(batches.len());
    for (batch, multiplier) in batches {
        // An artifact that is present but will not deserialize is a broken
        // board message, not a batch that has yet to be mixed. Mapping it to
        // the latter waits for it forever; the whole point of separating the
        // two is that only one of them ever resolves.
        let artifact = relevant_plaintexts
            .iter()
            .find(|message| batch == message.statement.get_batch_number() as i64)
            .and_then(|message| message.artifact.clone());
        let plaintexts = artifact
            .map(|artifact| {
                Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
                    .map(|plaintexts| plaintexts.0 .0)
                    .map_err(|error| {
                        anyhow!(
                            "Could not read the Plaintexts artifact for batch {batch} of \
                             tally session contest {}: {error:?}",
                            tally_session_contest.id
                        )
                    })
            })
            .transpose()?;
        let Some(plaintexts) = plaintexts else {
            event!(
                Level::INFO,
                "Expected: Plaintexts not found yet for session contest = {}, batch number = {}",
                tally_session_contest.id,
                batch
            );
            return Ok(None);
        };
        found.push((plaintexts, multiplier));
    }

    let total: u64 = found
        .iter()
        .try_fold(0u64, |acc, (plaintexts, multiplier)| {
            (plaintexts.len() as u64)
                .checked_mul(*multiplier)
                .and_then(|batch_total| acc.checked_add(batch_total))
                .ok_or_else(|| anyhow!("Weighted plaintext count overflowed"))
        })?;
    // Only where a weight actually multiplies something. The dump bounds the
    // summed weight, but it is not what runs here: the multipliers come from a
    // mask read back out of a jsonb column and the ballot counts from the
    // board, so a corrupted or hand-edited mask can ask for up to 2^17 copies
    // of every ballot, and an allocation that large aborts the process instead
    // of failing this tally.
    //
    // `MAX_TOTAL_VOTE_WEIGHT` bounds a summed vote weight and has never applied
    // to anything else. Without a mask every multiplier is 1 and `total` is
    // just the ballot count, so applying it there would newly refuse an
    // unweighted area with more ballots than the cap -- delegated voting has no
    // per-voter limit at all -- and would do it after those ballots were
    // irreversibly on the board.
    let is_weighted = found.iter().any(|(_, multiplier)| *multiplier != 1);
    if is_weighted && total > MAX_TOTAL_VOTE_WEIGHT {
        return Err(anyhow!(
            "Refusing to expand {total} weighted plaintexts for tally session contest {}: \
             the maximum summed vote weight is {MAX_TOTAL_VOTE_WEIGHT}",
            tally_session_contest.id,
        ));
    }

    let mut collected: Vec<<RistrettoCtx as Ctx>::P> = Vec::with_capacity(total as usize);
    for (plaintexts, multiplier) in found {
        for plaintext in plaintexts {
            collected.extend(std::iter::repeat_n(plaintext, multiplier as usize));
        }
    }
    Ok(Some(collected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::keycloak::MAX_VOTE_WEIGHT;
    use serde_json::json;

    fn contest_with(annotations: Option<serde_json::Value>) -> TallySessionContest {
        TallySessionContest {
            id: "id".to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            area_id: "area".to_string(),
            contest_id: None,
            session_id: 100,
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations,
            tally_session_id: "session".to_string(),
            election_id: "election".to_string(),
        }
    }

    fn annotations_with_mask(mask: Option<u32>) -> serde_json::Value {
        let mut value = json!({
            "elegible_voters": 10,
            "ballots_without_voter": 0,
            "casted_ballots": 10,
        });
        if let Some(mask) = mask {
            value["weight_bit_mask"] = json!(mask);
        }
        value
    }

    #[test]
    fn no_annotations_is_one_unweighted_batch() {
        assert_eq!(
            contest_weight_batches(&contest_with(None)).unwrap(),
            vec![(100, 1)]
        );
    }

    #[test]
    fn annotations_without_a_mask_is_one_unweighted_batch() {
        let contest = contest_with(Some(annotations_with_mask(None)));
        assert_eq!(contest_weight_batches(&contest).unwrap(), vec![(100, 1)]);
    }

    #[test]
    fn unreadable_annotations_are_an_error_not_an_unweighted_batch() {
        // Falling back here would count one batch at multiplier 1 and discard
        // every other batch, reporting a wrong result as complete.
        let contest = contest_with(Some(json!({"elegible_voters": "not a number"})));
        assert!(contest_weight_batches(&contest).is_err());
    }

    #[test]
    fn a_mask_expands_to_its_set_bits_with_powers_of_two() {
        // 0b1011 -> offsets 0, 1 and 3.
        let contest = contest_with(Some(annotations_with_mask(Some(0b1011))));
        assert_eq!(
            contest_weight_batches(&contest).unwrap(),
            vec![(100, 1), (101, 2), (103, 8)]
        );
    }

    #[test]
    fn multipliers_sum_to_the_weight_they_encode() {
        // Any weight is the sum of the multipliers of the batches it occupies;
        // this is the property the tally depends on for an exact count.
        for weight in [1u64, 2, 3, 7, 100, 4321, 65536, 100_000] {
            let contest = contest_with(Some(annotations_with_mask(Some(weight as u32))));
            let total: u64 = contest_weight_batches(&contest)
                .unwrap()
                .into_iter()
                .map(|(_, multiplier)| multiplier)
                .sum();
            assert_eq!(total, weight, "weight {weight}");
        }
    }

    #[test]
    fn a_mask_bit_outside_the_owned_batches_is_ignored_not_miscounted() {
        // Bits at or above VOTE_WEIGHT_BATCHES name batches the area does not
        // own, and are dropped by the range rather than reaching the shift.
        // `weight_bit_multiplier` refuses them too, so neither layer can wrap
        // such a bit round to a multiplier of 1.
        assert_eq!(weight_bit_multiplier(VOTE_WEIGHT_BATCHES), None);
        assert_eq!(weight_bit_multiplier(64), None);
        let contest = contest_with(Some(annotations_with_mask(Some(u32::MAX))));
        let batches = contest_weight_batches(&contest).unwrap();
        assert_eq!(batches.len(), VOTE_WEIGHT_BATCHES as usize);
        assert_eq!(batches.first(), Some(&(100, 1)));
        assert_eq!(
            batches.last(),
            Some(&(
                100 + VOTE_WEIGHT_BATCHES as i64 - 1,
                1u64 << (VOTE_WEIGHT_BATCHES - 1)
            ))
        );
    }

    #[test]
    fn a_weight_is_the_sum_of_the_multipliers_of_the_batches_it_occupies() {
        // The invariant the whole mechanism rests on: splitting a weight across
        // batches and multiplying each batch back must reproduce it exactly.
        for weight in 1..=2048u64 {
            let total: u64 = weight_batch_offsets(weight)
                .unwrap()
                .map(|bit| weight_bit_multiplier(bit).unwrap())
                .sum();
            assert_eq!(total, weight, "weight {weight}");
        }
        for weight in [4321u64, 65_535, 65_536, 99_999, MAX_VOTE_WEIGHT] {
            let total: u64 = weight_batch_offsets(weight)
                .unwrap()
                .map(|bit| weight_bit_multiplier(bit).unwrap())
                .sum();
            assert_eq!(total, weight, "weight {weight}");
        }
    }

    #[test]
    fn an_electorate_tallies_to_its_summed_weight() {
        // What the tally actually computes: each batch holds one ciphertext per
        // voter whose weight sets that bit, and contributes its multiplier for
        // each. That must equal the sum of the weights.
        let electorate: Vec<u64> = vec![1, 1, 2, 3, 7, 100, 4321, 65_536, MAX_VOTE_WEIGHT];
        let mut batch_sizes = vec![0u64; VOTE_WEIGHT_BATCHES as usize];
        for weight in &electorate {
            for offset in weight_batch_offsets(*weight).unwrap() {
                batch_sizes[offset as usize] += 1;
            }
        }
        let tallied: u64 = batch_sizes
            .iter()
            .enumerate()
            .map(|(bit, size)| size * weight_bit_multiplier(bit as u32).unwrap())
            .sum();
        assert_eq!(tallied, electorate.iter().sum::<u64>());
    }

    #[test]
    fn no_batch_holds_a_voter_twice() {
        // The property that removes the within-batch signal: a voter must
        // occupy any given batch at most once.
        for weight in 1..=4096u64 {
            let offsets: Vec<u32> = weight_batch_offsets(weight).unwrap().collect();
            let mut deduped = offsets.clone();
            deduped.sort_unstable();
            deduped.dedup();
            assert_eq!(offsets.len(), deduped.len(), "weight {weight}");
        }
    }

    #[test]
    fn a_weight_too_large_to_represent_is_refused_not_truncated() {
        assert!(weight_batch_offsets(1u64 << VOTE_WEIGHT_BATCHES).is_err());
        assert!(weight_batch_offsets(u64::MAX).is_err());
        // The largest weight the import permits must still be representable.
        assert!(weight_batch_offsets(MAX_VOTE_WEIGHT).is_ok());
    }

    #[test]
    fn the_batch_layout_is_the_one_already_written_to_the_database() {
        // session_ids are allocated VOTE_WEIGHT_BATCHES apart and persist, so
        // this value cannot be changed without renumbering existing rows.
        // Pinned as a literal precisely so that raising MAX_VOTE_WEIGHT, which
        // derives it, fails here rather than silently overlapping stored runs.
        assert_eq!(VOTE_WEIGHT_BATCHES, 17);
        assert_eq!(MAX_VOTE_WEIGHT, 100_000);
    }
}
