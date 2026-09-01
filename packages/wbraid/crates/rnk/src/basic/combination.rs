// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Combination set types for choosing k elements from n without repetition.
//!
//! This module provides `Combination` value structs and `CombinationSet` for representing
//! and ranking combinations where k unique elements are chosen from the ground set.

use anyhow::Result;
use number_encoding::combinadics;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{Rank, Ranked, RankedValue};

/// A value representing a specific combination of k elements.
///
/// # Example
/// ```json
/// {"member_of":"C(colors)(4,2)","values":["red","blue"]}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Combination {
    member_of: String,
    values: Vec<String>,
}

impl Combination {
    fn new(member_of: String, values: Vec<String>) -> Self {
        Combination { member_of, values }
    }

    /// Create a new Combination from a CombinationSet with given values.
    ///
    /// # Errors
    /// Returns an error if the values are not valid for the given set.
    pub fn from_set(set: &CombinationSet, values: Vec<String>) -> Result<Self> {
        let combination = Self::new(set.name().to_string(), values);
        set.validate(&combination)?;
        Ok(combination)
    }

    /// Parse a Combination from JSON string.
    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialize this Combination to JSON string.
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Get the values (chosen elements) of this combination.
    pub fn get_values(&self) -> &Vec<String> {
        &self.values
    }

    /// Get the name of the set this combination belongs to.
    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// A set type representing all possible combinations of k elements from a ground set.
///
/// Combinations choose k unique elements from the ground set without regard to order.
/// The cardinality is C(n,k) = n!/(k!(n-k)!) where n is the size of the ground set.
///
/// # Example
/// ```rust
/// use rnk::basic::CombinationSet;
/// use rnk::Ranked;
///
/// let colors = CombinationSet::new(
///     "colors".to_string(),
///     &["red".to_string(), "green".to_string(), "blue".to_string(), "yellow".to_string()],
///     2
/// );
/// assert_eq!(colors.cardinality(), 6); // C(4,2) = 6
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CombinationSet {
    pub name: String,
    ground_set: Vec<String>,
    k: usize,
}

impl CombinationSet {
    /// Create a new CombinationSet with the given name, ground set, and k value.
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
            "CombinationSet: Options must be unique"
        );
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        CombinationSet {
            name: format!("C({})({},{})", name, ground_set.len(), k),
            ground_set: v,
            k,
        }
    }

    fn size(&self) -> usize {
        self.ground_set.len()
    }

    // Calculate binomial coefficient C(n, k)
    fn binomial_coefficient(n: usize, k: usize) -> Rank {
        number_encoding::combination(n, k) as Rank
    }

    // Convert combination to rank using combinadics
    fn combination_to_rank(&self, combination: &[String]) -> Rank {
        // Find indices of elements in the ground set
        let mut indices = Vec::new();
        for element in combination {
            let index = self.ground_set.iter().position(|x| x == element)
                .expect("Element not in ground set - caller must validate before calling combination_to_rank");
            indices.push(index);
        }
        indices.sort();

        // Use combinadics to encode the indices
        combinadics::encode(&indices) as Rank
    }

    // Convert rank to combination using combinadics
    fn rank_to_combination(&self, rank: Rank) -> Vec<String> {
        // Use combinadics to decode the rank to indices
        let indices = combinadics::decode(rank as usize, self.k);

        // Convert indices to ground set elements
        indices
            .into_iter()
            .map(|index| self.ground_set[index].clone())
            .collect()
    }

    /// Validate that a Combination object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - exactly k elements are present
    /// - all elements are in the ground set
    /// - no duplicate elements
    pub fn validate(&self, combination: &Combination) -> Result<()> {
        if combination.member_of != self.name {
            return Err(anyhow::anyhow!("member_of does not match set name"));
        }

        // Check that we have exactly k elements
        if combination.values.len() != self.k {
            return Err(anyhow::anyhow!("Wrong number of elements in combination"));
        }

        // Check that all elements are in the ground set and unique
        let mut seen = HashSet::new();
        for value in &combination.values {
            if !self.ground_set.contains(value) {
                return Err(anyhow::anyhow!("Element not in ground set: {}", value));
            }
            if !seen.insert(value.clone()) {
                return Err(anyhow::anyhow!(
                    "Duplicate element in combination: {}",
                    value
                ));
            }
        }

        Ok(())
    }

    /// Rank a Combination.
    pub fn rank_combination(&self, combination: &Combination) -> Result<Rank> {
        self.validate(combination)?;

        // Combinadics to encode the combination
        Ok(self.combination_to_rank(&combination.values))
    }

    /// Unrank a rank to get a Combination.
    pub fn unrank_combination(&self, rank: Rank) -> Combination {
        assert!(rank < self.cardinality(), "Invalid rank");

        // Combinadics to decode the rank to a combination
        let values = self.rank_to_combination(rank);
        Combination::from_set(self, values)
            .expect("unrank_object: generated values should always be valid")
    }
}

impl Ranked for CombinationSet {
    fn cardinality(&self) -> Rank {
        Self::binomial_coefficient(self.size(), self.k)
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let combination: &Combination = value.as_combination()?;
        self.rank_combination(combination)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let combination = self.unrank_combination(rank);
        combination.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::Combination(combination) => self.validate(combination).is_ok(),
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
