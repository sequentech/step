// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Permutation set types for full permutations (n!) of all elements.
//!
//! This module provides `Permutation` value structs and `PermutationSet` for representing
//! and ranking full permutations where all elements from the ground set are used exactly once.

use anyhow::Result;
use number_encoding::factoradics;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use crate::{Rank, Ranked, RankedValue};

/// A value representing a specific permutation of elements.
///
/// # Example
/// ```json
/// {"member_of":"P(candidates)(3)","values":["Alice","Bob","Charlie"]}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Permutation {
    member_of: String,
    values: Vec<String>,
}

impl Permutation {
    fn new(member_of: String, values: Vec<String>) -> Self {
        Permutation { member_of, values }
    }

    /// Create a new Permutation from a PermutationSet with given values.
    ///
    /// # Errors
    /// Returns an error if the values are not valid for the given set.
    pub fn from_set(set: &PermutationSet, values: Vec<String>) -> Result<Self> {
        let permutation = Self::new(set.name().to_string(), values);
        set.validate(&permutation)?;
        Ok(permutation)
    }

    /// Parse a Permutation from JSON string.
    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Get the values (ordered elements) of this permutation.
    pub fn get_values(&self) -> &Vec<String> {
        &self.values
    }

    /// Get the name of the set this permutation belongs to.
    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// JSON serialization, the inverse of [`Permutation::from_string`].
impl fmt::Display for Permutation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

/// A set type representing all possible permutations of a ground set.
///
/// Permutations use all elements from the ground set exactly once, in different orders.
/// The cardinality is n! where n is the size of the ground set.
///
/// # Example
/// ```rust
/// use rnk::basic::PermutationSet;
/// use rnk::Ranked;
///
/// let candidates = PermutationSet::new(
///     "candidates".to_string(),
///     &["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()]
/// );
/// assert_eq!(candidates.cardinality(), 6); // 3! = 6
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PermutationSet {
    pub name: String,
    ground_set: Vec<String>,
}

impl PermutationSet {
    /// Create a new PermutationSet with the given name and ground set.
    ///
    /// # Panics
    /// Panics if the ground set contains duplicate elements.
    pub fn new(name: String, ground_set: &[String]) -> Self {
        let set: HashSet<String> = HashSet::from_iter(ground_set.iter().cloned());
        assert_eq!(
            set.len(),
            ground_set.len(),
            "PermutationSet: Options must be unique"
        );
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        PermutationSet {
            name: format!("P({})({})", name, v.len()),
            ground_set: v,
        }
    }

    fn size(&self) -> usize {
        self.ground_set.len()
    }

    /// Validate that a Permutation object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - exactly n elements are present (full permutation)
    /// - all elements are in the ground set
    /// - no duplicate elements
    /// - all ground set elements are present exactly once
    pub fn validate(&self, permutation: &Permutation) -> Result<()> {
        if permutation.member_of != self.name {
            return Err(anyhow::anyhow!("PermutationSet: not member of this set"));
        }

        if permutation.values.len() != self.size() {
            return Err(anyhow::anyhow!(
                "PermutationSet: invalid permutation length, expected {}, got {}",
                self.size(),
                permutation.values.len()
            ));
        }

        // Validate that it's a valid permutation: all ground set elements exactly once
        let mut found_elements = HashSet::new();
        for val in &permutation.values {
            if !self.ground_set.contains(val) {
                return Err(anyhow::anyhow!(
                    "PermutationSet: element '{}' not in ground set",
                    val
                ));
            }

            if !found_elements.insert(val.clone()) {
                return Err(anyhow::anyhow!(
                    "PermutationSet: duplicate element '{}' in permutation",
                    val
                ));
            }
        }

        if found_elements.len() != self.size() {
            return Err(anyhow::anyhow!(
                "PermutationSet: permutation must contain all ground set elements exactly once"
            ));
        }

        Ok(())
    }

    // Calculate factorial
    fn factorial(&self, n: usize) -> Rank {
        (1..=n).map(|i| i as Rank).product()
    }

    /// Rank a Permutation
    pub fn rank_permutation(&self, perm: &Permutation) -> Result<Rank> {
        self.validate(perm)?;

        // Factoradics to encode the permutation
        Ok(factoradics::encode(&perm.values) as Rank)
    }

    /// Unrank a rank to get a Permutation.
    pub fn unrank_permutation(&self, rank: Rank) -> Permutation {
        assert!(
            rank < self.cardinality(),
            "Invalid rank: {} >= {}",
            rank,
            self.cardinality()
        );

        // Factoradics to decode the rank to a permutation
        let perm_values = factoradics::decode(&self.ground_set, rank as usize);
        Permutation::from_set(self, perm_values)
            .expect("unrank_permutation: generated values should always be valid")
    }
}

impl Ranked for PermutationSet {
    fn cardinality(&self) -> Rank {
        self.factorial(self.size())
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let permutation: &Permutation = value.as_permutation()?;
        self.rank_permutation(permutation)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let permutation = self.unrank_permutation(rank);
        permutation.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::Permutation(permutation) => self.validate(permutation).is_ok(),
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
