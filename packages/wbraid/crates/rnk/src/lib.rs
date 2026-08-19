// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use serde::{Deserialize, Serialize};

use derive_more::{From, TryInto};
use enum_dispatch::enum_dispatch;

pub mod basic;
pub mod composite;

pub use basic::*;
pub use composite::*;

pub type Rank = u64;

#[enum_dispatch(Ranked)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RankedSet {
    PermutationSet(PermutationSet),
    CombinationSet(CombinationSet),
    MCombinationSet(MCombinationSet),
    SubsetSet(SubsetSet),
    OrderedCombinationSet(OrderedCombinationSet),
    ProductSet(ProductSet),
    UnionSet(UnionSet),
}

/// Enum representing a value from any ranked set type.
///
/// Each variant corresponds to a value type from one of
/// the set types in `RankedSet`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TryInto, From)]
#[try_into(owned, ref)]
pub enum RankedValue {
    Permutation(Permutation),
    Combination(Combination),
    MCombination(MCombination),
    Subset(Subset),
    OrderedCombination(OrderedCombination),
    Product(Product),
    UnionElement(UnionElement),
}
impl RankedValue {
    // We use conversions from an enum to a variant struct provided by the derive_more crate

    pub fn as_permutation(&self) -> Result<&Permutation> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected Permutation"))
    }

    pub fn as_combination(&self) -> Result<&Combination> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected Combination"))
    }

    pub fn as_mcombination(&self) -> Result<&MCombination> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected MCombination"))
    }

    pub fn as_subset(&self) -> Result<&Subset> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected Subset"))
    }

    pub fn as_ordered_combination(&self) -> Result<&OrderedCombination> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected OrderedCombination"))
    }

    pub fn as_product(&self) -> Result<&Product> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected Product"))
    }

    pub fn as_union_element(&self) -> Result<&UnionElement> {
        self.try_into()
            .map_err(|_| anyhow::anyhow!("Expected UnionElement"))
    }
}

#[enum_dispatch]
pub trait Ranked {
    fn cardinality(&self) -> Rank;
    fn rank(&self, value: &RankedValue) -> Result<Rank>;
    fn unrank(&self, rank: Rank) -> RankedValue;
    fn is_member(&self, value: &RankedValue) -> bool;
    fn name(&self) -> &str;
}

pub fn decode_multi(mut n: usize, mut k: usize) -> Vec<usize> {
    let mut r = vec![0; k];
    debug_assert_eq!(r.len(), k, "Failed precondition");
    debug_assert!(k > 0 || n == 0, "Failed precondition");
    while k > 0 {
        let mut i = 1;
        let mut x = 1;
        while x <= n {
            i += 1;
            x = number_encoding::combination(i + k - 1, k);
        }
        x = number_encoding::combination(i - 1 + k - 1, k);
        i -= 1;
        n -= x;
        k -= 1;
        r[k] = i;
    }

    r
}

pub fn encode_multi(xs: &[usize]) -> usize {
    let mut r = 0;
    for (i, &x) in xs.iter().enumerate() {
        // x + i + 1 - 1 = x + i
        r += number_encoding::combination(x + i, i + 1);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Combination Tests ==========

    #[test]
    fn test_combination_round_trip() {
        let set = CombinationSet::new(
            "colors".to_string(),
            &[
                "red".to_string(),
                "green".to_string(),
                "blue".to_string(),
                "yellow".to_string(),
            ],
            2,
        );

        for rank in 0..set.cardinality() {
            let value = set.unrank(rank);
            let reranked = set.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    #[test]
    fn test_combination_cardinality() {
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );
        assert_eq!(set.cardinality(), 3); // C(3,2) = 3
    }

    #[test]
    #[should_panic(expected = "Invalid rank")]
    fn test_combination_invalid_rank() {
        let set = CombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 1);
        set.unrank(10); // Out of bounds
    }

    // ========== Permutation Tests ==========

    #[test]
    fn test_permutation_round_trip() {
        let set = PermutationSet::new(
            "items".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );

        for rank in 0..set.cardinality() {
            let value = set.unrank(rank);
            let reranked = set.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    #[test]
    fn test_permutation_cardinality() {
        let set = PermutationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );
        assert_eq!(set.cardinality(), 6); // 3! = 6
    }

    // ========== Subset Tests ==========

    #[test]
    fn test_subset_round_trip() {
        let set = SubsetSet::new(
            "items".to_string(),
            &["x".to_string(), "y".to_string(), "z".to_string()],
        );

        for rank in 0..set.cardinality() {
            let value = set.unrank(rank);
            let reranked = set.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    #[test]
    fn test_subset_cardinality() {
        let set = SubsetSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );
        assert_eq!(set.cardinality(), 8); // 2^3 = 8
    }

    // ========== MCombination Tests ==========

    #[test]
    fn test_mcombination_round_trip() {
        let set = MCombinationSet::new(
            "fruits".to_string(),
            &["apple".to_string(), "banana".to_string()],
            3,
        );

        for rank in 0..set.cardinality() {
            let value = set.unrank(rank);
            let reranked = set.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    // ========== OrderedCombination Tests ==========

    #[test]
    fn test_ordered_combination_round_trip() {
        let set = OrderedCombinationSet::new(
            "choices".to_string(),
            &[
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
            2,
        );

        for rank in 0..set.cardinality() {
            let value = set.unrank(rank);
            let reranked = set.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    // ========== Product Tests ==========

    #[test]
    fn test_product_round_trip() {
        let colors = CombinationSet::new(
            "color".to_string(),
            &["red".to_string(), "blue".to_string()],
            1,
        );
        let sizes = CombinationSet::new(
            "size".to_string(),
            &["small".to_string(), "large".to_string()],
            1,
        );

        let product = ProductSet::new("items", vec![colors.into(), sizes.into()]);

        for rank in 0..product.cardinality() {
            let value = product.unrank(rank);
            let reranked = product.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    #[test]
    fn test_product_cardinality() {
        let set1 = CombinationSet::new("a".to_string(), &["x".to_string(), "y".to_string()], 1);
        let set2 = CombinationSet::new(
            "b".to_string(),
            &["p".to_string(), "q".to_string(), "r".to_string()],
            1,
        );

        let product = ProductSet::new("test", vec![set1.into(), set2.into()]);
        assert_eq!(product.cardinality(), 6); // 2 * 3 = 6
    }

    // ========== Union Tests ==========

    #[test]
    fn test_union_round_trip() {
        let colors = CombinationSet::new(
            "colors".to_string(),
            &["red".to_string(), "blue".to_string()],
            1,
        );
        let shapes = CombinationSet::new(
            "shapes".to_string(),
            &["circle".to_string(), "square".to_string()],
            1,
        );

        let union = UnionSet::new("items", vec![colors.into(), shapes.into()]);

        for rank in 0..union.cardinality() {
            let value = union.unrank(rank);
            let reranked = union.rank(&value).unwrap();
            assert_eq!(rank, reranked);
        }
    }

    #[test]
    fn test_union_cardinality() {
        let set1 = CombinationSet::new("a".to_string(), &["x".to_string(), "y".to_string()], 1);
        let set2 = CombinationSet::new(
            "b".to_string(),
            &["p".to_string(), "q".to_string(), "r".to_string()],
            1,
        );

        let union = UnionSet::new("test", vec![set1.into(), set2.into()]);
        assert_eq!(union.cardinality(), 5); // 2 + 3 = 5
    }

    // ========== Type Safety Tests ==========

    #[test]
    fn test_wrong_variant_rejected() {
        let combo_set =
            CombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 1);
        let perm_set =
            PermutationSet::new("other".to_string(), &["x".to_string(), "y".to_string()]);

        let perm_value = perm_set.unrank(0);

        // Trying to rank a permutation in a combination set should fail
        assert!(!combo_set.is_member(&perm_value));
    }

    // ========== Edge Case Tests ==========

    #[test]
    fn test_combination_empty_selection() {
        // C(n, 0) = 1 - there's exactly one way to choose nothing
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            0,
        );
        assert_eq!(set.cardinality(), 1);

        // Verify round-trip works
        let value = set.unrank(0);
        assert_eq!(set.rank(&value).unwrap(), 0);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_combination_select_all() {
        // C(3, 3) = 1 - there's exactly one way to choose everything
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            3,
        );
        assert_eq!(set.cardinality(), 1);

        let value = set.unrank(0);
        assert_eq!(set.rank(&value).unwrap(), 0);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_combination_single_element() {
        // C(1, 1) = 1
        let set = CombinationSet::new("test".to_string(), &["x".to_string()], 1);
        assert_eq!(set.cardinality(), 1);

        let value = set.unrank(0);
        assert_eq!(set.rank(&value).unwrap(), 0);
    }

    #[test]
    fn test_combination_boundary_ranks() {
        let set = CombinationSet::new(
            "test".to_string(),
            &[
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
            2,
        );

        // Test first rank
        let first = set.unrank(0);
        assert_eq!(set.rank(&first).unwrap(), 0);
        assert!(set.is_member(&first));

        // Test last rank
        let last = set.unrank(set.cardinality() - 1);
        assert_eq!(set.rank(&last).unwrap(), set.cardinality() - 1);
        assert!(set.is_member(&last));
    }

    #[test]
    fn test_permutation_single_element() {
        // 1! = 1
        let set = PermutationSet::new("test".to_string(), &["x".to_string()]);
        assert_eq!(set.cardinality(), 1);

        let value = set.unrank(0);
        assert_eq!(set.rank(&value).unwrap(), 0);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_permutation_boundary() {
        let set = PermutationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );

        // Test first and last permutations
        let first = set.unrank(0);
        assert_eq!(set.rank(&first).unwrap(), 0);

        let last = set.unrank(5); // 3! - 1 = 5
        assert_eq!(set.rank(&last).unwrap(), 5);
    }

    #[test]
    fn test_subset_empty_universe() {
        // 2^0 = 1 - only the empty subset exists
        let set = SubsetSet::new("test".to_string(), &[]);
        assert_eq!(set.cardinality(), 1);

        let value = set.unrank(0);
        assert_eq!(set.rank(&value).unwrap(), 0);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_subset_boundary() {
        let set = SubsetSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );

        // rank 0 should be empty subset
        let empty = set.unrank(0);
        assert_eq!(set.rank(&empty).unwrap(), 0);

        // rank 7 should be full subset {a,b,c}
        let full = set.unrank(7);
        assert_eq!(set.rank(&full).unwrap(), 7);
    }

    #[test]
    fn test_mcombination_zero_choices() {
        // Choosing 0 items with replacement from any set
        let set = MCombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 0);
        assert_eq!(set.cardinality(), 1);

        let value = set.unrank(0);
        assert_eq!(set.rank(&value).unwrap(), 0);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_mcombination_single_item() {
        // Choosing 1 item with replacement: should equal n
        let set = MCombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            1,
        );
        assert_eq!(set.cardinality(), 3);

        // Verify all ranks work
        for rank in 0..3 {
            let value = set.unrank(rank);
            assert_eq!(set.rank(&value).unwrap(), rank);
        }
    }

    #[test]
    fn test_ordered_combination_boundary() {
        let set =
            OrderedCombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 2);
        // P(2,2) = 2! = 2 (ab, ba)
        assert_eq!(set.cardinality(), 2);

        let value0 = set.unrank(0);
        assert_eq!(set.rank(&value0).unwrap(), 0);

        let value1 = set.unrank(1);
        assert_eq!(set.rank(&value1).unwrap(), 1);
    }

    #[test]
    fn test_product_empty_factors() {
        // Product with no factors should have cardinality 1 (empty product)
        let product = ProductSet::new("test", vec![]);
        assert_eq!(product.cardinality(), 1);

        let value = product.unrank(0);
        assert_eq!(product.rank(&value).unwrap(), 0);
    }

    #[test]
    fn test_product_single_factor() {
        let set = CombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 1);
        let product = ProductSet::new("single", vec![set.into()]);

        assert_eq!(product.cardinality(), 2);

        for rank in 0..2 {
            let value = product.unrank(rank);
            assert_eq!(product.rank(&value).unwrap(), rank);
        }
    }

    #[test]
    fn test_union_single_set() {
        let set = CombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 1);
        let union = UnionSet::new("single", vec![set.into()]);

        assert_eq!(union.cardinality(), 2);

        for rank in 0..2 {
            let value = union.unrank(rank);
            assert_eq!(union.rank(&value).unwrap(), rank);
        }
    }

    // ========== is_member_value Positive Tests ==========

    #[test]
    fn test_is_member_combination_positive() {
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );

        // Generate a valid value and verify it's a member
        let value = set.unrank(1);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_is_member_permutation_positive() {
        let set = PermutationSet::new("test".to_string(), &["x".to_string(), "y".to_string()]);

        let value = set.unrank(0);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_is_member_subset_positive() {
        let set = SubsetSet::new("test".to_string(), &["a".to_string(), "b".to_string()]);

        let value = set.unrank(2);
        assert!(set.is_member(&value));
    }

    #[test]
    fn test_is_member_product_positive() {
        let set1 = CombinationSet::new("a".to_string(), &["x".to_string(), "y".to_string()], 1);
        let set2 = CombinationSet::new("b".to_string(), &["p".to_string(), "q".to_string()], 1);
        let product = ProductSet::new("test", vec![set1.into(), set2.into()]);

        let value = product.unrank(0);
        assert!(product.is_member(&value));
    }

    #[test]
    fn test_is_member_union_positive() {
        let set1 = CombinationSet::new("a".to_string(), &["x".to_string(), "y".to_string()], 1);
        let set2 = CombinationSet::new("b".to_string(), &["p".to_string()], 1);
        let union = UnionSet::new("test", vec![set1.into(), set2.into()]);

        let value = union.unrank(1);
        assert!(union.is_member(&value));
    }

    // ========== Error Handling Tests ==========

    // --- rank_value with wrong variant ---

    #[test]
    fn test_rank_value_wrong_variant() {
        let combo_set =
            CombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 1);
        let perm_set =
            PermutationSet::new("other".to_string(), &["x".to_string(), "y".to_string()]);

        let perm_value = perm_set.unrank(0);

        // Trying to rank a permutation in a combination set should error
        let result = combo_set.rank(&perm_value);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Expected Combination")
        );
    }

    #[test]
    fn test_rank_value_all_variant_mismatches() {
        let combo = CombinationSet::new("c".to_string(), &["a".to_string()], 1);
        let perm = PermutationSet::new("p".to_string(), &["a".to_string()]);
        let subset = SubsetSet::new("s".to_string(), &["a".to_string()]);
        let mc = MCombinationSet::new("m".to_string(), &["a".to_string()], 1);
        let oc =
            OrderedCombinationSet::new("o".to_string(), &["a".to_string(), "b".to_string()], 1);

        // Get one valid value from each
        let combo_val = combo.unrank(0);
        let perm_val = perm.unrank(0);
        let subset_val = subset.unrank(1);
        let mc_val = mc.unrank(0);
        let oc_val = oc.unrank(0);

        // Each set should reject all other types
        assert!(combo.rank(&perm_val).is_err());
        assert!(combo.rank(&subset_val).is_err());
        assert!(perm.rank(&combo_val).is_err());
        assert!(perm.rank(&mc_val).is_err());
        assert!(subset.rank(&oc_val).is_err());
    }

    // --- rank_value with invalid structure ---

    #[test]
    fn test_rank_value_wrong_number_of_elements() {
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );

        // Try to create a combination with wrong number of elements
        let bad_combo = Combination::from_set(&set, vec!["a".to_string()]);
        assert!(bad_combo.is_err());
        assert!(
            bad_combo
                .unwrap_err()
                .to_string()
                .contains("Wrong number of elements")
        );
    }

    #[test]
    fn test_rank_value_element_not_in_ground_set() {
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );

        // Try to create a combination with element not in ground set
        let bad_combo = Combination::from_set(&set, vec!["a".to_string(), "x".to_string()]);
        assert!(bad_combo.is_err());
        assert!(
            bad_combo
                .unwrap_err()
                .to_string()
                .contains("Element not in ground set")
        );
    }

    #[test]
    fn test_rank_value_duplicate_elements() {
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );

        // Try to create a combination with duplicate elements
        let bad_combo = Combination::from_set(&set, vec!["a".to_string(), "a".to_string()]);
        assert!(bad_combo.is_err());
        assert!(
            bad_combo
                .unwrap_err()
                .to_string()
                .contains("Duplicate element")
        );
    }

    #[test]
    fn test_rank_value_permutation_wrong_size() {
        let set = PermutationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );

        // Permutation must have all elements
        let bad_perm = Permutation::from_set(&set, vec!["a".to_string(), "b".to_string()]);
        assert!(bad_perm.is_err());
    }

    #[test]
    fn test_rank_value_permutation_duplicate() {
        let set = PermutationSet::new("test".to_string(), &["a".to_string(), "b".to_string()]);

        let bad_perm = Permutation::from_set(&set, vec!["a".to_string(), "a".to_string()]);
        assert!(bad_perm.is_err());
        assert!(
            bad_perm
                .unwrap_err()
                .to_string()
                .contains("duplicate element")
        );
    }

    #[test]
    fn test_rank_value_subset_element_not_in_universe() {
        let set = SubsetSet::new("test".to_string(), &["a".to_string(), "b".to_string()]);

        let bad_subset = Subset::from_set(&set, vec!["a".to_string(), "x".to_string()]);
        assert!(bad_subset.is_err());
        assert!(
            bad_subset
                .unwrap_err()
                .to_string()
                .contains("Element not in ground set")
        );
    }

    #[test]
    fn test_rank_value_mcombination_wrong_count() {
        let set = MCombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 3);

        // Must have exactly k elements (with possible repeats)
        let bad_mc = MCombination::from_set(&set, vec!["a".to_string(), "a".to_string()]);
        assert!(bad_mc.is_err());
    }

    #[test]
    fn test_rank_value_ordered_combination_wrong_size() {
        let set = OrderedCombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );

        let bad_oc = OrderedCombination::from_set(&set, vec!["a".to_string()]);
        assert!(bad_oc.is_err());
    }

    // --- is_member_value with wrong variant ---

    #[test]
    fn test_is_member_value_wrong_variant() {
        let combo_set =
            CombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 1);
        let perm_set =
            PermutationSet::new("other".to_string(), &["x".to_string(), "y".to_string()]);

        let perm_value = perm_set.unrank(0);

        // Wrong variant should return false
        assert!(!combo_set.is_member(&perm_value));
    }

    #[test]
    fn test_is_member_value_invalid_structure() {
        let set1 = CombinationSet::new("set1".to_string(), &["a".to_string(), "b".to_string()], 1);
        let set2 = CombinationSet::new("set2".to_string(), &["x".to_string(), "y".to_string()], 1);

        // Get a valid value from set1
        let value_from_set1 = set1.unrank(0);

        // It should not be a member of set2 (different ground set)
        assert!(!set2.is_member(&value_from_set1));
    }

    // --- from_set constructor errors ---

    #[test]
    fn test_from_set_validates_input() {
        let set = CombinationSet::new(
            "test".to_string(),
            &["a".to_string(), "b".to_string(), "c".to_string()],
            2,
        );

        // Valid construction should work
        let valid = Combination::from_set(&set, vec!["a".to_string(), "b".to_string()]);
        assert!(valid.is_ok());

        // Invalid constructions should fail
        assert!(Combination::from_set(&set, vec!["a".to_string()]).is_err());
        assert!(Combination::from_set(&set, vec!["x".to_string(), "y".to_string()]).is_err());
        assert!(Combination::from_set(&set, vec!["a".to_string(), "a".to_string()]).is_err());
    }

    #[test]
    fn test_permutation_from_set_validates() {
        let set = PermutationSet::new("test".to_string(), &["a".to_string(), "b".to_string()]);

        // Valid
        assert!(Permutation::from_set(&set, vec!["a".to_string(), "b".to_string()]).is_ok());
        assert!(Permutation::from_set(&set, vec!["b".to_string(), "a".to_string()]).is_ok());

        // Invalid
        assert!(Permutation::from_set(&set, vec!["a".to_string()]).is_err());
        assert!(Permutation::from_set(&set, vec!["a".to_string(), "a".to_string()]).is_err());
        assert!(Permutation::from_set(&set, vec!["x".to_string(), "y".to_string()]).is_err());
    }

    #[test]
    fn test_subset_from_set_validates() {
        let set = SubsetSet::new("test".to_string(), &["a".to_string(), "b".to_string()]);

        // Valid (any subset of ground set is ok, including duplicates should be rejected)
        assert!(Subset::from_set(&set, vec![]).is_ok());
        assert!(Subset::from_set(&set, vec!["a".to_string()]).is_ok());
        assert!(Subset::from_set(&set, vec!["a".to_string(), "b".to_string()]).is_ok());

        // Invalid
        assert!(Subset::from_set(&set, vec!["x".to_string()]).is_err());
    }

    #[test]
    fn test_mcombination_from_set_validates() {
        let set = MCombinationSet::new("test".to_string(), &["a".to_string(), "b".to_string()], 3);

        // Valid (can have repeats)
        assert!(
            MCombination::from_set(
                &set,
                vec!["a".to_string(), "a".to_string(), "a".to_string()]
            )
            .is_ok()
        );
        assert!(
            MCombination::from_set(
                &set,
                vec!["a".to_string(), "a".to_string(), "b".to_string()]
            )
            .is_ok()
        );

        // Invalid (wrong count or bad elements)
        assert!(MCombination::from_set(&set, vec!["a".to_string(), "a".to_string()]).is_err());
        assert!(
            MCombination::from_set(
                &set,
                vec!["x".to_string(), "x".to_string(), "x".to_string()]
            )
            .is_err()
        );
    }

    // --- Product and Union error cases ---

    #[test]
    fn test_product_rank_value_wrong_variant() {
        let set1 = CombinationSet::new("a".to_string(), &["x".to_string()], 1);
        let set2 = CombinationSet::new("b".to_string(), &["y".to_string()], 1);
        let product = ProductSet::new("test", vec![set1.into(), set2.into()]);

        let combo = CombinationSet::new("other".to_string(), &["z".to_string()], 1);
        let combo_val = combo.unrank(0);

        // Product expects Product variant
        assert!(product.rank(&combo_val).is_err());
    }

    #[test]
    fn test_union_rank_value_wrong_variant() {
        let set1 = CombinationSet::new("a".to_string(), &["x".to_string()], 1);
        let set2 = CombinationSet::new("b".to_string(), &["y".to_string()], 1);
        let union = UnionSet::new("test", vec![set1.into(), set2.into()]);

        let combo = CombinationSet::new("other".to_string(), &["z".to_string()], 1);
        let combo_val = combo.unrank(0);

        // Union expects UnionElement variant
        assert!(union.rank(&combo_val).is_err());
    }
}
