// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The query-provider — the RATIONALIZED (fixed) implementation of vote
//! validation (distillation step 5, phase 2).
//!
//! Production computes validation in six scattered sites that each re-derive
//! their facts locally and drift. This provider derives the vote-state facts
//! ONCE and answers every site as a projection of that single derivation, so
//! the drift bugs have nowhere to live. Two stages, keyed on how much is
//! known:
//!
//!   ContestValidator   — config only (policies + bounds). Answers
//!                        `reachability(requested)` (an intention, possibly
//!                        unformable) and `for_vote_state(vs)` — THE
//!                        derivation point.
//!   VoteValidator      — config + the one derived VoteState, and the ONE
//!                        selection count and checker record derived from it.
//!                        Answers the four ballot-queries. Owns its data.
//!   BallotValidator    — the other axis (composition, not accumulation):
//!                        ORs the gates across a ballot's contests.
//!
//! NOT bug-compatible, by design. `f` (lib.rs) is the frozen oracle — the
//! bug-compatible free functions, reproducing production byte-identically —
//! and this provider is the fixed re-implementation. Writing it the rational
//! way, "as if the bug had never existed," makes three quirks disappear by
//! construction — you cannot write them on purpose:
//!
//!   * S6 (gates counted first preferences, the checker counted all ranked)
//!     — there is ONE selection count [`selections`], used by both the
//!     checker rules and the gates. No `first_preferences`.
//!   * S4 (the checker's under-vote zone included the empty ballot, the gate
//!     re-derived it without) — there is ONE under-vote predicate
//!     [`is_undervote`], shared by the checker alert and the gate, and it
//!     excludes the empty ballot (that is the blank rule's domain).
//!   * D3 (the selectedMax alert deduped against itself and so never
//!     rendered) — [`derive_rendered_keys`] dedups an alert against the
//!     ERROR copy, which is what the dedup was always meant to do. (D3 was
//!     latent: the error copy is always present when the alert is, so this
//!     changes no cell — but the rational code cannot reproduce the bug.)
//!
//! The surprising-but-intended rules are PRESERVED — they are real rules a
//! rational implementation still encodes, not accidents: S3 (a marker counts
//! as one selection — inside [`selections`]), S2 (a marker-only ballot with
//! any error classifies invalid, not blank — inside the reused [`classify`]),
//! S1 (invalid=allowed mutes inline errors — inside [`derive_rendered_keys`]),
//! and S5 (the invalid marker preserves co-selected choices — the reused
//! [`reachability`], intentional per upstream #2949). Whether any of those
//! is itself wrong is a separate, deliberate judgment (distillation step 5,
//! phase 3), recorded in the quirk ledger (lib.rs `quirks`).
//!
//! Where oracle and fixed diverge (emissions, the gates, the inline dedup)
//! this module computes its own; the genuinely-unchanged pure classifiers
//! (`classify`, `selection_class`) and `reachability` are reused, not copied
//! — phase 3 forks those if and when it changes them.
//!
//! The acceptance artifact is therefore NOT the byte-identical sweep (which
//! measures the oracle) but `characterization/fix-diff.md`: `f` vs `f_fixed`
//! over the certified domain, every differing cell attributed to one
//! intended fix, zero unexplained.
//!
//! ABSTRACT ONLY, on purpose. These take `Config` / `VoteState`, never
//! production's `Contest` / `DecodedVoteContest` — the adapters that derive
//! the abstract shapes from the wire types belong to the production-injection
//! branch, and keeping them out preserves this crate's independence.

use crate::{
    classify, reachability, selection_class, BallotClass, BlankVotePolicy, Config, Dialog,
    Effects, Emissions, Gate, InlineViews, InvalidVotePolicy, OverVotePolicy, Policies, RankPolicy,
    Reachability, UnderVotePolicy, VoteState, BLANK_VOTE, DUPLICATED_POSITION, EXPLICIT_ALERT,
    EXPLICIT_NOT_ALLOWED, EXPLICIT_OR_ENCODING, OVER_VOTE_DISABLED, PREFERENCE_ORDER_WITH_GAPS,
    SELECTED_MAX, SELECTED_MIN, UNDER_VOTE,
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
    /// Reuses the shared predicate; S5 (the invalid marker preserving
    /// co-selected choices) is intentional (#2949) and preserved.
    pub fn reachability(&self, requested: &VoteState) -> Reachability {
        reachability(&self.config, requested)
    }

    /// THE derivation step: fix the vote-state facts once — the single
    /// selection count and the checker record computed from it — and hand
    /// back the stage-1 validator. Everything downstream reads this one
    /// derivation, which is why the count/zone drift has nowhere to live.
    pub fn for_vote_state(&self, vs: VoteState) -> VoteValidator {
        let n = selections(&vs);
        let em = derive_emissions(&self.config, &vs, n);
        VoteValidator {
            config: self.config,
            vs,
            n,
            em,
        }
    }
}

/// Stage 1 — config + this contest's one derived VoteState, its single
/// selection count `n`, and the checker record. Self-contained.
pub struct VoteValidator {
    config: Config,
    vs: VoteState,
    /// The one selection count (see [`selections`]) — used by the gates as
    /// well as the checker, so gate and checker cannot disagree (S6).
    n: u32,
    em: Emissions,
}

impl VoteValidator {
    /// The checker record (checker.rs).
    pub fn emissions(&self) -> &Emissions {
        &self.em
    }

    /// The blocking submission gate (voting_screen.rs, per contest).
    pub fn hard_gate(&self) -> bool {
        derive_hard_gate(&self.config, self.n, &self.em)
    }

    /// The dismissible submission gate (voting_screen.rs, per contest).
    pub fn soft_gate(&self) -> bool {
        derive_soft_gate(&self.config, &self.vs, self.n, &self.em)
    }

    /// What the filter renders at one observation point (InvalidErrorsList).
    pub fn inline(&self, point: ObservationPoint) -> Vec<String> {
        let views = derive_inline_views(&self.config.policies, &self.em.errors, &self.em.alerts);
        match point {
            ObservationPoint::UntouchedVoting => views.voting_untouched,
            ObservationPoint::Voting => views.voting,
            ObservationPoint::Review => views.review,
        }
    }

    /// The tally class (classify_ballot). Reuses the shared classifier — the
    /// precedence there (S2) is a preserved rule, not a phase-2 fix.
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

// ---------------------------------------------------------------------------
// The rational compute — one count, one under-vote predicate, an honest dedup
// ---------------------------------------------------------------------------

/// The single selection count — regulars plus each marker. Derived ONCE and
/// used by both the checker rules and the gates, so there is no separate
/// first-preference count for the two to disagree over (S6 gone). A selected
/// blank marker and a set invalid flag each count as one selection — the S3
/// rule, a deliberate modelling choice, preserved here.
fn selections(vs: &VoteState) -> u32 {
    vs.regulars + u32::from(vs.blank_marker) + u32::from(vs.explicit_invalid)
}

/// The one under-vote predicate, shared by the checker alert and the gate: a
/// non-empty ballot short of the maximum but at least the minimum. It
/// excludes the empty ballot — that is the blank rule's domain, not the
/// under-vote rule's — so the checker and the gate agree at `n = 0` (S4 gone).
fn is_undervote(config: &Config, n: u32) -> bool {
    n > 0 && n >= config.min && n < config.max
}

/// The checker record — the invalid → over → min → under → blank →
/// preference-gap → duplicated-rank calls of `raw_ballot.rs::decode`, reading
/// the single count `n` and the single under-vote predicate.
fn derive_emissions(config: &Config, vs: &VoteState, n: u32) -> Emissions {
    let p = &config.policies;
    let mut errors: Vec<String> = Vec::new();
    let mut alerts: Vec<String> = Vec::new();

    if vs.explicit_invalid {
        if p.invalid == InvalidVotePolicy::NotAllowed {
            errors.push(EXPLICIT_NOT_ALLOWED.into());
        }
        if p.invalid == InvalidVotePolicy::WarnInvalidImplicitAndExplicit {
            alerts.push(EXPLICIT_ALERT.into());
        }
    }
    if n > config.max {
        errors.push(SELECTED_MAX.into());
        if p.over != OverVotePolicy::Allowed {
            alerts.push(SELECTED_MAX.into());
        }
    } else if n == config.max && p.over == OverVotePolicy::NotAllowedWithMsgAndDisable {
        alerts.push(OVER_VOTE_DISABLED.into());
    }
    if n < config.min {
        errors.push(SELECTED_MIN.into());
    }
    if is_undervote(config, n) && p.under != UnderVotePolicy::Allowed {
        alerts.push(UNDER_VOTE.into());
    }
    if n == 0 && !vs.explicit_invalid && p.blank != BlankVotePolicy::Allowed {
        if p.blank == BlankVotePolicy::NotAllowed {
            errors.push(BLANK_VOTE.into());
        } else {
            alerts.push(BLANK_VOTE.into());
        }
    }
    if vs.rank_gaps {
        errors.push(PREFERENCE_ORDER_WITH_GAPS.into());
    }
    if vs.duplicate_ranks {
        errors.push(DUPLICATED_POSITION.into());
    }
    Emissions { errors, alerts }
}

/// `check_voting_not_allowed_next_util`, reading the single count `n`.
fn derive_hard_gate(config: &Config, n: u32, em: &Emissions) -> bool {
    let p = &config.policies;
    em.errors
        .iter()
        .any(|m| EXPLICIT_OR_ENCODING.contains(&m.as_str()))
        || (!em.errors.is_empty() && p.invalid == InvalidVotePolicy::NotAllowed)
        || (n == 0 && p.blank == BlankVotePolicy::NotAllowed)
        || (n > config.max && p.over == OverVotePolicy::NotAllowedWithMsgAndAlert)
        || (p.dup == RankPolicy::NotAllowedWarnAndDialog
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == RankPolicy::NotAllowedWarnAndDialog
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

/// `check_voting_error_dialog_util`, reading the single count `n` and the
/// single under-vote predicate.
fn derive_soft_gate(config: &Config, vs: &VoteState, n: u32, em: &Emissions) -> bool {
    let p = &config.policies;
    (!em.errors.is_empty()
        && p.invalid != InvalidVotePolicy::Allowed
        && p.invalid != InvalidVotePolicy::AllowedWithExclusiveExplicit)
        || (p.invalid == InvalidVotePolicy::WarnInvalidImplicitAndExplicit && vs.explicit_invalid)
        || (p.blank == BlankVotePolicy::Warn && n == 0)
        || (n > config.max && p.over == OverVotePolicy::AllowedWithMsgAndAlert)
        || (is_undervote(config, n) && p.under == UnderVotePolicy::WarnAndAlert)
        || (p.dup == RankPolicy::AllowedWarnAndDialog
            && em.errors.iter().any(|m| m == DUPLICATED_POSITION))
        || (p.gap == RankPolicy::AllowedWarnAndDialog
            && em.errors.iter().any(|m| m == PREFERENCE_ORDER_WITH_GAPS))
}

/// One observation point of `InvalidErrorsList.tsx::filterErrorList`: alert
/// visibility → dedup → the master keep-list on errors; errors render first.
fn derive_rendered_keys(
    p: &Policies,
    errors: &[String],
    alerts: &[String],
    is_review: bool,
) -> Vec<String> {
    // Alert visibility — the only point-dependent rules.
    let mut kept_alerts: Vec<&String> = alerts
        .iter()
        .filter(|m| {
            !((m.as_str() == UNDER_VOTE
                && !is_review
                && p.under == UnderVotePolicy::WarnOnlyInReview)
                || (m.as_str() == BLANK_VOTE
                    && !is_review
                    && p.blank == BlankVotePolicy::WarnOnlyInReview)
                || (m.as_str() == OVER_VOTE_DISABLED && is_review))
        })
        .collect();
    // Dedup: an empty ballot shows the blank message, not the under-vote
    // hint; and an alert whose key already renders as an error is redundant
    // (errors render first), so drop it. The second clause is deduping
    // against the ERROR copy — not against the alert itself (that was D3).
    let blank_present = kept_alerts.iter().any(|a| a.as_str() == BLANK_VOTE)
        || errors.iter().any(|e| e == BLANK_VOTE);
    kept_alerts.retain(|m| {
        !((m.as_str() == UNDER_VOTE && blank_present)
            || errors.iter().any(|e| e.as_str() == m.as_str()))
    });
    // The master keep-list: under invalid=allowed every error is hidden
    // except the two carve-outs (S1 — a preserved rule).
    let kept_errors = errors.iter().filter(|m| {
        if p.invalid != InvalidVotePolicy::Allowed
            && p.invalid != InvalidVotePolicy::AllowedWithExclusiveExplicit
        {
            return true;
        }
        (m.as_str() == SELECTED_MAX && p.over != OverVotePolicy::Allowed)
            || (m.as_str() == BLANK_VOTE && p.blank == BlankVotePolicy::NotAllowed)
    });
    kept_errors
        .cloned()
        .chain(kept_alerts.into_iter().cloned())
        .collect()
}

fn derive_inline_views(p: &Policies, errors: &[String], alerts: &[String]) -> InlineViews {
    InlineViews {
        voting_untouched: Vec::new(),
        voting: derive_rendered_keys(p, errors, alerts, false),
        review: derive_rendered_keys(p, errors, alerts, true),
    }
}

/// The fixed mapping — the exact analog of `f`, composed from this provider
/// instead of the oracle's free functions. `f` and `f_fixed` share their
/// composition shape and differ ONLY by the fork's fixes; that difference,
/// swept over the certified domain, is the diff report (fix-diff.md).
pub fn f_fixed(config: &Config, vs: &VoteState) -> Effects {
    let cv = ContestValidator::from_config(*config);
    let vv = cv.for_vote_state(*vs);
    let hard = vv.hard_gate();
    let soft = vv.soft_gate();
    Effects {
        inline: InlineViews {
            voting_untouched: vv.inline(ObservationPoint::UntouchedVoting),
            voting: vv.inline(ObservationPoint::Voting),
            review: vv.inline(ObservationPoint::Review),
        },
        gate: Gate { hard, soft },
        dialog: if hard {
            Dialog::Blocking
        } else if soft {
            Dialog::Dismissible
        } else {
            Dialog::None
        },
        reachability: cv.reachability(vs),
        tally: vv.tally(),
        emissions: vv.emissions().clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::f;

    fn cfg(min: u32, max: u32) -> Config {
        Config {
            min,
            max,
            policies: Policies::default(),
        }
    }

    /// Off the fix cells, the fixed implementation still equals the oracle
    /// field-for-field — the fork changes only what the fixes touch. (A
    /// min-vote cell: n = 0 with min = 1, so neither the S6 count nor the S4
    /// zero-zone is in play.)
    fn plain() -> (Config, VoteState) {
        (
            cfg(1, 2),
            VoteState {
                regulars: 0,
                ..VoteState::default()
            },
        )
    }

    #[test]
    fn fixed_matches_oracle_off_the_fix_cells() {
        let (config, vs) = plain();
        assert_eq!(f_fixed(&config, &vs), f(&config, &vs));
    }

    /// The projections compose back to `f_fixed` — the interface is a
    /// faithful decomposition of the fixed mapping.
    #[test]
    fn queries_compose_to_f_fixed() {
        let (config, vs) = plain();
        let cv = ContestValidator::from_config(config);
        let vv = cv.for_vote_state(vs);
        let effects = f_fixed(&config, &vs);

        assert_eq!(vv.emissions(), &effects.emissions);
        assert_eq!(vv.hard_gate(), effects.gate.hard);
        assert_eq!(vv.soft_gate(), effects.gate.soft);
        assert_eq!(vv.tally(), effects.tally);
        assert_eq!(vv.inline(ObservationPoint::Review), effects.inline.review);
        assert_eq!(cv.reachability(&vs), effects.reachability);
    }

    /// S6 fixed: a ranked ballot of two candidates with max = 1 is an
    /// over-vote to the checker (it emits selectedMax) AND now to the gate.
    /// The oracle gate counted only the single first preference, so it stayed
    /// open while the checker flagged the error — the silent-discount shape.
    #[test]
    fn s6_gate_counts_all_ranked_selections() {
        let config = Config {
            min: 0,
            max: 1,
            policies: Policies {
                over: OverVotePolicy::NotAllowedWithMsgAndAlert,
                ..Policies::default()
            },
        };
        // Two candidates ranked; one sits at rank 0 (an ordinary ranking).
        let vs = VoteState {
            regulars: 2,
            first_preferences: Some(1),
            ..VoteState::default()
        };
        let oracle = f(&config, &vs);
        let fixed = f_fixed(&config, &vs);
        // The checker sees the over-vote in both.
        assert!(oracle.emissions.errors.iter().any(|m| m == SELECTED_MAX));
        assert!(fixed.emissions.errors.iter().any(|m| m == SELECTED_MAX));
        // But only the oracle gate misses it (counted 1, not 2).
        assert!(!oracle.gate.hard);
        assert!(fixed.gate.hard);
    }

    /// S4 fixed: the empty ballot is not an under-vote. With min = 0 the
    /// oracle checker emitted an under-vote alert on the empty ballot
    /// (overlapping the blank rule); the fixed checker does not.
    #[test]
    fn s4_empty_ballot_is_not_an_undervote() {
        let config = Config {
            min: 0,
            max: 2,
            policies: Policies {
                under: UnderVotePolicy::WarnAndAlert,
                ..Policies::default()
            },
        };
        let vs = VoteState {
            regulars: 0,
            ..VoteState::default()
        };
        assert!(f(&config, &vs).emissions.alerts.iter().any(|m| m == UNDER_VOTE));
        assert!(!f_fixed(&config, &vs)
            .emissions
            .alerts
            .iter()
            .any(|m| m == UNDER_VOTE));
    }

    /// BallotValidator ORs the gates across contests — not reachable through
    /// `f_fixed` (which is per-contest), so it is only covered here.
    #[test]
    fn ballot_gate_is_the_or_across_contests() {
        let clean = ContestValidator::from_config(cfg(1, 2)).for_vote_state(VoteState {
            regulars: 1,
            ..VoteState::default()
        });
        assert!(!clean.hard_gate() && !clean.soft_gate());

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
        let requested = VoteState {
            regulars: 1,
            explicit_invalid: true,
            ..VoteState::default()
        };
        assert_eq!(cv.reachability(&requested), Reachability::MarkerCleared);
    }
}
