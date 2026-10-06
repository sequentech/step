// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Basic set types for combinatorial structures.
//!
//! This module contains all the fundamental set types for representing different
//! combinatorial structures and their corresponding value types.
//!
//! # Set Types
//!
//! - [`PermutationSet`] - Full permutations of all elements (n!)
//! - [`CombinationSet`] - Combinations of k elements without repetition (C(n,k))
//! - [`MCombinationSet`] - Multicombinations with repetition allowed (C(n+k-1,k))
//! - [`SubsetSet`] - All possible subsets (power set, 2^n)
//! - [`OrderedCombinationSet`] - k-permutations, ordered selections (P(n,k))
//!
//! # Value Types
//!
//! - [`Combination`] - Represents a specific combination of elements
//! - [`MCombination`] - Represents a specific multicombination of elements
//! - [`OrderedCombination`] - Represents a specific ordered combination of elements
//! - [`Permutation`] - Represents a specific permutation of elements
//! - [`Subset`] - Represents a specific subset of elements

pub mod combination;
pub mod mcombination;
pub mod ordered_combination;
pub mod permutation;
pub mod subset;

// Re-export all public types for easy access
pub use combination::{Combination, CombinationSet};
pub use mcombination::{MCombination, MCombinationSet};
pub use ordered_combination::{OrderedCombination, OrderedCombinationSet};
pub use permutation::{Permutation, PermutationSet};
pub use subset::{Subset, SubsetSet};
