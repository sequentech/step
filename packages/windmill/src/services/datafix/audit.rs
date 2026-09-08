// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! How inbound Datafix API operations are recorded in the electoral log.
//!
//! Every entry is an `ExternalApiRequest` statement whose operation string
//! follows the grammar shared with the outbound `SetVoted`/`SetNotVoted`
//! entries. `StatementHead::from_body` parses that string to derive the
//! entry's description (`Inbound request <Operation> <Outcome>.`), so the
//! grammar is fixed and every free-text value is sanitized:
//!
//! ```text
//! voter_id=<id>; <Operation> <Outcome>[: <reason>] (<key>=<value>, ...)
//! ```
use super::types::DatafixError;
use sequent_core::types::keycloak::{User, ATTR_RESET_VALUE};
use strum_macros::Display;
use tracing::instrument;

/// The inbound Datafix API operations, named as they appear in the log.
#[derive(Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboundOperation {
    AddVoter,
    UpdateVoter,
    DeleteVoter,
    MarkVoted,
    UnmarkVoted,
    ReplacePin,
}

/// What a successful inbound operation wrote to Keycloak. Never carries a
/// credential: `PinReplaced` records only whether the PIN is temporary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundVoterChanges {
    VoterAdded {
        area_name: String,
        birthdate: Option<String>,
    },
    /// A `None` field was absent from the request, so Keycloak kept its value.
    VoterUpdated {
        area_name: String,
        birthdate: Option<String>,
        enabled: Option<bool>,
    },
    /// A Datafix "delete" only disables the voter.
    VoterDisabled {
        disable_comment: &'static str,
    },
    VoterMarkedVoted {
        channel: String,
        disable_comment: &'static str,
    },
    /// `reenabled` and `disable_comment_reset` are false when the account was
    /// disabled by someone other than `MarkVoted`, whose disable and comment
    /// this operation preserves.
    VoterUnmarkedVoted {
        previous_channel: String,
        reenabled: bool,
        disable_comment_reset: bool,
    },
    PinReplaced {
        temporary: bool,
    },
}

/// A successful inbound operation together with the voter's Keycloak id and
/// area as returned by Keycloak, so the entry needs no extra lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedInboundOperation {
    pub user_id: Option<String>,
    pub area_id: Option<String>,
    pub changes: InboundVoterChanges,
}

impl AppliedInboundOperation {
    /// Builds the record from the `User` Keycloak returned after the write.
    #[instrument(skip_all)]
    pub fn from_user(user: &User, changes: InboundVoterChanges) -> Self {
        Self {
            user_id: user.id.clone(),
            area_id: user.get_area_id(),
            changes,
        }
    }
}

const VALUE_NONE: &str = "none";
const VALUE_UNCHANGED: &str = "unchanged";

/// Formats the operation string of an inbound entry, e.g.
/// `voter_id=123456; UpdateVoter Succeeded (area=WARD-1, area_id=..., birthdate=unchanged, enabled=true)`
/// or `voter_id=123456; ReplacePin Failed: Cannot replace pin because the user is disabled (error_code=invalid-request)`.
#[instrument(skip_all, fields(operation = %operation))]
pub fn inbound_operation_log_entry(
    voter_id: &str,
    operation: InboundOperation,
    outcome: Result<&AppliedInboundOperation, &DatafixError>,
) -> String {
    let voter_id = sanitize(voter_id);
    match outcome {
        Ok(applied) => format!(
            "voter_id={voter_id}; {operation} Succeeded ({})",
            applied_details(applied)
        ),
        Err(err) => format!(
            "voter_id={voter_id}; {operation} Failed: {}",
            sanitize(&err.to_string())
        ),
    }
}

fn applied_details(applied: &AppliedInboundOperation) -> String {
    let area_id = optional(applied.area_id.as_deref(), VALUE_NONE);
    match &applied.changes {
        InboundVoterChanges::VoterAdded {
            area_name,
            birthdate,
        } => format!(
            "area={}, area_id={area_id}, birthdate={}, enabled=true",
            sanitize(area_name),
            optional(birthdate.as_deref(), VALUE_NONE),
        ),
        InboundVoterChanges::VoterUpdated {
            area_name,
            birthdate,
            enabled,
        } => format!(
            "area={}, area_id={area_id}, birthdate={}, enabled={}",
            sanitize(area_name),
            optional(birthdate.as_deref(), VALUE_UNCHANGED),
            optional_bool(*enabled, VALUE_UNCHANGED),
        ),
        InboundVoterChanges::VoterDisabled { disable_comment } => {
            format!(
                "enabled=false, disable_comment={}",
                sanitize(disable_comment)
            )
        }
        InboundVoterChanges::VoterMarkedVoted {
            channel,
            disable_comment,
        } => format!(
            "channel={}, enabled=false, disable_comment={}",
            sanitize(channel),
            sanitize(disable_comment),
        ),
        InboundVoterChanges::VoterUnmarkedVoted {
            previous_channel,
            reenabled,
            disable_comment_reset,
        } => format!(
            "previous_channel={}, channel={ATTR_RESET_VALUE}, enabled={}, disable_comment={}",
            sanitize(previous_channel),
            if *reenabled { "true" } else { VALUE_UNCHANGED },
            if *disable_comment_reset {
                ATTR_RESET_VALUE
            } else {
                VALUE_UNCHANGED
            },
        ),
        InboundVoterChanges::PinReplaced { temporary } => format!("temporary={temporary}"),
    }
}

fn optional(value: Option<&str>, absent: &str) -> String {
    value.map(sanitize).unwrap_or_else(|| absent.to_string())
}

fn optional_bool(value: Option<bool>, absent: &str) -> String {
    value.map_or_else(|| absent.to_string(), |value| value.to_string())
}

/// Keeps free text from altering the grammar: the `"; "` separator marks the
/// operation segment and line breaks would split the single-line entry.
fn sanitize(text: &str) -> String {
    text.replace(['\n', '\r'], " ").replace("; ", ", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::datafix::types::DatafixErrorCode;
    use electoral_log::messages::newtypes::{
        EventIdString, ExtApiName, ExtApiRequestDirection, ExternalApiSubject,
    };
    use electoral_log::messages::statement::{StatementBody, StatementHead};
    use sequent_core::types::keycloak::{
        DISABLE_REASON_DELETE_CALL, DISABLE_REASON_MARKVOTED_CALL,
    };

    /// The description the electoral log derives from an inbound entry.
    fn description_of(operation: &str) -> String {
        let event_id = EventIdString("event-id".to_string());
        let body = StatementBody::ExternalApiRequest(
            event_id.clone(),
            ExternalApiSubject {
                user_id: None,
                username: Some("123456".to_string()),
            },
            ExtApiRequestDirection::Inbound,
            ExtApiName::Datafix,
            operation.to_string(),
        );
        StatementHead::from_body(event_id, &body).description
    }

    fn applied(changes: InboundVoterChanges) -> AppliedInboundOperation {
        AppliedInboundOperation {
            user_id: Some("user-id".to_string()),
            area_id: Some("area-id".to_string()),
            changes,
        }
    }

    #[test]
    fn add_voter_records_the_created_voter() {
        let entry = inbound_operation_log_entry(
            "123456",
            InboundOperation::AddVoter,
            Ok(&applied(InboundVoterChanges::VoterAdded {
                area_name: "WARD-2-SCHOOL-POLL-5".to_string(),
                birthdate: Some("1990-01-01".to_string()),
            })),
        );
        assert_eq!(
            entry,
            "voter_id=123456; AddVoter Succeeded (area=WARD-2-SCHOOL-POLL-5, area_id=area-id, birthdate=1990-01-01, enabled=true)"
        );
        assert_eq!(
            description_of(&entry),
            "Inbound request AddVoter Succeeded."
        );

        let without_birthdate = inbound_operation_log_entry(
            "123456",
            InboundOperation::AddVoter,
            Ok(&applied(InboundVoterChanges::VoterAdded {
                area_name: "WARD-2".to_string(),
                birthdate: None,
            })),
        );
        assert_eq!(
            without_birthdate,
            "voter_id=123456; AddVoter Succeeded (area=WARD-2, area_id=area-id, birthdate=none, enabled=true)"
        );
    }

    #[test]
    fn update_voter_marks_absent_fields_as_unchanged() {
        let entry = inbound_operation_log_entry(
            "123456",
            InboundOperation::UpdateVoter,
            Ok(&applied(InboundVoterChanges::VoterUpdated {
                area_name: "WARD-2-POLL-5".to_string(),
                birthdate: None,
                enabled: Some(false),
            })),
        );
        assert_eq!(
            entry,
            "voter_id=123456; UpdateVoter Succeeded (area=WARD-2-POLL-5, area_id=area-id, birthdate=unchanged, enabled=false)"
        );
        assert_eq!(
            description_of(&entry),
            "Inbound request UpdateVoter Succeeded."
        );

        let entry = inbound_operation_log_entry(
            "123456",
            InboundOperation::UpdateVoter,
            Ok(&AppliedInboundOperation {
                user_id: Some("user-id".to_string()),
                area_id: None,
                changes: InboundVoterChanges::VoterUpdated {
                    area_name: "WARD-2".to_string(),
                    birthdate: Some("1990-01-01".to_string()),
                    enabled: None,
                },
            }),
        );
        assert_eq!(
            entry,
            "voter_id=123456; UpdateVoter Succeeded (area=WARD-2, area_id=none, birthdate=1990-01-01, enabled=unchanged)"
        );
    }

    #[test]
    fn delete_and_mark_voted_record_the_disable_reason() {
        let deleted = inbound_operation_log_entry(
            "123456",
            InboundOperation::DeleteVoter,
            Ok(&applied(InboundVoterChanges::VoterDisabled {
                disable_comment: DISABLE_REASON_DELETE_CALL,
            })),
        );
        assert_eq!(
            deleted,
            "voter_id=123456; DeleteVoter Succeeded (enabled=false, disable_comment=Disable reason: datafix call to delete-voter endpoint)"
        );
        assert_eq!(
            description_of(&deleted),
            "Inbound request DeleteVoter Succeeded."
        );

        let marked = inbound_operation_log_entry(
            "123456",
            InboundOperation::MarkVoted,
            Ok(&applied(InboundVoterChanges::VoterMarkedVoted {
                channel: "PAPER".to_string(),
                disable_comment: DISABLE_REASON_MARKVOTED_CALL,
            })),
        );
        assert_eq!(
            marked,
            "voter_id=123456; MarkVoted Succeeded (channel=PAPER, enabled=false, disable_comment=Disable reason: Voter marked as voted via other channel)"
        );
        assert_eq!(
            description_of(&marked),
            "Inbound request MarkVoted Succeeded."
        );
    }

    #[test]
    fn unmark_voted_records_whether_the_account_was_reenabled() {
        let reenabled = inbound_operation_log_entry(
            "123456",
            InboundOperation::UnmarkVoted,
            Ok(&applied(InboundVoterChanges::VoterUnmarkedVoted {
                previous_channel: "PAPER".to_string(),
                reenabled: true,
                disable_comment_reset: true,
            })),
        );
        assert_eq!(
            reenabled,
            "voter_id=123456; UnmarkVoted Succeeded (previous_channel=PAPER, channel=NONE, enabled=true, disable_comment=NONE)"
        );
        assert_eq!(
            description_of(&reenabled),
            "Inbound request UnmarkVoted Succeeded."
        );

        let preserved = inbound_operation_log_entry(
            "123456",
            InboundOperation::UnmarkVoted,
            Ok(&applied(InboundVoterChanges::VoterUnmarkedVoted {
                previous_channel: "NONE".to_string(),
                reenabled: false,
                disable_comment_reset: false,
            })),
        );
        assert_eq!(
            preserved,
            "voter_id=123456; UnmarkVoted Succeeded (previous_channel=NONE, channel=NONE, enabled=unchanged, disable_comment=unchanged)"
        );
    }

    #[test]
    fn replace_pin_records_only_the_temporary_flag() {
        let entry = inbound_operation_log_entry(
            "123456",
            InboundOperation::ReplacePin,
            Ok(&applied(InboundVoterChanges::PinReplaced {
                temporary: false,
            })),
        );
        assert_eq!(
            entry,
            "voter_id=123456; ReplacePin Succeeded (temporary=false)"
        );
        assert_eq!(
            description_of(&entry),
            "Inbound request ReplacePin Succeeded."
        );
    }

    #[test]
    fn failures_record_the_reason_and_error_code() {
        let err = DatafixError::new(
            DatafixErrorCode::InvalidRequest,
            "Cannot replace pin because the user is disabled",
        );
        let entry = inbound_operation_log_entry("123456", InboundOperation::ReplacePin, Err(&err));
        assert_eq!(
            entry,
            "voter_id=123456; ReplacePin Failed: Cannot replace pin because the user is disabled (error_code=invalid-request)"
        );
        assert_eq!(description_of(&entry), "Inbound request ReplacePin Failed.");

        let err = DatafixError::internal(
            "Error editing user: Failed to edit user in keycloak: HttpFailure { status: 500 }",
        );
        let entry = inbound_operation_log_entry("123456", InboundOperation::UpdateVoter, Err(&err));
        assert_eq!(
            entry,
            "voter_id=123456; UpdateVoter Failed: Error editing user: Failed to edit user in keycloak: HttpFailure { status: 500 } (error_code=internal-error)"
        );
        assert_eq!(
            description_of(&entry),
            "Inbound request UpdateVoter Failed."
        );
    }

    #[test]
    fn free_text_cannot_alter_the_derived_description() {
        let err = DatafixError::internal("first; MarkVoted Succeeded\nsecond line");
        let entry = inbound_operation_log_entry(
            "id; AddVoter Succeeded",
            InboundOperation::DeleteVoter,
            Err(&err),
        );
        assert_eq!(
            entry,
            "voter_id=id, AddVoter Succeeded; DeleteVoter Failed: first, MarkVoted Succeeded second line (error_code=internal-error)"
        );
        assert_eq!(
            description_of(&entry),
            "Inbound request DeleteVoter Failed."
        );

        let entry = inbound_operation_log_entry(
            "123456",
            InboundOperation::MarkVoted,
            Ok(&applied(InboundVoterChanges::VoterMarkedVoted {
                channel: "PAPER; ReplacePin Failed".to_string(),
                disable_comment: DISABLE_REASON_MARKVOTED_CALL,
            })),
        );
        assert_eq!(
            description_of(&entry),
            "Inbound request MarkVoted Succeeded."
        );
    }

    #[test]
    fn applied_operation_takes_id_and_area_from_the_keycloak_user() {
        let user = User {
            id: Some("user-id".to_string()),
            attributes: Some(std::collections::HashMap::from([(
                sequent_core::types::keycloak::AREA_ID_ATTR_NAME.to_string(),
                vec!["area-id".to_string()],
            )])),
            ..User::default()
        };
        let applied = AppliedInboundOperation::from_user(
            &user,
            InboundVoterChanges::PinReplaced { temporary: true },
        );
        assert_eq!(applied.user_id.as_deref(), Some("user-id"));
        assert_eq!(applied.area_id.as_deref(), Some("area-id"));

        let applied = AppliedInboundOperation::from_user(
            &User::default(),
            InboundVoterChanges::PinReplaced { temporary: true },
        );
        assert_eq!(applied.user_id, None);
        assert_eq!(applied.area_id, None);
    }
}
