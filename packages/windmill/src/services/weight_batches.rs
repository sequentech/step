// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Batch layout for `VOTERS_WEIGHTED_VOTING`.
//!
//! A voter's weight is applied by placing their ciphertext in the batch for
//! each bit that weight sets, so a contest area owns a run of consecutive board
//! batches starting at its `session_id`, and velvet counts every ballot in the
//! batch at offset `bit` `2^bit` times. Summing those multipliers over the set
//! bits reconstructs the weight, which is what makes the count exact. A session
//! created now owns `VOTE_WEIGHT_BATCHES` batches per area; one created before
//! the layout widened owns `LEGACY_VOTE_WEIGHT_BATCHES`. Every other policy
//! fills only the first batch, and the helpers here collapse to the
//! single-batch behaviour that predates weighting.

use anyhow::{anyhow, Result};
use b4::messages::{artifact::Plaintexts, message::Message};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::{
    TallySessionConfiguration, TallySessionContest, TallySessionContestAnnotations,
};
use sequent_core::types::keycloak::{
    weight_bit_multiplier, weight_has_bit, LEGACY_VOTE_WEIGHT_BATCHES, VOTE_WEIGHT_BATCHES,
};
use strand::elgamal::Ciphertext;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tracing::{event, Level};

/// One batch of a contest area's decrypted ballots, and how many times velvet
/// counts each of them. Nothing is repeated to apply the multiplier, so memory
/// grows with the number of ballots rather than with their summed weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaintextBatch {
    pub multiplier: u64,
    pub plaintexts: Vec<<RistrettoCtx as Ctx>::P>,
}

/// How many batches each contest area of a voter-weighted session owns.
///
/// Recorded when the session is created. A session without it predates the
/// record and was allocated `LEGACY_VOTE_WEIGHT_BATCHES` apart. A count wider
/// than the current layout would reach into the next contest area's batches,
/// so it is refused rather than trusted.
pub fn session_weight_batches(configuration: &TallySessionConfiguration) -> Result<u32> {
    let batches = configuration
        .vote_weight_batches
        .unwrap_or(LEGACY_VOTE_WEIGHT_BATCHES);
    if !(1..=VOTE_WEIGHT_BATCHES).contains(&batches) {
        return Err(anyhow!(
            "The tally session records {batches} vote weight batches per contest area, \
             but a contest area owns between 1 and {VOTE_WEIGHT_BATCHES}"
        ));
    }
    Ok(batches)
}

/// The batch offsets one copy of this voter's ciphertext goes into, when each
/// contest area owns `batches` batches.
///
/// Errors rather than truncating on a weight too large to represent: masking it
/// would drop the part that does not fit and under-count that voter silently.
/// The voter import already refuses weights above `MAX_VOTE_WEIGHT`, so for a
/// current session this is a backstop; for a session created before the layout
/// widened it is the only thing that stops a large weight spilling into the
/// next contest area's batches.
pub fn weight_batch_offsets(weight: u64, batches: u32) -> Result<impl Iterator<Item = u32>> {
    if batches > VOTE_WEIGHT_BATCHES || weight >> batches != 0 {
        return Err(anyhow!(
            "Vote weight {weight} does not fit in the {batches} weight batches each contest \
             area of this tally session owns, which hold weights up to {}. A tally session \
             created before vote weights above that were allowed cannot count it; create a \
             new tally session",
            (1u64 << batches.min(VOTE_WEIGHT_BATCHES)) - 1
        ));
    }
    Ok((0..batches).filter(move |bit| weight_has_bit(weight, *bit)))
}

/// The batches the area actually posted, each with the multiplier the tally
/// owes it, for a session whose contest areas own `batches` batches.
///
/// Falls back to the single unweighted batch when no mask was recorded, which
/// covers every other policy and any row written before weighting existed.
/// Errors rather than falling back when the annotations are present but
/// unreadable: treating a corrupt weighted row as unweighted would count one
/// batch at multiplier 1, discard every other batch, and report the result as
/// complete.
pub fn contest_weight_batches(
    tally_session_contest: &TallySessionContest,
    batches: u32,
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
    // The dump only ever sets bits below the session's batch count, and forces
    // the mask to 1 for an area with no ballots at all. A mask of zero names no
    // batch, which is absorbing rather than loud: an empty batch list makes the
    // area complete with no votes at every layer that consumes it, publishes
    // results and closes the session. A bit at or above the batch count names a
    // batch number allocated to the next contest area, whose ballots would be
    // counted here at that bit's multiplier.
    let batches = batches.min(VOTE_WEIGHT_BATCHES);
    let owned = (1u64 << batches) - 1;
    if mask == 0 || mask & !owned != 0 {
        return Err(anyhow!(
            "Weight batch mask {mask:#b} for tally session contest {} does not name a set of \
             the {batches} batches this contest area owns",
            tally_session_contest.id
        ));
    }
    (0..batches)
        .filter(|bit| mask & (1u64 << bit) != 0)
        .map(|bit| {
            let multiplier = weight_bit_multiplier(bit).ok_or_else(|| {
                anyhow!("Weight batch offset {bit} is outside the batches a contest area owns")
            })?;
            Ok((base + bit as i64, multiplier))
        })
        .collect()
}

/// What a run should do with one of its area's batches, having compared what
/// the board already holds for it against what this run built.
#[derive(Debug, PartialEq, Eq)]
pub enum BatchReconciliation {
    /// Nothing readable on the board for it. Normally that means it has not
    /// been posted and should be; at the one call site that can see an
    /// unreadable artifact it means the batch is there but cannot be compared,
    /// and posting is not an option because the board is append-only.
    Post,
    /// The board already holds exactly this: leave it alone.
    Keep,
    /// The board holds something else. Under weighting that is a wrong tally,
    /// because the ballots on the board are what will be mixed and published
    /// whatever is counted; without it, it is the census drifting from the
    /// board, which has always been allowed.
    Diverged,
}

/// Decides one batch. Split out from the dump so that every case can be pinned
/// by a test: this decision changed in three consecutive review rounds, and
/// each change broke the previous one in a way no test could catch.
pub fn reconcile_batch(
    posted: Option<&[Ciphertext<RistrettoCtx>]>,
    built: &[Ciphertext<RistrettoCtx>],
) -> BatchReconciliation {
    let Some(posted) = posted else {
        return BatchReconciliation::Post;
    };
    // Note this is also what an unreadable artifact resolves to, and the caller
    // then leaves that batch alone. `collect_weighted_plaintexts` takes the
    // opposite line and errors, because there it is the difference between
    // waiting forever and failing. Here neither is recoverable -- the board is
    // append-only, so a batch that cannot be read cannot be replaced -- and a
    // batch that cannot be read also cannot be mixed, so nothing wrong can be
    // published. Refusing the whole dump over it would only lose the areas that
    // are still fine.
    // An area with no ballots posts one empty batch, so the tally has something
    // to wait for. It encodes no weight and contributes nothing, so it is not
    // evidence that anything changed -- but only while this run agrees there is
    // nothing to put in it.
    if posted.is_empty() && built.is_empty() {
        return BatchReconciliation::Keep;
    }
    if posted == built {
        return BatchReconciliation::Keep;
    }
    BatchReconciliation::Diverged
}

/// The area's decrypted ballots, batch by batch, each batch with the
/// multiplier velvet counts its ballots by, for a session whose contest areas
/// own `batches` batches.
///
/// `None` while any expected batch is still unmixed: counting the batches that
/// have arrived would silently drop the weight of the ones that have not, and
/// publish a result that looks complete.
pub fn collect_plaintext_batches(
    tally_session_contest: &TallySessionContest,
    relevant_plaintexts: &[&Message],
    batches: u32,
) -> Result<Option<Vec<PlaintextBatch>>> {
    gather_plaintext_batches(tally_session_contest, batches, |batch| {
        relevant_plaintexts
            .iter()
            .find(|message| batch == message.statement.get_batch_number() as i64)
            .and_then(|message| message.artifact.clone())
    })
}

/// `collect_plaintext_batches` over any source of `Plaintexts` artifacts,
/// keyed by batch number.
fn gather_plaintext_batches(
    tally_session_contest: &TallySessionContest,
    batches: u32,
    artifact_for_batch: impl Fn(i64) -> Option<Vec<u8>>,
) -> Result<Option<Vec<PlaintextBatch>>> {
    let expected = contest_weight_batches(tally_session_contest, batches)?;
    let mut found: Vec<PlaintextBatch> = Vec::with_capacity(expected.len());
    for (batch, multiplier) in expected {
        // An artifact that is present but will not deserialize is a broken
        // board message, not a batch that has yet to be mixed. Mapping it to
        // the latter waits for it forever; the whole point of separating the
        // two is that only one of them ever resolves.
        let plaintexts = artifact_for_batch(batch)
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
        found.push(PlaintextBatch {
            multiplier,
            plaintexts,
        });
    }
    Ok(Some(found))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::keycloak::MAX_VOTE_WEIGHT;
    use serde_json::json;
    use strand::serialization::{StrandSerialize, StrandVector};

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

    fn annotations_with_mask(mask: Option<u64>) -> serde_json::Value {
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

    /// A handful of distinct ciphertexts, built once per test so that the same
    /// index yields the same value and different indices do not.
    fn pool() -> Vec<Ciphertext<RistrettoCtx>> {
        let ctx = RistrettoCtx::default();
        let mut rng = ctx.get_rng();
        (0..4)
            .map(|_| Ciphertext {
                mhr: ctx.rnd(&mut rng),
                gr: ctx.rnd(&mut rng),
            })
            .collect()
    }

    fn plaintext(byte: u8) -> <RistrettoCtx as Ctx>::P {
        let mut plaintext = [0u8; 30];
        plaintext[0] = byte;
        plaintext
    }

    fn plaintexts_artifact(plaintexts: &[<RistrettoCtx as Ctx>::P]) -> Vec<u8> {
        Plaintexts::<RistrettoCtx>(StrandVector(plaintexts.to_vec()))
            .strand_serialize()
            .unwrap()
    }

    fn configuration_with(vote_weight_batches: Option<u32>) -> TallySessionConfiguration {
        TallySessionConfiguration {
            vote_weight_batches,
            ..Default::default()
        }
    }

    #[test]
    fn an_unposted_batch_is_posted() {
        let pool = pool();
        assert_eq!(reconcile_batch(None, &pool[..2]), BatchReconciliation::Post);
        assert_eq!(reconcile_batch(None, &[]), BatchReconciliation::Post);
    }

    #[test]
    fn a_batch_holding_what_this_run_built_is_kept() {
        let pool = pool();
        assert_eq!(
            reconcile_batch(Some(&pool[..3]), &pool[..3]),
            BatchReconciliation::Keep
        );
    }

    #[test]
    fn an_empty_placeholder_is_kept_only_while_it_is_still_empty() {
        // An area with no ballots posts one empty batch; that is not evidence
        // anything changed. But once this run has ballots for it, the empty
        // batch on the board would strand them.
        let pool = pool();
        assert_eq!(reconcile_batch(Some(&[]), &[]), BatchReconciliation::Keep);
        assert_eq!(
            reconcile_batch(Some(&[]), &pool[..1]),
            BatchReconciliation::Diverged
        );
    }

    #[test]
    fn a_batch_the_current_weights_no_longer_fill_has_diverged() {
        let pool = pool();
        assert_eq!(
            reconcile_batch(Some(&pool[..2]), &[]),
            BatchReconciliation::Diverged
        );
    }

    #[test]
    fn a_batch_of_a_different_size_has_diverged() {
        let pool = pool();
        assert_eq!(
            reconcile_batch(Some(&pool[..3]), &pool[..2]),
            BatchReconciliation::Diverged
        );
    }

    #[test]
    fn a_batch_of_the_same_size_holding_other_ballots_has_diverged() {
        // The case a ballot count cannot see: weights permuted between two
        // voters keeps every batch exactly the same size while changing who is
        // in it, and counting the board would seat the old weights.
        let pool = pool();
        let swapped: Vec<Ciphertext<RistrettoCtx>> = vec![pool[1].clone(), pool[0].clone()];
        assert_eq!(
            reconcile_batch(Some(&pool[..2]), &swapped),
            BatchReconciliation::Diverged
        );
        assert_eq!(
            reconcile_batch(Some(&pool[..2]), &pool[1..3]),
            BatchReconciliation::Diverged
        );
    }

    #[test]
    fn a_session_without_a_recorded_batch_count_owns_the_legacy_run() {
        // Voter-weighted sessions created before the layout widened were
        // allocated 17 batch numbers per contest area and never recorded it.
        assert_eq!(
            session_weight_batches(&configuration_with(None)).unwrap(),
            LEGACY_VOTE_WEIGHT_BATCHES
        );
        assert_eq!(
            session_weight_batches(&configuration_with(Some(VOTE_WEIGHT_BATCHES))).unwrap(),
            VOTE_WEIGHT_BATCHES
        );
    }

    #[test]
    fn a_recorded_batch_count_outside_the_layout_is_refused() {
        // Wider than the layout would read the next contest area's batches as
        // this one's; zero would leave the area nowhere to put a ballot.
        for batches in [0, VOTE_WEIGHT_BATCHES + 1, u32::MAX] {
            assert!(
                session_weight_batches(&configuration_with(Some(batches))).is_err(),
                "{batches} batches should be refused"
            );
        }
    }

    #[test]
    fn no_annotations_is_one_unweighted_batch() {
        assert_eq!(
            contest_weight_batches(&contest_with(None), VOTE_WEIGHT_BATCHES).unwrap(),
            vec![(100, 1)]
        );
    }

    #[test]
    fn annotations_without_a_mask_is_one_unweighted_batch() {
        let contest = contest_with(Some(annotations_with_mask(None)));
        assert_eq!(
            contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).unwrap(),
            vec![(100, 1)]
        );
    }

    #[test]
    fn unreadable_annotations_are_an_error_not_an_unweighted_batch() {
        // Falling back here would count one batch at multiplier 1 and discard
        // every other batch, reporting a wrong result as complete.
        let contest = contest_with(Some(json!({"elegible_voters": "not a number"})));
        assert!(contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).is_err());
    }

    #[test]
    fn a_mask_expands_to_its_set_bits_with_powers_of_two() {
        // 0b1011 -> offsets 0, 1 and 3.
        let contest = contest_with(Some(annotations_with_mask(Some(0b1011))));
        assert_eq!(
            contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).unwrap(),
            vec![(100, 1), (101, 2), (103, 8)]
        );
    }

    #[test]
    fn a_mask_written_for_a_legacy_session_reads_back_unchanged() {
        // Masks stored before they were widened to 64 bits are plain JSON
        // numbers below 2^17. They must name the same batches as before, both
        // for the legacy session that wrote them and under the wider layout.
        let mask: u64 = 0b1_0000_0000_0110_0101; // 65536 + 100 + 1
        let contest = contest_with(Some(annotations_with_mask(Some(mask))));
        let expected = vec![(100, 1), (102, 4), (105, 32), (106, 64), (116, 65_536)];
        assert_eq!(
            contest_weight_batches(&contest, LEGACY_VOTE_WEIGHT_BATCHES).unwrap(),
            expected
        );
        assert_eq!(
            contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).unwrap(),
            expected
        );
    }

    #[test]
    fn multipliers_sum_to_the_weight_they_encode() {
        // Any weight is the sum of the multipliers of the batches it occupies;
        // this is the property the tally depends on for an exact count.
        for weight in [
            1u64,
            2,
            3,
            7,
            100,
            4321,
            65_536,
            100_000,
            150_000,
            1_000_000,
            12_000_000,
            MAX_VOTE_WEIGHT,
        ] {
            let contest = contest_with(Some(annotations_with_mask(Some(weight))));
            let total: u64 = contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES)
                .unwrap()
                .into_iter()
                .map(|(_, multiplier)| multiplier)
                .sum();
            assert_eq!(total, weight, "weight {weight}");
        }
    }

    #[test]
    fn a_mask_bit_outside_the_owned_batches_is_an_error_not_a_neighbours_batch() {
        // The batch number past the end of a contest area's run belongs to the
        // next contest area. Counting it here would publish that area's ballots
        // at this bit's multiplier, so a bit the session cannot own is refused,
        // and `weight_bit_multiplier` refuses one beyond the layout too.
        assert_eq!(weight_bit_multiplier(VOTE_WEIGHT_BATCHES), None);
        assert_eq!(weight_bit_multiplier(64), None);
        let legacy_overflow = contest_with(Some(annotations_with_mask(Some(
            1u64 << LEGACY_VOTE_WEIGHT_BATCHES,
        ))));
        assert!(contest_weight_batches(&legacy_overflow, LEGACY_VOTE_WEIGHT_BATCHES).is_err());
        assert_eq!(
            contest_weight_batches(&legacy_overflow, VOTE_WEIGHT_BATCHES).unwrap(),
            vec![(
                100 + LEGACY_VOTE_WEIGHT_BATCHES as i64,
                1 << LEGACY_VOTE_WEIGHT_BATCHES
            )]
        );
        for mask in [1u64 << VOTE_WEIGHT_BATCHES, u64::MAX] {
            let contest = contest_with(Some(annotations_with_mask(Some(mask))));
            assert!(
                contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).is_err(),
                "mask {mask:#b} should be refused"
            );
        }
    }

    #[test]
    fn the_full_mask_names_every_owned_batch() {
        let contest = contest_with(Some(annotations_with_mask(Some(MAX_VOTE_WEIGHT))));
        let batches = contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).unwrap();
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
    fn a_mask_naming_no_owned_batch_is_an_error_not_an_empty_tally() {
        // An empty batch list is absorbing: it makes the area complete with no
        // votes everywhere it is consumed, so it must never be produced.
        let contest = contest_with(Some(annotations_with_mask(Some(0))));
        assert!(contest_weight_batches(&contest, VOTE_WEIGHT_BATCHES).is_err());
    }

    #[test]
    fn a_weight_is_the_sum_of_the_multipliers_of_the_batches_it_occupies() {
        // The invariant the whole mechanism rests on: splitting a weight across
        // batches and multiplying each batch back must reproduce it exactly.
        let sum_of_multipliers = |weight: u64| -> u64 {
            weight_batch_offsets(weight, VOTE_WEIGHT_BATCHES)
                .unwrap()
                .map(|bit| weight_bit_multiplier(bit).unwrap())
                .sum()
        };
        for weight in 1..=2048u64 {
            assert_eq!(sum_of_multipliers(weight), weight, "weight {weight}");
        }
        for weight in [
            4321u64,
            65_535,
            65_536,
            99_999,
            100_000,
            131_071,
            131_072,
            150_000,
            1_000_000,
            12_000_000,
            MAX_VOTE_WEIGHT - 1,
            MAX_VOTE_WEIGHT,
        ] {
            assert_eq!(sum_of_multipliers(weight), weight, "weight {weight}");
        }
    }

    #[test]
    fn an_electorate_tallies_to_its_summed_weight() {
        // What the tally actually computes: each batch holds one ciphertext per
        // voter whose weight sets that bit, and contributes its multiplier for
        // each. That must equal the sum of the weights, far beyond the summed
        // weight that the expansion used to be capped at.
        let electorate: Vec<u64> = vec![
            1,
            1,
            2,
            5,
            100,
            65_536,
            150_000,
            1_000_000,
            10_784_355,
            MAX_VOTE_WEIGHT,
        ];
        let mut batch_sizes = vec![0u64; VOTE_WEIGHT_BATCHES as usize];
        for weight in &electorate {
            for offset in weight_batch_offsets(*weight, VOTE_WEIGHT_BATCHES).unwrap() {
                batch_sizes[offset as usize] += 1;
            }
        }
        assert!(batch_sizes
            .iter()
            .all(|size| *size <= electorate.len() as u64));
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
        for weight in (1..=4096u64).chain([1_000_000, MAX_VOTE_WEIGHT]) {
            let offsets: Vec<u32> = weight_batch_offsets(weight, VOTE_WEIGHT_BATCHES)
                .unwrap()
                .collect();
            let mut deduped = offsets.clone();
            deduped.sort_unstable();
            deduped.dedup();
            assert_eq!(offsets.len(), deduped.len(), "weight {weight}");
        }
    }

    #[test]
    fn a_weight_too_large_to_represent_is_refused_not_truncated() {
        assert!(weight_batch_offsets(1u64 << VOTE_WEIGHT_BATCHES, VOTE_WEIGHT_BATCHES).is_err());
        assert!(weight_batch_offsets(u64::MAX, VOTE_WEIGHT_BATCHES).is_err());
        // The largest weight the import permits must still be representable.
        assert!(weight_batch_offsets(MAX_VOTE_WEIGHT, VOTE_WEIGHT_BATCHES).is_ok());
        assert!(weight_batch_offsets(1, VOTE_WEIGHT_BATCHES + 1).is_err());
    }

    #[test]
    fn a_legacy_session_refuses_a_weight_its_run_cannot_hold() {
        // A session created before the layout widened owns 17 batches, and the
        // 18th batch number is the next contest area's first. A weight the
        // import now accepts must be refused there, not spilled into it.
        let legacy = LEGACY_VOTE_WEIGHT_BATCHES;
        let largest = (1u64 << legacy) - 1;
        assert_eq!(
            weight_batch_offsets(largest, legacy).unwrap().max(),
            Some(legacy - 1)
        );
        for weight in [largest + 1, 150_000, 1_000_000] {
            let error = weight_batch_offsets(weight, legacy)
                .err()
                .unwrap_or_else(|| panic!("weight {weight} should be refused"));
            assert!(
                error.to_string().contains("create a new tally session"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn plaintext_batches_are_returned_once_each_with_their_multiplier() {
        // Weights 1, 3 and 4 fill the batches for 1 (two voters), 2 (one) and
        // 4 (one). Each plaintext is returned once; nothing is repeated.
        let contest = contest_with(Some(annotations_with_mask(Some(0b111))));
        let artifacts = [
            (100, plaintexts_artifact(&[plaintext(1), plaintext(3)])),
            (101, plaintexts_artifact(&[plaintext(3)])),
            (102, plaintexts_artifact(&[plaintext(4)])),
        ];
        let batches = gather_plaintext_batches(&contest, VOTE_WEIGHT_BATCHES, |batch| {
            artifacts
                .iter()
                .find(|(number, _)| *number == batch)
                .map(|(_, artifact)| artifact.clone())
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            batches,
            vec![
                PlaintextBatch {
                    multiplier: 1,
                    plaintexts: vec![plaintext(1), plaintext(3)],
                },
                PlaintextBatch {
                    multiplier: 2,
                    plaintexts: vec![plaintext(3)],
                },
                PlaintextBatch {
                    multiplier: 4,
                    plaintexts: vec![plaintext(4)],
                },
            ]
        );
    }

    #[test]
    fn a_huge_summed_weight_is_collected_without_expanding_it() {
        // Two voters carrying 12 000 000 between them and one at the largest
        // weight, all far above the 1 000 000 the expansion was capped at in
        // total. What comes back is one plaintext per batch each weight sets,
        // never the weight's worth of copies.
        let weights = [11_000_000, 1_000_000, MAX_VOTE_WEIGHT];
        let mask = weights.iter().fold(0u64, |mask, weight| mask | weight);
        let contest = contest_with(Some(annotations_with_mask(Some(mask))));
        let batches = gather_plaintext_batches(&contest, VOTE_WEIGHT_BATCHES, |batch| {
            let bit = (batch - 100) as u32;
            let members: Vec<_> = weights
                .iter()
                .enumerate()
                .filter(|(_, weight)| weight_has_bit(**weight, bit))
                .map(|(voter, _)| plaintext(voter as u8))
                .collect();
            Some(plaintexts_artifact(&members))
        })
        .unwrap()
        .unwrap();
        let stored: usize = batches.iter().map(|batch| batch.plaintexts.len()).sum();
        let popcount: u32 = weights.iter().map(|weight| weight.count_ones()).sum();
        assert_eq!(stored, popcount as usize);
        let counted: u64 = batches
            .iter()
            .map(|batch| batch.plaintexts.len() as u64 * batch.multiplier)
            .sum();
        assert_eq!(counted, weights.iter().sum::<u64>());
    }

    #[test]
    fn a_missing_batch_waits_and_an_unreadable_one_fails() {
        let contest = contest_with(Some(annotations_with_mask(Some(0b11))));
        let only_first = gather_plaintext_batches(&contest, VOTE_WEIGHT_BATCHES, |batch| {
            (batch == 100).then(|| plaintexts_artifact(&[plaintext(1)]))
        })
        .unwrap();
        assert_eq!(only_first, None);

        let unreadable = gather_plaintext_batches(&contest, VOTE_WEIGHT_BATCHES, |batch| {
            Some(if batch == 100 {
                plaintexts_artifact(&[plaintext(1)])
            } else {
                vec![0xff]
            })
        });
        assert!(unreadable.is_err());
    }

    #[test]
    fn the_batch_layout_is_the_one_already_written_to_the_database() {
        // session_ids persist, so the stride a session was allocated with
        // cannot change under it. Voter-weighted sessions record their batch
        // count; ones that predate the record were allocated 17 apart. Pinned
        // as literals so that changing either constant fails here first.
        assert_eq!(LEGACY_VOTE_WEIGHT_BATCHES, 17);
        assert_eq!(VOTE_WEIGHT_BATCHES, 32);
        assert_eq!(MAX_VOTE_WEIGHT, u32::MAX as u64);
        assert!(VOTE_WEIGHT_BATCHES >= LEGACY_VOTE_WEIGHT_BATCHES);
    }
}
