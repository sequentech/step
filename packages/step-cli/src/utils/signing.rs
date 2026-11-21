// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Protected actions: when the event's signing rule needs signatures, the
//! action answers a signing request instead of running, and runs only once
//! the request has collected them.

use std::fmt;

/// The exit status of a command whose action now waits for signatures:
/// neither done (0) nor failed (1).
pub const EXIT_SIGNATURES_PENDING: i32 = 3;

/// A signing request an action answered instead of running
/// (`SigningRequestSummary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSignatures {
    pub id: String,
    pub code: String,
    pub required: i64,
    pub expires_at: Option<String>,
}

impl fmt::Display for PendingSignatures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = if self.required == 1 { "" } else { "s" };
        write!(
            f,
            "waiting for {} signature{plural} (request {}, code {})",
            self.required, self.id, self.code
        )?;
        if let Some(expires_at) = &self.expires_at {
            write!(f, ", expires at {expires_at}")?;
        }
        Ok(())
    }
}

/// Builds a [`PendingSignatures`] from any generated `signing_request` type:
/// each GraphQL operation has its own copy of the summary.
#[macro_export]
macro_rules! pending_signatures {
    ($request:expr) => {
        $request.map(|request| $crate::utils::signing::PendingSignatures {
            id: request.id,
            code: request.code,
            required: request.required,
            expires_at: request.expires_at,
        })
    };
}

/// What a protected action answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signed<T> {
    /// The action ran.
    Done(T),
    /// The action waits for signatures and has not run.
    Pending(PendingSignatures),
}

/// What a trustee's key step answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    /// The step was recorded.
    Recorded,
    /// The custody proof is not this trustee's for this ceremony.
    Invalid,
    /// The trustee signs the request first; nothing was recorded.
    Pending(PendingSignatures),
}

impl StepOutcome {
    pub fn of(is_valid: bool, pending: Option<PendingSignatures>) -> Self {
        match (is_valid, pending) {
            (false, _) => StepOutcome::Invalid,
            (true, Some(pending)) => StepOutcome::Pending(pending),
            (true, None) => StepOutcome::Recorded,
        }
    }
}

/// Prints that the action waits for signatures and exits with
/// [`EXIT_SIGNATURES_PENDING`].
pub fn exit_pending(what: &str, pending: &PendingSignatures) -> ! {
    println!("{what} is {pending}.");
    std::process::exit(EXIT_SIGNATURES_PENDING);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PendingSignatures {
        PendingSignatures {
            id: "r1".into(),
            code: "AB-12".into(),
            required: 1,
            expires_at: None,
        }
    }

    #[test]
    fn a_valid_step_with_a_request_is_not_recorded() {
        assert_eq!(StepOutcome::of(true, None), StepOutcome::Recorded);
        assert_eq!(
            StepOutcome::of(true, Some(request())),
            StepOutcome::Pending(request())
        );
        assert_eq!(StepOutcome::of(false, None), StepOutcome::Invalid);
        assert_eq!(
            StepOutcome::of(false, Some(request())),
            StepOutcome::Invalid
        );
    }

    #[test]
    fn a_pending_request_names_its_signatures_id_and_code() {
        let pending = PendingSignatures {
            id: "4e0c".into(),
            code: "K7Q-2M".into(),
            required: 2,
            expires_at: None,
        };
        assert_eq!(
            pending.to_string(),
            "waiting for 2 signatures (request 4e0c, code K7Q-2M)"
        );
        let single = PendingSignatures {
            required: 1,
            expires_at: Some("2026-11-21T10:00:00Z".into()),
            ..pending
        };
        assert_eq!(
            single.to_string(),
            "waiting for 1 signature (request 4e0c, code K7Q-2M), expires at 2026-11-21T10:00:00Z"
        );
    }
}
