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
use sequent_core::types::keycloak::{weight_bit_multiplier, weight_has_bit, VOTE_WEIGHT_BATCHES};
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tracing::{event, Level};

/// Every batch number the area could own, posted or not. Used to decide whether
/// its ballots have already been dumped, which cannot read the mask because the
/// mask is written by the dump itself.
pub fn contest_batch_range(session_id: i32) -> impl Iterator<Item = i64> {
    let base = session_id as i64;
    (0..VOTE_WEIGHT_BATCHES as i64).map(move |offset| base + offset)
}

/// The batches the area actually posted, each with the multiplier the tally
/// owes it. Falls back to the single unweighted batch when no mask was
/// recorded, which covers every other policy and any row written before
/// weighting existed.
pub fn contest_weight_batches(tally_session_contest: &TallySessionContest) -> Vec<(i64, u64)> {
    let base = tally_session_contest.session_id as i64;
    let mask = tally_session_contest
        .annotations
        .clone()
        .and_then(|annotations| {
            deserialize_value::<TallySessionContestAnnotations>(annotations).ok()
        })
        .and_then(|annotations| annotations.weight_bit_mask);

    let Some(mask) = mask else {
        return vec![(base, 1)];
    };
    (0..VOTE_WEIGHT_BATCHES)
        .filter(|bit| mask & (1u32 << bit) != 0)
        .map(|bit| (base + bit as i64, weight_bit_multiplier(bit)))
        .collect()
}

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

/// The area's decrypted ballots, each repeated by the multiplier its batch
/// carries, in one vector for the contest to count.
///
/// `None` while any expected batch is still unmixed: counting the batches that
/// have arrived would silently drop the weight of the ones that have not, and
/// publish a result that looks complete.
pub fn collect_weighted_plaintexts(
    tally_session_contest: &TallySessionContest,
    relevant_plaintexts: &[&Message],
) -> Option<Vec<<RistrettoCtx as Ctx>::P>> {
    let mut collected: Vec<<RistrettoCtx as Ctx>::P> = Vec::new();
    for (batch, multiplier) in contest_weight_batches(tally_session_contest) {
        let plaintexts = relevant_plaintexts
            .iter()
            .find(|message| batch == message.statement.get_batch_number() as i64)
            .and_then(|message| message.artifact.clone())
            .and_then(|artifact| Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact).ok())
            .map(|plaintexts| plaintexts.0 .0);
        let Some(plaintexts) = plaintexts else {
            event!(
                Level::INFO,
                "Expected: Plaintexts not found yet for session contest = {}, batch number = {}",
                tally_session_contest.id,
                batch
            );
            return None;
        };
        for plaintext in plaintexts {
            collected.extend(std::iter::repeat_n(plaintext, multiplier as usize));
        }
    }
    Some(collected)
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
        assert_eq!(contest_weight_batches(&contest_with(None)), vec![(100, 1)]);
    }

    #[test]
    fn annotations_without_a_mask_is_one_unweighted_batch() {
        let contest = contest_with(Some(annotations_with_mask(None)));
        assert_eq!(contest_weight_batches(&contest), vec![(100, 1)]);
    }

    #[test]
    fn a_mask_expands_to_its_set_bits_with_powers_of_two() {
        // 0b1011 -> offsets 0, 1 and 3.
        let contest = contest_with(Some(annotations_with_mask(Some(0b1011))));
        assert_eq!(
            contest_weight_batches(&contest),
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
                .into_iter()
                .map(|(_, multiplier)| multiplier)
                .sum();
            assert_eq!(total, weight, "weight {weight}");
        }
    }

    #[test]
    fn the_range_covers_every_batch_the_mask_can_name() {
        let batches: Vec<i64> = contest_batch_range(100).collect();
        assert_eq!(batches.len(), VOTE_WEIGHT_BATCHES as usize);
        let contest = contest_with(Some(annotations_with_mask(Some(u32::MAX))));
        for (batch, _) in contest_weight_batches(&contest) {
            assert!(batches.contains(&batch), "batch {batch} outside the range");
        }
    }

    #[test]
    fn a_weight_is_the_sum_of_the_multipliers_of_the_batches_it_occupies() {
        // The invariant the whole mechanism rests on: splitting a weight across
        // batches and multiplying each batch back must reproduce it exactly.
        for weight in 1..=2048u64 {
            let total: u64 = weight_batch_offsets(weight)
                .unwrap()
                .map(weight_bit_multiplier)
                .sum();
            assert_eq!(total, weight, "weight {weight}");
        }
        for weight in [4321u64, 65_535, 65_536, 99_999, MAX_VOTE_WEIGHT] {
            let total: u64 = weight_batch_offsets(weight)
                .unwrap()
                .map(weight_bit_multiplier)
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
            .map(|(bit, size)| size * weight_bit_multiplier(bit as u32))
            .sum();
        assert_eq!(tallied, electorate.iter().sum::<u64>());
    }

    #[test]
    fn no_batch_holds_a_voter_twice() {
        // The privacy property: a repeated ciphertext inside a batch would be a
        // readable weight, so a voter must occupy any given batch at most once.
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
    fn the_batch_count_covers_the_largest_permitted_weight() {
        // If MAX_VOTE_WEIGHT grows past what VOTE_WEIGHT_BATCHES can hold, the
        // dump would start refusing valid weights; this pins the relationship.
        assert!(MAX_VOTE_WEIGHT < 1u64 << VOTE_WEIGHT_BATCHES);
        assert!(MAX_VOTE_WEIGHT >= 1u64 << (VOTE_WEIGHT_BATCHES - 1));
    }

    #[test]
    fn the_largest_permitted_weight_fits() {
        let contest = contest_with(Some(annotations_with_mask(Some(
            sequent_core::types::keycloak::MAX_VOTE_WEIGHT as u32,
        ))));
        let total: u64 = contest_weight_batches(&contest)
            .into_iter()
            .map(|(_, multiplier)| multiplier)
            .sum();
        assert_eq!(total, sequent_core::types::keycloak::MAX_VOTE_WEIGHT);
    }
}
