// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Finding the structured reasons behind a failed import.
//!
//! An importer refuses a file by returning a
//! [`Rejected`](sequent_core::election_config::Rejected) somewhere in its error
//! chain. Everything that only prints the error keeps printing the same English
//! it always did; whoever answers the operator — harvest's response, the task's
//! annotations — asks this module for the problems instead, so the Admin Portal
//! can say them in the operator's language.

use crate::types::error::Error;
use sequent_core::election_config::{Problem, Rejected};

/// Refuse a file for one reason, as an `anyhow` error.
pub fn reject(what: &str, problem: Problem) -> anyhow::Error {
    anyhow::Error::new(Rejected::one(what, problem))
}

/// The rejection somewhere in `error`'s chain, however deeply it was wrapped.
pub fn rejection_of(error: &anyhow::Error) -> Option<&Rejected> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<Rejected>())
}

/// The problems a failed import can show, when it failed for reasons it knows.
pub fn problems_of(error: &anyhow::Error) -> Option<Vec<Problem>> {
    rejection_of(error).map(|rejected| rejected.report.problems.clone())
}

/// The same, for windmill's own error type.
pub fn problems_in(error: &Error) -> Option<Vec<Problem>> {
    match error {
        Error::Anyhow(error) => problems_of(error),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use sequent_core::election_config::import_problems;

    #[test]
    fn a_rejection_is_found_through_context() {
        let error: anyhow::Error =
            reject("voters file", import_problems::duplicate_column("email"));
        let wrapped = Err::<(), _>(error)
            .context("Error obtaining copy_from query")
            .unwrap_err();

        let problems = problems_of(&wrapped).expect("the rejection survives the context");
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].id.as_deref(), Some("voters.duplicate-column"));
        assert_eq!(
            problems_in(&Error::Anyhow(wrapped)).map(|p| p.len()),
            Some(1)
        );
    }

    #[test]
    fn an_ordinary_error_has_no_problems() {
        assert!(problems_of(&anyhow::anyhow!("database is down")).is_none());
        assert!(problems_in(&Error::String("x".into())).is_none());
    }
}
