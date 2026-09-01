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
//! - the CONSTRUCTION reachability needs. Production decides reachability
//!   in `selection_capped` and `apply`, which read a contest and a decoded
//!   record rather than the spec's abstract cell, so
//!   [`reachability_from_production`] builds the contest a cell describes
//!   and tries to reach its state through those two — deriving the answer
//!   from production's rules instead of restating them here.
//!
//! Every effect is therefore production's own; what remains is converting
//! between the oracle's frozen enums and production's.
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
use validation_spec::{selection_class, Dialog, Effects, Emissions, Gate, InlineViews, Policies};

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
    let shown = native::visible_messages(
        &validator.config().policies,
        validator.messages(),
        is_review,
        is_touched,
    );
    shown.errors.into_iter().chain(shown.alerts).collect()
}

/// Converts the oracle's view of a ballot's selections to production's.
fn selections_of(vs: &spec::VoteState) -> native::SelectionClass {
    match selection_class(vs) {
        spec::SelectionClass::None => native::SelectionClass::None,
        spec::SelectionClass::Regular => native::SelectionClass::Regular,
        spec::SelectionClass::Marker => native::SelectionClass::Marker,
        spec::SelectionClass::Mixed => native::SelectionClass::Mixed,
    }
}

/// Converts production's tally class to the oracle's, so the two can be
/// compared cell by cell. The oracle keeps its own enum on purpose: it is
/// the frozen record of pre-fix behaviour and must not depend on the code
/// it measures.
fn tally_class(class: native::BallotClass) -> spec::BallotClass {
    match class {
        native::BallotClass::ExplicitInvalid => spec::BallotClass::ExplicitInvalid,
        native::BallotClass::ImplicitInvalid => spec::BallotClass::ImplicitInvalid,
        native::BallotClass::ExplicitBlank => spec::BallotClass::ExplicitBlank,
        native::BallotClass::ImplicitBlank => spec::BallotClass::ImplicitBlank,
        native::BallotClass::Declined => spec::BallotClass::Declined,
        native::BallotClass::Valid => spec::BallotClass::Valid,
    }
}

/// Derives reachability from production's own rules instead of modelling it.
///
/// Reachability asks whether the booth can form a given vote state at all.
/// Production answers that in two functions — `selection_capped`, which
/// decides when a control stops accepting selections, and `apply`, which
/// decides what a marker clears — so the honest derivation is to build the
/// contest the cell describes and try to reach the state through them, in
/// the order a voter would: the candidates, then the blank marker, then the
/// invalid flag. A state those edits cannot produce is not reachable, and
/// the way they fail says why.
///
/// The alternative, re-stating the predicate here, would make the sweep
/// compare production against a transcription of production.
fn reachability_from_production(config: &spec::Config, vs: &spec::VoteState) -> spec::Reachability {
    let contest = cell_contest(config, vs);
    let validator = native::ContestValidator::for_contest(&contest);
    let mut selection = untouched(&contest);

    let choose = |id: &str| {
        native::SelectionEdit::Choice(sequent_core::plaintext::DecodedVoteChoice {
            id: id.to_string(),
            selected: 0,
            write_in_text: None,
        })
    };

    // Each selection the voter would make, refused if the controls have
    // already stopped accepting them.
    for index in 0..vs.regulars {
        if validator.selection_capped(&selection) {
            return spec::Reachability::InputsDisabled;
        }
        selection = validator.apply(&selection, choose(&format!("r{index}")));
    }
    if vs.blank_marker {
        if validator.selection_capped(&selection) {
            return spec::Reachability::InputsDisabled;
        }
        selection = validator.apply(&selection, choose(BLANK_ID));
    }
    if vs.explicit_invalid {
        if validator.selection_capped(&selection) {
            return spec::Reachability::InputsDisabled;
        }
        selection = validator.apply(&selection, native::SelectionEdit::ExplicitInvalid(true));
    }

    // Whatever the edits produced, is it the state the cell asked for? If a
    // marker cleared something on the way, it is not.
    let reached = validator.vote_state(&selection);
    if reached.regulars == vs.regulars
        && reached.blank_marker == vs.blank_marker
        && reached.explicit_invalid == vs.explicit_invalid
    {
        spec::Reachability::Yes
    } else {
        spec::Reachability::MarkerCleared
    }
}

const BLANK_ID: &str = "blank";
const INVALID_ID: &str = "null";

/// The contest a cell describes: its bounds and policies, enough ordinary
/// candidates to hold the state's selections, and one of each marker.
fn cell_contest(config: &spec::Config, vs: &spec::VoteState) -> Contest {
    use sequent_core::ballot::{Candidate, CandidatePresentation, ContestPresentation};
    let candidate = |id: String, blank: bool, invalid: bool| Candidate {
        id,
        presentation: Some(CandidatePresentation {
            is_explicit_blank: Some(blank),
            is_explicit_invalid: Some(invalid),
            ..CandidatePresentation::default()
        }),
        ..Candidate::default()
    };
    let mut candidates: Vec<Candidate> = (0..vs.regulars)
        .map(|index| candidate(format!("r{index}"), false, false))
        .collect();
    candidates.push(candidate(BLANK_ID.to_string(), true, false));
    candidates.push(candidate(INVALID_ID.to_string(), false, true));
    Contest {
        min_votes: i64::from(config.min),
        max_votes: i64::from(config.max),
        presentation: Some(ContestPresentation {
            invalid_vote_policy: Some(wire(&config.policies.invalid)),
            blank_vote_policy: Some(wire(&config.policies.blank)),
            over_vote_policy: Some(wire(&config.policies.over)),
            under_vote_policy: Some(wire(&config.policies.under)),
            duplicated_rank_policy: Some(wire(&config.policies.dup)),
            preference_gaps_policy: Some(wire(&config.policies.gap)),
            ..ContestPresentation::default()
        }),
        candidates,
        ..Contest::default()
    }
}

fn untouched(contest: &Contest) -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: contest
            .candidates
            .iter()
            .map(|c| sequent_core::plaintext::DecodedVoteChoice {
                id: c.id.clone(),
                selected: -1,
                write_in_text: None,
            })
            .collect(),
    }
}

/// Computes the fixed mapping — the exact analog of the oracle's `f`, from
/// production's own rules.
pub fn f_fixed(config: &spec::Config, vs: &spec::VoteState) -> Effects {
    let validator = native::ContestValidator::from_config(native_config(config))
        .for_vote_state(native_vote_state(vs))
        .expect("the spec's cells always carry representable bounds");
    let em = Emissions {
        errors: validator.messages().errors.clone(),
        alerts: validator.messages().alerts.clone(),
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
        reachability: reachability_from_production(config, vs),
        tally: tally_class(native::classify(
            vs.decline,
            vs.explicit_invalid,
            !em.errors.is_empty(),
            selections_of(vs),
        )),
        emissions: em,
    }
}

/// Returns the spec-typed shape of a production contest's configuration —
/// for evaluating `f_fixed` on real fixture contests.
pub fn spec_config(contest: &Contest) -> Result<spec::Config, native::ValidationError> {
    let v = native::ContestValidator::for_contest(contest);
    let c = v.config()?;
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
    let v = native::ContestValidator::for_contest(contest).vote_state(decoded);
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
