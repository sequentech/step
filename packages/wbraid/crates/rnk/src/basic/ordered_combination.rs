// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ordered combination set types for k-permutations (choosing and ordering k elements from n).
//!
//! This module provides `OrderedCombination` value structs and `OrderedCombinationSet` for representing
//! and ranking k-permutations where k unique elements are chosen from the ground set and arranged in order.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use crate::{Rank, Ranked, RankedValue};

/// A value representing a specific ordered combination (k-permutation) of k elements.
///
/// # Example
/// ```json
/// {"member_of":"OC(candidates)(4,3)","values":["Alice","Charlie","Bob"]}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderedCombination {
    member_of: String,
    values: Vec<String>,
}

impl OrderedCombination {
    fn new(member_of: String, values: Vec<String>) -> Self {
        OrderedCombination { member_of, values }
    }

    /// Create a new OrderedCombination from an OrderedCombinationSet with given values.
    ///
    /// # Errors
    /// Returns an error if the values are not valid for the given set.
    pub fn from_set(set: &OrderedCombinationSet, values: Vec<String>) -> Result<Self> {
        let ordered_combination = Self::new(set.name().to_string(), values);
        set.validate(&ordered_combination)?;
        Ok(ordered_combination)
    }

    /// Parse an OrderedCombination from JSON string.
    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Get the values (ordered chosen elements) of this k-permutation.
    pub fn get_values(&self) -> &Vec<String> {
        &self.values
    }

    /// Get the name of the set this k-permutation belongs to.
    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// JSON serialization, the inverse of [`OrderedCombination::from_string`].
impl fmt::Display for OrderedCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

/// A set type representing all possible k-permutations from a ground set.
///
/// Ordered combinations choose k unique elements from the ground set and arrange them in order.
/// This is also known as k-permutations or partial permutations.
/// The cardinality is P(n,k) = n!/(n-k)! where n is the size of the ground set.
///
/// # Example
/// ```rust
/// use rnk::basic::OrderedCombinationSet;
/// use rnk::Ranked;
///
/// let candidates = OrderedCombinationSet::new(
///     "candidates".to_string(),
///     &["Alice".to_string(), "Bob".to_string(), "Charlie".to_string(), "David".to_string()],
///     3
/// );
/// assert_eq!(candidates.cardinality(), 24); // P(4,3) = 4!/(4-3)! = 24
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OrderedCombinationSet {
    pub name: String,
    ground_set: Vec<String>,
    k: usize,
}

impl OrderedCombinationSet {
    /// Create a new OrderedCombinationSet with the given name, ground set, and k value.
    ///
    /// # Panics
    /// Panics if k > ground_set.len() or if the ground set contains duplicates.
    pub fn new(name: String, ground_set: &[String], k: usize) -> Self {
        assert!(
            k <= ground_set.len(),
            "k cannot be larger than ground set size"
        );

        let set: HashSet<String> = HashSet::from_iter(ground_set.iter().cloned());
        assert_eq!(
            set.len(),
            ground_set.len(),
            "OrderedCombinationSet: Options must be unique"
        );
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        OrderedCombinationSet {
            name: format!("OC({})({},{})", name, ground_set.len(), k),
            ground_set: v,
            k,
        }
    }

    fn size(&self) -> usize {
        self.ground_set.len()
    }

    /// Validate that an OrderedCombination object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - exactly k elements are present
    /// - all elements are in the ground set
    /// - no duplicate elements
    pub fn validate(&self, combination: &OrderedCombination) -> Result<()> {
        if combination.member_of != self.name {
            return Err(anyhow::anyhow!("member_of does not match set name"));
        }

        // Check that we have exactly k elements
        if combination.values.len() != self.k {
            return Err(anyhow::anyhow!(
                "Wrong number of elements in ordered combination"
            ));
        }

        // Check that all elements are in the ground set and unique
        let mut seen = HashSet::new();
        for value in &combination.values {
            if !self.ground_set.contains(value) {
                return Err(anyhow::anyhow!("Element not in ground set: {}", value));
            }
            if !seen.insert(value.clone()) {
                return Err(anyhow::anyhow!(
                    "Duplicate element in ordered combination: {}",
                    value
                ));
            }
        }

        Ok(())
    }

    // Calculate P(n,k) = n!/(n-k)! - permutations of k elements from n total
    fn permutation_coefficient(n: usize, k: usize) -> Rank {
        if k == 0 {
            return 1;
        }
        if k > n {
            return 0;
        }

        // Calculate n!/(n-k)! = n × (n-1) × ... × (n-k+1)
        let mut result = 1u64;
        for i in (n - k + 1)..=n {
            result *= i as u64;
        }
        result
    }

    // Encode k-permutation using modified factorial number system
    fn ordered_combination_to_rank(&self, combination: &[String]) -> Rank {
        // Convert to indices in ground_set
        let mut indices = Vec::new();
        for element in combination {
            let index = self.ground_set.iter().position(|x| x == element)
                .expect("Element not in ground set - caller must validate before calling ordered_combination_to_rank");
            indices.push(index);
        }

        // Direct k-factoradics: encode k-permutation without requiring full permutation
        let mut rank = 0u64;
        let mut available: Vec<usize> = (0..self.size()).collect();

        for (position, &chosen_index) in indices.iter().enumerate() {
            // Find position of chosen_index in remaining available elements
            let pos_in_available = available.iter().position(|&x| x == chosen_index).unwrap();

            // Add contribution: pos_in_available * (n-position-1)!
            let remaining_positions = self.k - position - 1;
            let factorial_base = if remaining_positions == 0 {
                1
            } else {
                let remaining_elements = available.len() - 1;
                // Calculate (remaining_elements)! / (remaining_elements - remaining_positions)!
                let mut factorial = 1u64;
                for i in 0..remaining_positions {
                    factorial *= (remaining_elements - i) as u64;
                }
                factorial
            };

            rank += pos_in_available as u64 * factorial_base;

            // Remove chosen element from available
            available.remove(pos_in_available);
        }

        rank
    }

    // Decode rank to k-permutation using direct k-factoradics
    fn rank_to_ordered_combination(&self, rank: Rank) -> Vec<String> {
        let mut result = Vec::new();
        let mut available: Vec<usize> = (0..self.size()).collect();
        let mut remaining_rank = rank;

        for position in 0..self.k {
            let remaining_positions = self.k - position - 1;
            let factorial_base = if remaining_positions == 0 {
                1
            } else {
                let remaining_elements = available.len() - 1;
                // Calculate (remaining_elements)! / (remaining_elements - remaining_positions)!
                let mut factorial = 1u64;
                for i in 0..remaining_positions {
                    factorial *= (remaining_elements - i) as u64;
                }
                factorial
            };

            // Find which element to choose at this position
            let choice_index = (remaining_rank / factorial_base) as usize;
            remaining_rank %= factorial_base;

            // Select the element and add to result
            let chosen_ground_index = available.remove(choice_index);
            result.push(self.ground_set[chosen_ground_index].clone());
        }

        result
    }

    /// Rank an OrderedCombination.
    pub fn rank_ordered_combination(&self, combination: &OrderedCombination) -> Result<Rank> {
        self.validate(combination)?;

        // Direct k-factoradics ranking
        Ok(self.ordered_combination_to_rank(&combination.values))
    }

    /// Unrank a rank to get an OrderedCombination.
    pub fn unrank_ordered_combination(&self, rank: Rank) -> OrderedCombination {
        assert!(rank < self.cardinality(), "Invalid rank");

        // Direct k-factoradics unranking
        let values = self.rank_to_ordered_combination(rank);
        OrderedCombination::from_set(self, values)
            .expect("unrank_ordered_combination: generated values should always be valid")
    }
}

impl Ranked for OrderedCombinationSet {
    fn cardinality(&self) -> Rank {
        Self::permutation_coefficient(self.size(), self.k)
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let ordered_combination: &OrderedCombination = value.as_ordered_combination()?;
        self.rank_ordered_combination(ordered_combination)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let ordered_combination = self.unrank_ordered_combination(rank);
        ordered_combination.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::OrderedCombination(ordered_combination) => {
                self.validate(ordered_combination).is_ok()
            }
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
