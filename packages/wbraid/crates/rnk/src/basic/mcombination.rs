// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Multicombination set types for choosing k elements from n with repetition allowed.
//!
//! This module provides `MCombination` value structs and `MCombinationSet` for representing
//! and ranking multiset combinations where k elements are chosen from the ground set with repetition allowed.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use crate::{Rank, Ranked, RankedValue, decode_multi, encode_multi};

/// A value representing a specific multicombination of k elements (with repetition).
///
/// # Example
/// ```json
/// {"member_of":"MC(fruits)(3,2)","values":["apple","apple"]}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MCombination {
    member_of: String,
    values: Vec<String>,
}

impl MCombination {
    fn new(member_of: String, values: Vec<String>) -> Self {
        MCombination { member_of, values }
    }

    /// Create a new MCombination from a MCombinationSet with given values.
    ///
    /// # Errors
    /// Returns an error if the values are not valid for the given set.
    pub fn from_set(set: &MCombinationSet, values: Vec<String>) -> Result<Self> {
        let mcombination = Self::new(set.name().to_string(), values);
        set.validate(&mcombination)?;
        Ok(mcombination)
    }

    /// Parse a MCombination from JSON string.
    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Get the values (chosen elements, with possible repetition) of this multicombination.
    pub fn get_values(&self) -> &Vec<String> {
        &self.values
    }

    /// Get the name of the set this multicombination belongs to.
    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// JSON serialization, the inverse of [`MCombination::from_string`].
impl fmt::Display for MCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

/// A set type representing all possible multicombinations of k elements from a ground set.
///
/// Multicombinations choose k elements from the ground set with repetition allowed.
/// The cardinality is C(n+k-1,k) where n is the size of the ground set.
///
/// # Example
/// ```rust
/// use rnk::basic::MCombinationSet;
/// use rnk::Ranked;
///
/// let fruits = MCombinationSet::new(
///     "fruits".to_string(),
///     &["apple".to_string(), "banana".to_string(), "cherry".to_string()],
///     2
/// );
/// assert_eq!(fruits.cardinality(), 6); // C(3+2-1,2) = C(4,2) = 6
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MCombinationSet {
    pub name: String,
    ground_set: Vec<String>,
    k: usize,
}

impl MCombinationSet {
    /// Create a new MCombinationSet with the given name, ground set, and k value.
    ///
    /// # Panics
    /// Panics if the ground set contains duplicates.
    pub fn new(name: String, ground_set: &[String], k: usize) -> Self {
        let set: HashSet<String> = HashSet::from_iter(ground_set.iter().cloned());
        assert_eq!(
            set.len(),
            ground_set.len(),
            "MCombinationSet: Options must be unique"
        );
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        MCombinationSet {
            name: format!("MC({})({},{})", name, ground_set.len(), k),
            ground_set: v,
            k,
        }
    }

    fn size(&self) -> usize {
        self.ground_set.len()
    }

    // Calculate multiset coefficient: C(n + k - 1, k) = C(n + k - 1, n - 1)
    fn multiset_coefficient(n: usize, k: usize) -> Rank {
        if k == 0 {
            return 1;
        }
        number_encoding::combination(n + k - 1, k) as Rank
    }

    /// Validate that a MCombination object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - exactly k elements are present
    /// - all elements are in the ground set (repetitions allowed)
    pub fn validate(&self, combination: &MCombination) -> Result<()> {
        if combination.member_of != self.name {
            return Err(anyhow::anyhow!("member_of does not match set name"));
        }

        // Check that we have exactly k elements
        if combination.values.len() != self.k {
            return Err(anyhow::anyhow!(
                "Wrong number of elements in multicombination"
            ));
        }

        // Check that all elements are in the ground set (repetitions allowed)
        for value in &combination.values {
            if !self.ground_set.contains(value) {
                return Err(anyhow::anyhow!("Element not in ground set: {}", value));
            }
        }

        Ok(())
    }

    // Convert multicombination to rank using multicombinatorics
    fn multicombination_to_rank(&self, combination: &[String]) -> Rank {
        // Find indices of elements in the ground set (repetitions allowed)
        let mut indices = Vec::new();
        for element in combination {
            let index = self.ground_set.iter().position(|x| x == element)
                .expect("Element not in ground set - caller must validate before calling multicombination_to_rank");
            indices.push(index);
        }

        // Use encode_multi to encode the indices
        encode_multi(&indices) as Rank
    }

    // Convert rank to multicombination using multicombinatorics
    fn rank_to_multicombination(&self, rank: Rank) -> Vec<String> {
        // Use decode_multi to decode the rank to indices (with repetitions allowed)
        let indices = decode_multi(rank as usize, self.k);

        // Convert indices to ground set elements
        indices
            .into_iter()
            .map(|index| self.ground_set[index].clone())
            .collect()
    }

    /// Rank a MCombination.
    pub fn rank_mcombination(&self, combination: &MCombination) -> Result<Rank> {
        self.validate(combination)?;

        // Use multicombinatorics to encode the multicombination
        Ok(self.multicombination_to_rank(&combination.values))
    }

    /// Unrank a rank to get a MCombination.
    pub fn unrank_mcombination(&self, rank: Rank) -> MCombination {
        assert!(rank < self.cardinality(), "Invalid rank");

        // Use multicombinatorics to decode the rank to a multicombination
        let values = self.rank_to_multicombination(rank);
        MCombination::from_set(self, values)
            .expect("unrank_mcombination: generated values should always be valid")
    }
}

impl Ranked for MCombinationSet {
    fn cardinality(&self) -> Rank {
        Self::multiset_coefficient(self.size(), self.k)
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let mcombination: &MCombination = value.as_mcombination()?;
        self.rank_mcombination(mcombination)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let mcombination = self.unrank_mcombination(rank);
        mcombination.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::MCombination(mcombination) => self.validate(mcombination).is_ok(),
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
