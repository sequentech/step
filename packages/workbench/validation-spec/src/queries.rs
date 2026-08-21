// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The query-provider — the rationalized shape of vote validation
//! (distillation step 5, phase 1: the workbench reference).
//!
//! Production computes validation in six scattered sites that each
//! re-derive their facts locally and drift (S4, S6). The rationalized shape
//! derives the vote-state facts ONCE and answers every site as a projection
//! of that single derivation. Two stages, keyed on how much is known:
//!
//!   ContestValidator   — config only (policies + bounds). Answers
//!                        `reachability(requested)` (an intention, possibly
//!                        unformable) and `for_vote_state(vs)` — THE
//!                        derivation point.
//!   VoteValidator      — config + the one derived VoteState. Answers the
//!                        four ballot-queries. Owns its data (no borrow of
//!                        the ContestValidator): the marker structure is
//!                        construction-time-only, so nothing a query reads
//!                        lives back in stage 0.
//!   BallotValidator    — the other axis (composition, not accumulation):
//!                        ORs the gates across a ballot's contests.
//!
//! ABSTRACT ONLY, on purpose. These take `Config` / `VoteState`, never
//! production's `Contest` / `DecodedVoteContest` — the adapters that derive
//! the abstract shapes from the wire types belong to the production-injection
//! branch, and keeping them out preserves this crate's independence (the
//! thing that makes the sweep's agreement real).
//!
//! Phase 1 is bug-compatible: each query delegates to the existing spec
//! internals, so `f` routed through here reproduces the mapping exactly (the
//! byte-identical sweep is the proof). Bug-compatibility is a temporary
//! sanity check — later phases isolate the quirks and apply the fixes we
//! judge warranted.

use crate::{
    classify, emissions, hard_gate, inline_views, reachability, selection_class, soft_gate,
    BallotClass, Config, Emissions, Reachability, VoteState,
};

/// Where an inline effect is read (the query parameter of the filter site).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationPoint {
    /// The voting screen before the voter's first selection — the
    /// untouched-clear renders nothing.
    UntouchedVoting,
    /// The voting screen after a selection has armed the touch.
    Voting,
    /// The review screen.
    Review,
}

/// Stage 0 — config is known (in production: the contest definition).
pub struct ContestValidator {
    config: Config,
}

impl ContestValidator {
    /// The abstract constructor. (The production-typed `for_contest(&Contest)`
    /// lives on the injection branch.)
    pub fn from_config(config: Config) -> Self {
        ContestValidator { config }
    }

    /// The booth's upstream enforcement, modelled: does this INTENTION form?
    /// Takes a requested state that may be unformable — never a cast ballot.
    pub fn reachability(&self, requested: &VoteState) -> Reachability {
        reachability(&self.config, requested)
    }

    /// THE derivation step: fix the vote-state facts once and hand back the
    /// stage-1 validator. Everything downstream reads this single derivation,
    /// which is why the drift bugs have nowhere to live.
    pub fn for_vote_state(&self, vs: VoteState) -> VoteValidator {
        let em = emissions(&self.config, &vs);
        VoteValidator {
            config: self.config.clone(),
            vs,
            em,
        }
    }
}

/// Stage 1 — config + this contest's one derived VoteState. Self-contained:
/// no reference back to the ContestValidator (see the module note).
pub struct VoteValidator {
    config: Config,
    vs: VoteState,
    em: Emissions,
}

impl VoteValidator {
    /// The checker record (checker.rs).
    pub fn emissions(&self) -> &Emissions {
        &self.em
    }

    /// The blocking submission gate (voting_screen.rs, per contest).
    pub fn hard_gate(&self) -> bool {
        hard_gate(&self.config, &self.vs, &self.em)
    }

    /// The dismissible submission gate (voting_screen.rs, per contest).
    pub fn soft_gate(&self) -> bool {
        soft_gate(&self.config, &self.vs, &self.em)
    }

    /// What the filter renders at one observation point (InvalidErrorsList).
    pub fn inline(&self, point: ObservationPoint) -> Vec<String> {
        let views = inline_views(&self.config.policies, &self.em.errors, &self.em.alerts);
        match point {
            ObservationPoint::UntouchedVoting => views.voting_untouched,
            ObservationPoint::Voting => views.voting,
            ObservationPoint::Review => views.review,
        }
    }

    /// The tally class (classify_ballot). Reads config only through
    /// emissions (`tally ⊥ config | emissions`).
    pub fn tally(&self) -> BallotClass {
        classify(
            self.vs.decline,
            self.vs.explicit_invalid,
            !self.em.errors.is_empty(),
            selection_class(&self.vs),
        )
    }
}

/// The composition axis: production's gates OR across every contest on the
/// ballot. Owns its per-contest validators outright.
pub struct BallotValidator {
    contests: Vec<VoteValidator>,
}

impl BallotValidator {
    pub fn from_votes(contests: Vec<VoteValidator>) -> Self {
        BallotValidator { contests }
    }

    /// Blocking if ANY contest blocks.
    pub fn hard_gate(&self) -> bool {
        self.contests.iter().any(VoteValidator::hard_gate)
    }

    /// Dismissible dialog if ANY contest raises one.
    pub fn soft_gate(&self) -> bool {
        self.contests.iter().any(VoteValidator::soft_gate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{f, BlankVotePolicy, InvalidVotePolicy, Policies};

    fn cfg(min: u32, max: u32) -> Config {
        Config {
            min,
            max,
            policies: Policies::default(),
        }
    }

    /// The two-stage queries compose back to `f`'s fields — the interface is
    /// a faithful decomposition of the mapping (the sweep proves this
    /// exhaustively; this pins it in-crate as the API contract).
    #[test]
    fn queries_compose_to_f() {
        let config = cfg(1, 2);
        let vs = VoteState {
            regulars: 0,
            ..VoteState::default()
        };
        let cv = ContestValidator::from_config(config);
        let vv = cv.for_vote_state(vs);
        let effects = f(&config, &vs);

        assert_eq!(vv.emissions(), &effects.emissions);
        assert_eq!(vv.hard_gate(), effects.gate.hard);
        assert_eq!(vv.soft_gate(), effects.gate.soft);
        assert_eq!(vv.tally(), effects.tally);
        assert_eq!(vv.inline(ObservationPoint::Review), effects.inline.review);
        assert_eq!(cv.reachability(&vs), effects.reachability);
    }

    /// BallotValidator ORs the gates across contests — not reachable through
    /// `f` (which is per-contest), so it is only covered here.
    #[test]
    fn ballot_gate_is_the_or_across_contests() {
        // A clean contest: one regular, min 1 max 2, default policies — no gate.
        let clean = ContestValidator::from_config(cfg(1, 2)).for_vote_state(VoteState {
            regulars: 1,
            ..VoteState::default()
        });
        assert!(!clean.hard_gate() && !clean.soft_gate());

        // A blocking contest: empty ballot, blank = not-allowed → hard gate.
        let blocking_cfg = Config {
            min: 0,
            max: 2,
            policies: Policies {
                blank: BlankVotePolicy::NotAllowed,
                ..Policies::default()
            },
        };
        let blocking = ContestValidator::from_config(blocking_cfg).for_vote_state(VoteState {
            regulars: 0,
            ..VoteState::default()
        });
        assert!(blocking.hard_gate());

        // The ballot blocks iff ANY contest does.
        let only_clean = BallotValidator::from_votes(vec![
            ContestValidator::from_config(cfg(1, 2)).for_vote_state(VoteState {
                regulars: 1,
                ..VoteState::default()
            }),
        ]);
        assert!(!only_clean.hard_gate());

        let with_blocking = BallotValidator::from_votes(vec![
            ContestValidator::from_config(cfg(1, 2)).for_vote_state(VoteState {
                regulars: 1,
                ..VoteState::default()
            }),
            ContestValidator::from_config(blocking_cfg).for_vote_state(VoteState {
                regulars: 0,
                ..VoteState::default()
            }),
        ]);
        assert!(with_blocking.hard_gate());
    }

    /// Reachability is answered at stage 0 from the intention — including one
    /// that will not form (the exclusive-explicit clear).
    #[test]
    fn reachability_reads_the_intention_at_stage_zero() {
        let cv = ContestValidator::from_config(Config {
            min: 0,
            max: 2,
            policies: Policies {
                invalid: InvalidVotePolicy::AllowedWithExclusiveExplicit,
                ..Policies::default()
            },
        });
        // regular + invalid flag under the exclusive policy: cannot form.
        let requested = VoteState {
            regulars: 1,
            explicit_invalid: true,
            ..VoteState::default()
        };
        assert_eq!(cv.reachability(&requested), Reachability::MarkerCleared);
    }
}
