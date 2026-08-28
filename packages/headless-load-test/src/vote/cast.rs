// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `insert_cast_vote` and outcome classification.
//!
//! Hasura wraps Harvest's `insert-cast-vote` action response, so every
//! call comes back HTTP 200 from Hasura's point of view regardless of the
//! underlying outcome — the only signal available is
//! `errors[0].extensions.code` (`packages/harvest/src/routes/insert_cast_vote.rs:113-287`).
//! `VoterStateLocked` (a same-voter concurrent-write conflict, expected to
//! be transient) and a genuine `CheckStatusFailed` share that same code, so
//! they're told apart by message text — the lock's message is fixed
//! (`"The voter state is being updated; retry the vote"`,
//! `insert_cast_vote.rs:153-157`).

use graphql_client::GraphQLQuery;

use crate::hasura::{first_error_code, HasuraClient};
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_cast_vote.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertCastVote;

const VOTER_STATE_LOCKED_MESSAGE: &str = "The voter state is being updated; retry the vote";

#[derive(Debug, Clone)]
pub enum CastOutcome {
    Success {
        id: String,
    },
    /// Concurrent write to the same voter's row — expected only if two
    /// workers ever draw the same voter; retryable.
    VoterStateLocked,
    /// `CheckRevotesFailed` or `InsertFailedExceedsAllowedRevotes` — the
    /// election's revote policy rejected this cast.
    RevoteLimitExceeded,
    /// Any other classified GraphQL error (auth, validation, ...).
    Rejected {
        code: String,
        message: String,
    },
    /// Transport/parse-level failure, not a classified application error.
    Transport(String),
}

pub async fn cast_vote(
    client: &HasuraClient,
    election_id: &str,
    ballot_id: &str,
    content: &str,
) -> CastOutcome {
    let variables = insert_cast_vote::Variables {
        election_id: election_id.to_string(),
        ballot_id: ballot_id.to_string(),
        content: content.to_string(),
    };

    let response = match client.send::<InsertCastVote>(variables).await {
        Ok(response) => response,
        Err(err) => return CastOutcome::Transport(err.to_string()),
    };

    if let Some(data) = response.data {
        if let Some(cast_vote) = data.insert_cast_vote {
            return CastOutcome::Success { id: cast_vote.id };
        }
    }

    let Some(errors) = response.errors else {
        return CastOutcome::Transport("GraphQL response had neither data nor errors".to_string());
    };
    let Some(code) = first_error_code(&errors) else {
        return CastOutcome::Rejected {
            code: "Unknown".to_string(),
            message: crate::hasura::format_errors(&errors),
        };
    };

    match code {
        "CheckStatusFailed" if errors[0].message == VOTER_STATE_LOCKED_MESSAGE => {
            CastOutcome::VoterStateLocked
        }
        "CheckRevotesFailed" | "InsertFailedExceedsAllowedRevotes" => {
            CastOutcome::RevoteLimitExceeded
        }
        code => CastOutcome::Rejected {
            code: code.to_string(),
            message: crate::hasura::format_errors(&errors),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphql_client::Error as GqlError;
    use serde_json::json;

    fn error_with(code: &str, message: &str) -> Vec<GqlError> {
        let mut extensions = std::collections::HashMap::new();
        extensions.insert("code".to_string(), json!(code));
        vec![GqlError {
            message: message.to_string(),
            locations: None,
            path: None,
            extensions: Some(extensions),
        }]
    }

    fn classify(errors: &[GqlError]) -> CastOutcome {
        let Some(code) = first_error_code(errors) else {
            return CastOutcome::Rejected {
                code: "Unknown".to_string(),
                message: crate::hasura::format_errors(errors),
            };
        };
        match code {
            "CheckStatusFailed" if errors[0].message == VOTER_STATE_LOCKED_MESSAGE => {
                CastOutcome::VoterStateLocked
            }
            "CheckRevotesFailed" | "InsertFailedExceedsAllowedRevotes" => {
                CastOutcome::RevoteLimitExceeded
            }
            code => CastOutcome::Rejected {
                code: code.to_string(),
                message: crate::hasura::format_errors(errors),
            },
        }
    }

    #[test]
    fn voter_state_locked_is_recognized_by_its_message() {
        let errors = error_with("CheckStatusFailed", VOTER_STATE_LOCKED_MESSAGE);
        assert!(matches!(classify(&errors), CastOutcome::VoterStateLocked));
    }

    #[test]
    fn a_different_check_status_failed_message_is_not_a_lock() {
        let errors = error_with("CheckStatusFailed", "some other status problem");
        assert!(matches!(classify(&errors), CastOutcome::Rejected { .. }));
    }

    #[test]
    fn revote_limit_variants_are_both_recognized() {
        let exceeded = error_with(
            "InsertFailedExceedsAllowedRevotes",
            "InsertFailedExceedsAllowedRevotes",
        );
        assert!(matches!(
            classify(&exceeded),
            CastOutcome::RevoteLimitExceeded
        ));

        let check_failed = error_with("CheckRevotesFailed", "too many revotes");
        assert!(matches!(
            classify(&check_failed),
            CastOutcome::RevoteLimitExceeded
        ));
    }

    #[test]
    fn an_unrecognized_code_is_rejected_with_its_code_and_message() {
        let errors = error_with("AreaNotFound", "Area not found");
        match classify(&errors) {
            CastOutcome::Rejected { code, message } => {
                assert_eq!(code, "AreaNotFound");
                assert_eq!(message, "Area not found");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }
}
