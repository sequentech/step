// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Composite set types for combinatorial structures.
//!
//! # Set Types
//! - [`ProductSet`] - Cartesian product of multiple sets
//! - [`UnionSet`] - Disjoint union of multiple sets
//!
//! # Value Types
//!
//! - [`Product`] - Represents a product of multiple set elements
//! - [`UnionElement`] - Represents a union element of multiple sets
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use crate::{Rank, Ranked, RankedSet, RankedValue};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Product {
    member_of: String,
    values: Vec<RankedValue>,
}
impl Product {
    pub fn new(member_of: String, values: Vec<RankedValue>) -> Self {
        Product { member_of, values }
    }

    /// Create a new Product from a ProductSet with given values.
    ///
    /// # Errors
    /// Returns an error if the values are not valid for the given set.
    pub fn from_set(set: &ProductSet, values: Vec<RankedValue>) -> Result<Self> {
        let product = Product {
            member_of: set.name().to_string(),
            values,
        };
        set.validate(&product)?;
        Ok(product)
    }

    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn get_values(&self) -> &Vec<RankedValue> {
        &self.values
    }

    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// JSON serialization, the inverse of [`Product::from_string`].
impl fmt::Display for Product {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UnionElement {
    member_of: String,
    value: Box<RankedValue>,
}
impl UnionElement {
    pub fn new(member_of: String, value: RankedValue) -> Self {
        UnionElement {
            member_of,
            value: Box::new(value),
        }
    }

    /// Create a new UnionElement from a UnionSet with given value.
    ///
    /// # Errors
    /// Returns an error if the value is not valid for the given set.
    pub fn from_set(set: &UnionSet, value: RankedValue) -> Result<Self> {
        let element = UnionElement {
            member_of: set.name().to_string(),
            value: Box::new(value),
        };
        set.validate(&element)?;
        Ok(element)
    }

    pub fn from_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn get_value(&self) -> &RankedValue {
        &self.value
    }

    pub fn get_member_of(&self) -> &String {
        &self.member_of
    }
}

/// JSON serialization, the inverse of [`UnionElement::from_string`].
impl fmt::Display for UnionElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UnionSet {
    pub name: String,
    sets: Vec<RankedSet>,
}
impl UnionSet {
    pub fn new(name: &str, sets: Vec<RankedSet>) -> Self {
        // Check that all sets have unique names
        let mut set_names = HashSet::new();
        for set in &sets {
            let set_name = set.name();
            if !set_names.insert(set_name.to_string()) {
                panic!("UnionSet: Duplicate set name '{}'", set_name);
            }
        }

        UnionSet {
            name: name.to_string(),
            sets,
        }
    }

    /// Validate that a UnionElement object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - value is a member of at least one constituent set
    pub fn validate(&self, element: &UnionElement) -> Result<()> {
        if element.member_of != self.name {
            return Err(anyhow::anyhow!("UnionSet: not member of this set"));
        }

        // Check if the value is a member of any constituent set
        for set in &self.sets {
            if set.is_member(&element.value) {
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "UnionSet: value not a member of any constituent set"
        ))
    }

    pub fn rank_union_element(&self, element: &UnionElement) -> Result<Rank> {
        self.validate(element)?;

        // Offset-based ranking
        let mut current_offset = 0;
        for set in &self.sets {
            if set.is_member(&element.value) {
                let inner_rank = set.rank(&element.value)?;
                return Ok(current_offset + inner_rank);
            }
            current_offset += set.cardinality();
        }

        Err(anyhow::anyhow!(
            "UnionSet: Element not found in any constituent set"
        ))
    }

    pub fn unrank_union_element(&self, rank: Rank) -> UnionElement {
        assert!(rank < self.cardinality(), "Invalid rank");

        // Offset-based decoding
        let mut current_offset = 0;
        for set in &self.sets {
            let set_cardinality = set.cardinality();
            if rank < current_offset + set_cardinality {
                let inner_rank = rank - current_offset;
                let inner_value = set.unrank(inner_rank);
                return UnionElement::from_set(self, inner_value)
                    .expect("unrank_product: generated value should always be valid");
            }
            current_offset += set_cardinality;
        }

        panic!("Invalid rank: {}", rank);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProductSet {
    pub name: String,
    sets: Vec<RankedSet>,
}
impl ProductSet {
    pub fn new(name: &str, sets: Vec<RankedSet>) -> Self {
        // Check that all sets have unique names
        let mut set_names = HashSet::new();
        for set in &sets {
            let set_name = set.name();
            if !set_names.insert(set_name.to_string()) {
                panic!("ProductSet: Duplicate set name '{}'", set_name);
            }
        }

        ProductSet {
            name: name.to_string(),
            sets,
        }
    }

    /// Validate that a Product object is a valid member of this set.
    ///
    /// Checks that:
    /// - member_of field matches this set's name
    /// - number of values matches number of constituent sets
    /// - each value is a member of the corresponding constituent set
    pub fn validate(&self, product: &Product) -> Result<()> {
        if product.member_of != self.name {
            return Err(anyhow::anyhow!("ProductSet: not member of this set"));
        }

        if product.values.len() != self.sets.len() {
            return Err(anyhow::anyhow!("ProductSet: invalid number of values"));
        }

        for (i, set) in self.sets.iter().enumerate() {
            if !set.is_member(&product.values[i]) {
                return Err(anyhow::anyhow!(
                    "ProductSet: value at index {} not a member of set",
                    i
                ));
            }
        }

        Ok(())
    }

    pub fn rank_product(&self, product: &Product) -> Result<Rank> {
        self.validate(product)?;

        // Mixed-radix encoding
        let mut acc = 0;
        let mut multiplier = 1;

        for (i, value) in product.values.iter().enumerate() {
            let rank = self.sets[i].rank(value)?;
            acc += rank * multiplier;
            multiplier *= self.sets[i].cardinality();
        }
        Ok(acc)
    }

    pub fn unrank_product(&self, rank: Rank) -> Product {
        assert!(rank < self.cardinality(), "Invalid rank");

        // Mixed-radix decoding
        let mut remaining_rank = rank;
        let mut component_ranks = Vec::new();

        // Decompose the rank using mixed-radix arithmetic
        for set in &self.sets {
            let cardinality = set.cardinality();
            let component_rank = remaining_rank % cardinality;
            component_ranks.push(component_rank);
            remaining_rank /= cardinality;
        }

        // Construct individual component values
        let mut values = Vec::new();
        for (i, &component_rank) in component_ranks.iter().enumerate() {
            let component_value = self.sets[i].unrank(component_rank);
            values.push(component_value);
        }

        Product::from_set(self, values)
            .expect("unrank_product: generated values should always be valid")
    }
}

impl Ranked for ProductSet {
    fn cardinality(&self) -> Rank {
        self.sets.iter().map(|s| s.cardinality()).product()
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let product: &Product = value.as_product()?;
        self.rank_product(product)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let product = self.unrank_product(rank);
        product.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::Product(product) => self.validate(product).is_ok(),
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Ranked for UnionSet {
    fn cardinality(&self) -> Rank {
        self.sets.iter().map(|s| s.cardinality()).sum()
    }

    fn rank(&self, value: &RankedValue) -> Result<Rank> {
        let element: &UnionElement = value.as_union_element()?;
        self.rank_union_element(element)
    }

    fn unrank(&self, rank: Rank) -> RankedValue {
        let element = self.unrank_union_element(rank);
        element.into()
    }

    fn is_member(&self, value: &RankedValue) -> bool {
        match value {
            RankedValue::UnionElement(element) => self.validate(element).is_ok(),
            _ => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
