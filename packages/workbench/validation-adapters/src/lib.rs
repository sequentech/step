// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The workbench's bridge to production's validation rules.
//!
//! The rationalized implementation lives IN PRODUCTION
//! (`sequent_core::validation` — the fold-in that completed the injection);
//! this crate is what the evidence apparatus links to evaluate it:
//!
//! - [`f_fixed`] evaluates the full effect record of the rationalized
//!   system over the spec's abstract cell types (`Config` × `VoteState` →
//!   `Effects`), composing production's own rules with the workbench
//!   models of the effects production computes elsewhere (the booth's
//!   rewritten inline filter, velvet's field-driven tally classifier,
//!   the booth's reachability). Served to the runners by this crate's
//!   `emit-grid` binary (the `fixed` kind; the oracle kinds read
//!   `validation-spec` directly).
//! - The conformance suite (`tests/conformance.rs`) is the native
//!   acceptance: production's own codec and gate functions over the real
//!   bundled fixtures, compared against `f_fixed`.
//!
//! `validation-spec` itself remains free of production types — it is the
//! FROZEN ORACLE's home, and the oracle stays independent of the code it
//! measures. The dependency arrow here runs workbench → production, which
//! is the sound direction.

mod fixed;
pub use fixed::{f_fixed, spec_config, spec_vote_state};

use sequent_core::ballot::Contest;
use sequent_core::plaintext::DecodedVoteContest;
pub use sequent_core::validation::{
    contest_config, policy_emissions, vote_state, BallotValidator, ContestValidator,
    ValidationError, VoteValidator,
};

/// The name this crate's API established before the derivations moved into
/// production.
pub type AdapterError = ValidationError;

/// Stage 0 from a production contest (config known).
pub fn for_contest(contest: &Contest) -> Result<ContestValidator, AdapterError> {
    Ok(ContestValidator::from_config(contest_config(contest)?))
}

/// Stage 1 from a production contest + decoded record (per edit / per decode).
pub fn for_vote(
    contest: &Contest,
    decoded: &DecodedVoteContest,
) -> Result<VoteValidator, AdapterError> {
    Ok(for_contest(contest)?.for_vote_state(vote_state(contest, decoded)))
}

/// The whole-ballot composition (the Next/review transition): one
/// (contest, decoded) pair per contest on the ballot, all pages.
pub fn for_ballot<'a>(
    pairs: impl IntoIterator<Item = (&'a Contest, &'a DecodedVoteContest)>,
) -> Result<BallotValidator, AdapterError> {
    let votes = pairs
        .into_iter()
        .map(|(c, d)| for_vote(c, d))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BallotValidator::from_votes(votes))
}
