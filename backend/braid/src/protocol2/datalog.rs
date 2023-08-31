pub use log::{debug, error, info, trace};
pub use std::collections::HashSet;
use tracing_attributes::instrument;

use crate::protocol2::action::Action;
use crate::protocol2::statement::THashes;
use crate::protocol2::Hash;
pub(crate) const NULL_HASH: [u8; 64] = [0u8; 64];
pub const NULL_TRUSTEE: usize = 1001;

pub(crate) use crate::protocol2::predicate::*;
pub(crate) use crate::protocol2::PROTOCOL_MANAGER_INDEX;
pub(crate) use crate::protocol2::VERIFIER_INDEX;

pub(crate) fn hashes_init(value: Hash) -> THashes {
    let mut ret = [NULL_HASH; crate::protocol2::MAX_TRUSTEES];
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
    let mut ret = [NULL_TRUSTEE; crate::protocol2::MAX_TRUSTEES];
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

///////////////////////////////////////////////////////////////////////////
// A Vec<Phase> loosely corresponds to a state machine.
//
// The input data, Predicate, is defined in predicate.rs
// The output data, Action, is defined in action.rs. Some outputs
// are also Predicates that feed to subsequent Phases.
///////////////////////////////////////////////////////////////////////////

pub(crate) fn get_phases() -> Vec<Phase> {
    vec![
        Phase::Cfg(cfg::S),
        Phase::Dkg(dkg::S),
        Phase::Shuffle(shuffle::S),
        Phase::Decrypt(decrypt::S),
    ]
}

#[instrument(skip_all)]
pub(crate) fn run(predicates: &Vec<Predicate>) -> (HashSet<Action>, Vec<Predicate>) {
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
        next.2.into_iter().for_each(|d| {
            error!("Datalog returned error {:?}", d);
            panic!();
        });
    }

    (actions, all_predicates)
}

#[derive(Debug)]
pub(crate) enum Phase {
    Cfg(cfg::S),
    Dkg(dkg::S),
    Shuffle(shuffle::S),
    Decrypt(decrypt::S),
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