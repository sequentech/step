// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The injection layer's workbench-facing API and conformance home.
//!
//! The wire-type derivations (`Contest` → `Config`,
//! `DecodedVoteContest` → `VoteState`) originated here and MOVED INTO
//! production when the gates were injected — they live in
//! `sequent_core::validation_provider`, the single source both production
//! call sites and this crate consume; this crate re-exports them and adds
//! the validator constructors. `validation-spec` itself remains free of
//! production types (the independence that makes the sweep meaningful).
//!
//! The conformance test (`tests/conformance.rs`) is the acceptance for the
//! whole injection layer: production's own codec and gate functions, run
//! natively over the bundled fixtures and a policy × vote-state matrix,
//! compared against the spec through these adapters — emissions against the
//! FROZEN ORACLE (decode is not injected), gates against the RATIONALIZED
//! `f_fixed` (the gates are injected, so production now carries the
//! ledger's gate fixes).

use sequent_core::ballot::Contest;
use sequent_core::plaintext::DecodedVoteContest;
use validation_spec::{BallotValidator, ContestValidator, VoteValidator};

pub use sequent_core::validation_provider::{contest_config, vote_state, ValidationProviderError};

/// The name this crate's API established before the derivations moved into
/// `sequent_core::validation_provider`.
pub type AdapterError = ValidationProviderError;

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
