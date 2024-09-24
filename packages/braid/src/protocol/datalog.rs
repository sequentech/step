// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use log::{debug, error, info, trace};
use std::collections::HashSet;

use crate::protocol::action::Action;

pub(crate) const NULL_HASH: [u8; 64] = [0u8; 64];

pub(self) use crate::protocol::predicate::*;

pub(self) use crate::util::ProtocolError;
pub(self) use board_messages::braid::newtypes::*;
pub(self) use strand::hash::Hash;

pub(crate) fn hashes_init(value: Hash) -> THashes {
    let mut ret = [NULL_HASH; MAX_TRUSTEES];
    ret[0] = value;

    ret
}

pub(crate) fn hashes_set(mut input: THashes, index: usize, value: Hash) -> THashes {
    input[index] = value;

    input
}

pub(crate) fn hashes_add(mut input: THashes, value: Hash) -> THashes {
    // expect: cannot happen due to (n < threshold) condition in decrypt.rs logic
    let index = input
        .iter()
        .position(|t| t == &NULL_HASH)
        .expect("impossible");
    input[index] = value;

    input
}

pub(crate) fn trustees_init(value: TrusteePosition) -> TrusteeSet {
    let mut ret = [NULL_TRUSTEE; MAX_TRUSTEES];
    ret[0] = value;

    ret
}

pub(crate) fn trustees_add(mut input: TrusteeSet, value: TrusteePosition) -> TrusteeSet {
    // expect: cannot happen due to (n < threshold) condition in decrypt.rs logic
    let index = input
        .iter()
        .position(|t| t == &NULL_TRUSTEE)
        .expect("impossible");

    input[index] = value;

    input
}

pub(crate) fn trustees_count(input: TrusteeSet) -> usize {
    input.iter().filter(|t| *t != &NULL_TRUSTEE).count()
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

#[derive(Debug)]
pub(crate) enum Phase {
    Cfg(cfg::D),
    Dkg(dkg::D),
    Shuffle(shuffle::D),
    Decrypt(decrypt::D),
}

impl Phase {
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum DatalogError {
    MixRepeat(ConfigurationHash, BatchNumber),
}

pub(crate) mod cfg;
pub(crate) mod decrypt;
pub(crate) mod dkg;
pub(crate) mod shuffle;
