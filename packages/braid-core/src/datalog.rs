// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use log::{debug, error, info, trace};
use std::collections::HashSet;

use crate::protocol::action::Action;
pub(self) use crate::protocol::predicate::*;

pub(self) use crate::util::ProtocolError;
pub(self) use b3::messages::newtypes::*;
pub(self) use strand::hash::Hash;

/// Marks a value in a THashes array as empty.
pub(crate) const NULL_HASH: [u8; 64] = [0u8; 64];

/// Returns a new empty THashes array.
///
/// A THashes array is an array of hashes with fixed size equal
/// to the maximum possible number of trustees. The array is marked
/// as empty with all values equal to the constant NULL_HASH.
pub(crate) fn hashes_init(value: Hash) -> THashes {
    let mut ret = [NULL_HASH; MAX_TRUSTEES];
    ret[0] = value;

    ret
}

/// Sets the value of the THashes array at the given position,
/// returning the modified array.
pub(crate) fn hashes_set(mut input: THashes, index: usize, value: Hash) -> THashes {
    input[index] = value;

    input
}

/// Appends a hash to the given THashes array, returning the modified
/// array.
///
/// The end of the array is marked by the first NULL_HASH value.
pub(crate) fn hashes_add(mut input: THashes, value: Hash) -> THashes {
    let index = input
        .iter()
        .position(|t| t == &NULL_HASH)
        // expect: cannot happen due to (n < threshold) condition in decrypt.rs logic
        .expect("impossible");
    input[index] = value;

    input
}

/// Returns a new TrusteeSet array with the given TrusteePosition
/// value at the first (0) position.
///
/// A TrusteeSet is an array of integers with fixed size equal
/// to the maximum possible number of trustees. It represents
/// a set of trustees, each identified by their position in the
/// the protocol's Configuration, starting at 1.
pub(crate) fn trustees_init(value: TrusteePosition) -> TrusteeSet {
    let mut ret = [NULL_TRUSTEE; MAX_TRUSTEES];
    ret[0] = value;

    ret
}

/// Appends a hash to the given TrusteeSet array, returning the modified
/// array.
///
/// The end of the array is marked by the first NULL_TRUSTEE value.
pub(crate) fn trustees_add(mut input: TrusteeSet, value: TrusteePosition) -> TrusteeSet {
    let index = input
        .iter()
        // expect: cannot happen due to (n < threshold) condition in decrypt.rs logic
        .position(|t| t == &NULL_TRUSTEE)
        .expect("impossible");

    input[index] = value;

    input
}

/// Returns the size of a TrusteeSet, the number of trustees
/// in the set.
///
/// The size is defined as the number of values that are not
/// NULL_TRUSTEE.
pub(crate) fn trustees_count(input: TrusteeSet) -> usize {
    input.iter().filter(|t| *t != &NULL_TRUSTEE).count()
}

/// Returns the size of a THashes array, the number of hashes
/// in the set.
///
/// The size is defined as the number of values that are not
/// NULL_HASH.
pub(crate) fn hashes_count(input: &THashes) -> usize {
    input.iter().filter(|t| *t != &NULL_HASH).count()
}

/// Returns the Phases that make up the protocol's main steps.
// A Vec<Phase> loosely corresponds to a state machine.
///
/// The input data, Predicate, is defined in predicate.rs
/// The output data, Action, is defined in action.rs. Some outputs
/// are also Predicates that feed to subsequent Phases (marked
/// as struct OutP in datalog the modules)
pub(crate) fn get_phases() -> Vec<Phase> {
    vec![
        Phase::Cfg(cfg::D),
        Phase::Dkg(dkg::D),
        Phase::Shuffle(shuffle::D),
        Phase::Decrypt(decrypt::D),
    ]
}

/// Runs datalog inference for all Phases against thee input Predicates.
///
/// The protocol is advanced through the execution of Actions output by
/// datalog inference rules. For greater clarity, these rules are decomposed
/// into four parts which are represented by Phase objects. These are:
///
/// Cfg: approving and signing the protocol configuration
/// Dkg: the distributed key generation as described in Cortier et al.;
/// based on Pedersen
/// Shuffle: Verifiable shuffling as described in Haenni et al, Haines;
/// based on Wikstrom et al.
/// Decrypt: Verifiable distributed decryption, as described in Cortier et al.;
/// based on Pedersen. Decryption is verifiable through Chaum-Pedersen proofs
/// of discrete log equality.
///
/// Each Phase outputs its required Actions which are accumulated. The output
/// predicates are also accumulated and passed on to subsequent Phases.
///
/// Returns the set of required Actions.
pub(crate) fn run(predicates: &Vec<Predicate>) -> Result<HashSet<Action>, ProtocolError> {
    let phases = get_phases();
    let mut all_predicates = vec![];
    for p in predicates {
        all_predicates.push(*p);
    }
    debug!(
        "Running with all predicates {}",
        all_predicates
            .iter()
            .map(|p| format!("\n\r{p:?}"))
            .collect::<Vec<String>>()
            .join("")
    );

    let mut actions = HashSet::new();
    for p in phases {
        let next = p.run(&all_predicates);
        debug!("Phase {:?} returns {} new predicates", p, next.0.len());
        next.0.into_iter().for_each(|p| {
            trace!("Adding output predicate {:?}", p);
            all_predicates.push(p);
        });
        next.1.into_iter().for_each(|a| {
            actions.insert(a);
        });

        if next.2.len() > 0 {
            let mut errs = vec![];
            next.2.iter().for_each(|d| {
                error!("Datalog returned error {:?}", d);
                errs.push(format!("{d:?}"));
            });
            return Err(ProtocolError::DatalogError(errs.join(",")));
        }
    }

    Ok(actions)
}

/// One of four subsets of rules that define the protocol.
#[derive(Debug)]
pub(crate) enum Phase {
    Cfg(cfg::D),
    Dkg(dkg::D),
    Shuffle(shuffle::D),
    Decrypt(decrypt::D),
}

impl Phase {
    /// Runs this Phase with the given input predicates.
    ///
    /// Returns the Actions that must be performed with
    /// respect to this subset of protocol rules.
    fn run(
        &self,
        predicates: &Vec<Predicate>,
    ) -> (HashSet<Predicate>, HashSet<Action>, HashSet<DatalogError>) {
        match self {
            Self::Cfg(s) => s.run(predicates),
            Self::Dkg(s) => s.run(predicates),
            Self::Shuffle(s) => s.run(predicates),
            Self::Decrypt(s) => s.run(predicates),
        }
    }
}
/// Represents errors raised when running the datalog engine.
///
/// These are inconsistent states that are explicitly detected
/// as inference rules.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum DatalogError {
    MixRepeat(ConfigurationHash, BatchNumber),
}

pub(crate) mod cfg;
pub(crate) mod decrypt;
pub(crate) mod dkg;
pub(crate) mod shuffle;
