// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `f_fixed` — the full effect record of the RATIONALIZED system, evaluated
//! over the spec's abstract cell types.
//!
//! The rules themselves are production's own (`sequent_core::validation`,
//! where the injection folded them) — emissions, the gates, AND the
//! inline views, which the booth now obtains from that module through a
//! wasm export rather than reimplementing in TypeScript. This module
//! supplies only what the full record needs beyond the rules:
//!
//! - the CONVERSIONS between the spec's abstract types and production's
//!   ([`wire`] — through the shared serde wire strings, loud on mismatch);
//! - the shared pure projections reused from the oracle where oracle and
//!   fixed genuinely agree, because no fix touched them: `classify` /
//!   `selection_class` (velvet's tally classifier) and `reachability`
//!   (the booth reducer, S5 kept).
//!
//! `f` (the oracle) and `f_fixed` share their composition shape and differ
//! ONLY by the fix ledger's changes; that difference, swept over the
//! certified domain, is the diff report (`characterization/fix-diff.md`).

use serde::de::DeserializeOwned;
use serde::Serialize;

use sequent_core::ballot::Contest;
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::validation as native;
use validation_spec as spec;
use validation_spec::{
    classify, reachability, selection_class, Dialog, Effects, Emissions, Gate, InlineViews,
    Policies,
};

/// Converts a policy enum between the spec's and production's types through
/// their shared serde wire strings. A mismatch (a variant one side lacks)
/// panics — the exhaustive apparatus surfaces that immediately, and it can
/// only mean the two vocabularies drifted.
fn wire<T: Serialize, U: DeserializeOwned>(t: &T) -> U {
    serde_json::from_value(serde_json::to_value(t).expect("serialize policy"))
        .expect("spec and production policy wire strings agree")
}

fn native_config(config: &spec::Config) -> native::Config {
    native::Config {
        min: config.min,
        max: config.max,
        policies: native::Policies {
            invalid: wire(&config.policies.invalid),
            blank: wire(&config.policies.blank),
            over: wire(&config.policies.over),
            under: wire(&config.policies.under),
            dup: wire(&config.policies.dup),
            gap: wire(&config.policies.gap),
        },
    }
}

/// Converts the spec's vote state to production's. `first_preferences` is
/// intentionally dropped: the rationalized rules read ONE selection count (the S6 fix is structural — the field does not
/// exist in production's `VoteState`).
fn native_vote_state(vs: &spec::VoteState) -> native::VoteState {
    native::VoteState {
        regulars: vs.regulars,
        blank_marker: vs.blank_marker,
        explicit_invalid: vs.explicit_invalid,
        decline: vs.decline,
        duplicate_ranks: vs.duplicate_ranks,
        rank_gaps: vs.rank_gaps,
    }
}

/// Returns what the voter sees on one screen as one list, which is how
/// `InlineViews` records an observation point. Production's display rules
/// answer with the errors and the alerts separately — the booth renders
/// them as two runs of boxes, errors above alerts — so flattening in that
/// order is this crate modelling the render, not a rule of its own.
fn shown(validator: &native::VoteValidator, is_review: bool, is_touched: bool) -> Vec<String> {
    let (errors, alerts) = native::visible_messages(
        &validator.config().policies,
        &validator.emissions().errors,
        &validator.emissions().alerts,
        is_review,
        is_touched,
    );
    errors.into_iter().chain(alerts).collect()
}

/// Computes the fixed mapping — the exact analog of the oracle's `f`, from
/// production's own rules.
pub fn f_fixed(config: &spec::Config, vs: &spec::VoteState) -> Effects {
    let validator = native::ContestValidator::from_config(native_config(config))
        .for_vote_state(native_vote_state(vs));
    let em = Emissions {
        errors: validator.emissions().errors.clone(),
        alerts: validator.emissions().alerts.clone(),
    };
    let hard = validator.hard_gate();
    let soft = validator.soft_gate();
    Effects {
        // The three observation points, each answered by production's own
        // inline rule (the booth calls the same one through wasm).
        inline: InlineViews {
            voting_untouched: shown(&validator, false, false),
            voting: shown(&validator, false, true),
            review: shown(&validator, true, true),
        },
        gate: Gate { hard, soft },
        dialog: if hard {
            Dialog::Blocking
        } else if soft {
            Dialog::Dismissible
        } else {
            Dialog::None
        },
        reachability: reachability(config, vs),
        tally: classify(
            vs.decline,
            vs.explicit_invalid,
            !em.errors.is_empty(),
            selection_class(vs),
        ),
        emissions: em,
    }
}

/// Returns the spec-typed shape of a production contest's configuration —
/// for evaluating `f_fixed` on real fixture contests.
pub fn spec_config(contest: &Contest) -> Result<spec::Config, native::ValidationError> {
    let c = native::contest_config(contest)?;
    Ok(spec::Config {
        min: c.min,
        max: c.max,
        policies: Policies {
            invalid: wire(&c.policies.invalid),
            blank: wire(&c.policies.blank),
            over: wire(&c.policies.over),
            under: wire(&c.policies.under),
            dup: wire(&c.policies.dup),
            gap: wire(&c.policies.gap),
        },
    })
}

/// Returns the spec-typed shape of a decoded contest's vote state. The
/// oracle-only
/// `first_preferences` is left absent (it defaults to `regulars`, which is
/// exact for plurality; `f_fixed` never reads it).
pub fn spec_vote_state(contest: &Contest, decoded: &DecodedVoteContest) -> spec::VoteState {
    let v = native::vote_state(contest, decoded);
    spec::VoteState {
        regulars: v.regulars,
        blank_marker: v.blank_marker,
        explicit_invalid: v.explicit_invalid,
        decline: v.decline,
        duplicate_ranks: v.duplicate_ranks,
        rank_gaps: v.rank_gaps,
        first_preferences: None,
    }
}
