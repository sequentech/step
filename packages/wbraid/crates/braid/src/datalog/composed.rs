// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The composed datalog program (§7.1, §7.6): splices the prelude and the
//! per-phase inference rules into a single ascent program, for use by the
//! trustee application.
//!
//! All types the rule templates reference (the [`Predicate`] enum and its
//! [`Slot`] trait, [`Action`], [`AccumulatorSet`], and the [`types`](super::types)
//! aliases) must be in scope at this expansion site.
//!
//! [`Predicate`]: crate::messages::predicate::Predicate
//! [`Slot`]: crate::messages::predicate::Slot

use super::accumulator::AccumulatorSet;
use super::types::*;
use super::{Action, MixSource};
use crate::messages::predicate::Predicate;

ascent::ascent! {
    include_source!(crate::datalog::prelude);
    include_source!(crate::datalog::dkg::infer::dkg_infer);
    include_source!(crate::datalog::mix::infer::mix_infer);
    include_source!(crate::datalog::decrypt::infer::decrypt_infer);
}

/// Run the datalog engine over a board's verified predicate set (§7.4).
///
/// Loads `predicates` as the EDB, runs the composed rules to fixpoint, and
/// returns the derived [`Action`]s. If any `error` fact was derived — an
/// equivocation, a configuration-domain violation, a public-key mismatch, a
/// malformed mix chain, ... — execution for this board must halt, so the
/// errors are returned as `Err` instead of actions.
pub fn run(predicates: &[Predicate]) -> Result<Vec<Action>, String> {
    let mut prog = AscentProgram {
        predicate: predicates.iter().map(|p| (p.clone(),)).collect(),
        ..Default::default()
    };
    prog.run();

    if !prog.error.is_empty() {
        let errors: Vec<String> = prog.error.into_iter().map(|e| e.0).collect();
        return Err(format!("datalog reported errors: {:?}", errors));
    }

    Ok(prog.action.into_iter().map(|a| a.0).collect())
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::datalog::Action;
    use crate::messages::newtypes::{zero_hash, ConfigurationHash};
    use crate::messages::predicate::{ConfigurationValid, Predicate};

    /// A lone `ConfigurationValid` predicate should make the trustee compute its
    /// DKG shares: the first action of the protocol.
    #[test]
    fn configuration_valid_triggers_compute_shares() {
        let cfg = ConfigurationHash(zero_hash());
        let predicates = vec![Predicate::ConfigurationValid(ConfigurationValid {
            configuration: cfg,
            threshold: 2,
            trustee_count: 3,
            self_index: 1,
        })];

        let actions = run(&predicates).expect("well-formed input must not error");

        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::ComputeShares(c, 1) if *c == cfg)),
            "expected ComputeShares(cfg, 1), got {:?}",
            actions
        );
    }
}
