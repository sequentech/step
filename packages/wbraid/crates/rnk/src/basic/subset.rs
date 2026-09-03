// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Subset set types for representing all possible subsets (power set elements).
//!
//! This module provides `Subset` value structs and `SubsetSet` for representing
//! and ranking all possible subsets of a ground set, including the empty set.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use crate::{Rank, Ranked, RankedValue};

/// A value representing a specific subset of elements.
///
/// # Example
/// ```json
/// {"member_of":"S(colors)(3)","values":["red","blue"]}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Subset {
    member_of: String,
    values: Vec<String>,
}

impl Subset {
    fn new(member_of: String, values: Vec<String>) -> Self {
        Subset { member_of, values }
    }

    /// Create a new Subset from a SubsetSet with given values.
    ///
    /// # Errors
    /// Returns an error if the values are not valid for the given set.
    pub fn from_set(set: &SubsetSet, values: Vec<String>) -> Result<Self> {
        let subset = Self::new(set.name().to_string(), values);
        set.validate(&subset)?;
        Ok(subset)
    }

    /// Parse a Subset from JSON string.
    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Get the values (elements in this subset) of this subset.
    pub fn get_values(&self) -> &Vec<String> {
        &self.values
    }

    /// Get the name of the set this subset belongs to.
    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// JSON serialization, the inverse of [`Subset::from_string`].
impl fmt::Display for Subset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

/// A set type representing all possible subsets of a ground set (power set).
///
/// Subsets can contain any combination of elements from the ground set, including
/// the empty set and the full set. The cardinality is 2^n where n is the size of the ground set.
///
/// # Example
/// ```rust
/// use rnk::basic::SubsetSet;
/// use rnk::Ranked;
///
/// let colors = SubsetSet::new(
///     "colors".to_string(),
///     &["red".to_string(), "green".to_string(), "blue".to_string()]
/// );
/// assert_eq!(colors.cardinality(), 8); // 2^3 = 8
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SubsetSet {
    pub name: String,
    ground_set: Vec<String>,
}

impl SubsetSet {
    /// Create a new SubsetSet with the given name and ground set.
    ///
    /// # Panics
    /// Panics if the ground set contains duplicates or if the ground set is too large (≥64 elements).
    pub fn new(name: String, ground_set: &[String]) -> Self {
        let set: HashSet<String> = HashSet::from_iter(ground_set.iter().cloned());
        assert_eq!(
            set.len(),
            ground_set.len(),
            "SubsetSet: Options must be unique"
        );
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        SubsetSet {
            name: format!("S({})({})", name, v.len()),
            ground_set: v,
        }
    }

    fn size(&self) -> usize {
        self.ground_set.len()
    }

    /// Validate that a Subset object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - all elements are in the ground set
    /// - no duplicate elements
    pub fn validate(&self, subset: &Subset) -> Result<()> {
        if subset.member_of != self.name {
            return Err(anyhow::anyhow!("member_of does not match set name"));
        }

        // Check that all elements are in the ground set and unique
        let mut seen = HashSet::new();
        for value in &subset.values {
            if !self.ground_set.contains(value) {
                return Err(anyhow::anyhow!("Element not in ground set: {}", value));
            }
            if !seen.insert(value.clone()) {
                return Err(anyhow::anyhow!("Duplicate element in subset: {}", value));
            }
        }

        Ok(())
    }

    // Convert subset to rank using binary representation
    fn subset_to_rank(&self, subset: &[String]) -> Rank {
        let mut rank = 0u64;

        for element in subset {
            let index = self.ground_set.iter().position(|x| x == element).expect(
                "Element not in ground set - caller must validate before calling subset_to_rank",
            );
            rank |= 1u64 << index;
        }

        rank
    }

    // Convert rank to subset using binary representation
    fn rank_to_subset(&self, rank: Rank) -> Vec<String> {
        let mut result = Vec::new();

        for (index, element) in self.ground_set.iter().enumerate() {
            if rank & (1u64 << index) != 0 {
                result.push(element.clone());
            }
        }

        result
    }

    /// Rank a Subset.
    pub fn rank_subset(&self, subset: &Subset) -> Result<Rank> {
        self.validate(subset)?;

        // Binary representation to encode the subset
        Ok(self.subset_to_rank(&subset.values))
    }

    /// Unrank a rank to get a Subset.
    pub fn unrank_subset(&self, rank: Rank) -> Subset {
        assert!(rank < self.cardinality(), "Invalid rank");

        // Binary representation to decode the rank to a subset
        let values = self.rank_to_subset(rank);
        Subset::from_set(self, values)
            .expect("unrank_subset: generated values should always be valid")
    }
}

impl Ranked for SubsetSet {
    fn cardinality(&self) -> Rank {
        // 2^n subsets for n elements
        if self.size() >= 64 {
            panic!(
                "SubsetSet too large: {} elements would overflow u64",
                self.size()
            );
        }
        1u64 << self.size()
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let subset: &Subset = value.as_subset()?;
        self.rank_subset(subset)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let subset = self.unrank_subset(rank);
        subset.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::Subset(subset) => self.validate(subset).is_ok(),
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
