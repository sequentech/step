// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decisions for a vote cast in a Datafix event. The vote is stored
//! in progress and becomes valid or discarded depending on the voter's
//! Keycloak record, their earlier votes and VoterView's reply to `SetVoted`.

use crate::services::external::datafix_types::SoapRequestResponse;
use crate::services::external::utils::{voted_via_internet, voted_via_not_internet_channel};
use crate::services::external::voterview_requests::SoapSendError;
use sequent_core::types::keycloak::User;
use std::collections::HashMap;

/// What the voter's Keycloak record means for their pending vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoterScreening {
    /// The voter is not enabled, or is marked as having voted through
    /// another channel.
    Discard,
    Eligible(InternetChannel),
}

/// Whether Keycloak already marks the voter as having voted via the Internet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternetChannel {
    Marked,
    NotMarked,
}

/// Whether validating the vote also marks the voter as having voted via the
/// Internet. Only the worker whose compare-and-set validated the vote writes
/// the mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternetChannelUpdate {
    Mark,
    Leave,
}

/// What to do with an eligible voter's vote before contacting VoterView.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreSendDecision {
    Validate(InternetChannelUpdate),
    SendSetVoted,
}

/// A `SetVoted` request without a usable reply: the vote stays in progress
/// and the task fails with `error`, so the next review beat retries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetVotedRetry {
    pub audit: String,
    pub error: String,
}

pub fn screen_voter(voter: &User) -> VoterScreening {
    let no_attributes = HashMap::new();
    let attributes = voter.attributes.as_ref().unwrap_or(&no_attributes);
    if voter.enabled != Some(true) || voted_via_not_internet_channel(attributes) {
        VoterScreening::Discard
    } else if voted_via_internet(attributes) {
        VoterScreening::Eligible(InternetChannel::Marked)
    } else {
        VoterScreening::Eligible(InternetChannel::NotMarked)
    }
}

/// Votes of voters already marked via the Internet, and re-votes of voters
/// with a valid vote, are validated without `SetVoted`.
pub fn pre_send_decision(channel: InternetChannel, prior_valid_vote: bool) -> PreSendDecision {
    match (channel, prior_valid_vote) {
        (InternetChannel::Marked, _) => PreSendDecision::Validate(InternetChannelUpdate::Leave),
        (InternetChannel::NotMarked, true) => {
            PreSendDecision::Validate(InternetChannelUpdate::Mark)
        }
        (InternetChannel::NotMarked, false) => PreSendDecision::SendSetVoted,
    }
}

/// Every reply validates the vote. An error reply (a definitive rejection, a
/// SOAP fault, or an unexpected already-not-voted echo) never discards it:
/// the next reconciliation syncs VoterView. Only the replies saying the voter
/// is now marked as voted also mark them in Keycloak.
pub fn set_voted_reply_update(response: &SoapRequestResponse) -> InternetChannelUpdate {
    match response {
        SoapRequestResponse::Ok | SoapRequestResponse::AlreadyVoted => InternetChannelUpdate::Mark,
        SoapRequestResponse::AlreadyNotVoted
        | SoapRequestResponse::Fault(_)
        | SoapRequestResponse::Rejected(_) => InternetChannelUpdate::Leave,
    }
}

/// The electoral log entry for a discarded vote; `changed` is whether this
/// worker's compare-and-set discarded it.
pub fn discard_audit(changed: bool) -> String {
    if changed {
        "SetVoted Skipped: voter is disabled or marked via another channel".to_string()
    } else {
        "SetVoted skip ignored after concurrent resolution".to_string()
    }
}

/// The electoral log entry for a VoterView reply; `changed` is whether this
/// worker's compare-and-set validated the vote. Error replies are logged as
/// failures either way.
pub fn set_voted_audit(
    response: &SoapRequestResponse,
    changed: bool,
    template_sha256: &str,
) -> String {
    match response {
        SoapRequestResponse::Ok if changed => {
            format!("SetVoted Succeeded (template_sha256={template_sha256})")
        }
        SoapRequestResponse::Ok => format!(
            "SetVoted result ignored after concurrent resolution (template_sha256={template_sha256})"
        ),
        SoapRequestResponse::AlreadyVoted if changed => {
            format!("SetVoted Failed: voter already voted (template_sha256={template_sha256})")
        }
        SoapRequestResponse::AlreadyVoted => format!(
            "SetVoted already-voted result ignored after concurrent resolution (template_sha256={template_sha256})"
        ),
        SoapRequestResponse::AlreadyNotVoted
        | SoapRequestResponse::Fault(_)
        | SoapRequestResponse::Rejected(_) => format!(
            "SetVoted Failed: {} (template_sha256={template_sha256})",
            response.classification()
        ),
    }
}

/// A transport failure is treated as transient: the vote is left in progress
/// for the next beat. A persistently failing vote is caught and fixed by the
/// manual daily reconciliation, not by this pipeline.
pub fn set_voted_retry(err: &SoapSendError, template_sha256: &str) -> SetVotedRetry {
    match err {
        SoapSendError::NotDispatched(err) => SetVotedRetry {
            audit: format!(
                "SetVoted NotDispatched: connection-error (template_sha256={template_sha256})"
            ),
            error: format!(
                "VoterView SetVoted was not dispatched; the vote stays in-progress: {err}"
            ),
        },
        SoapSendError::Ambiguous(err) => SetVotedRetry {
            audit: format!(
                "SetVoted Failed: transport-or-response-error (template_sha256={template_sha256})"
            ),
            error: format!(
                "VoterView SetVoted outcome is ambiguous; the vote stays in-progress: {err}"
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::keycloak::VOTED_CHANNEL;

    fn voter(enabled: Option<bool>, voted_channels: &[&str]) -> User {
        let channels = voted_channels.iter().map(|channel| channel.to_string());
        User {
            enabled,
            attributes: (!voted_channels.is_empty())
                .then(|| HashMap::from([(VOTED_CHANNEL.to_string(), channels.collect())])),
            ..Default::default()
        }
    }

    #[test]
    fn voters_who_are_not_enabled_are_discarded_whatever_their_channel() {
        for enabled in [Some(false), None] {
            for channels in [&[][..], &["Internet"]] {
                assert_eq!(
                    screen_voter(&voter(enabled, channels)),
                    VoterScreening::Discard,
                    "enabled={enabled:?} channels={channels:?}"
                );
            }
        }
    }

    #[test]
    fn voters_whose_latest_channel_is_not_the_internet_are_discarded() {
        for channels in [&["Paper"][..], &["Internet", "Kiosk"]] {
            assert_eq!(
                screen_voter(&voter(Some(true), channels)),
                VoterScreening::Discard,
                "{channels:?}"
            );
        }
    }

    #[test]
    fn enabled_voters_are_eligible_and_their_latest_channel_says_if_they_are_marked() {
        let cases: [(&[&str], InternetChannel); 5] = [
            (&[], InternetChannel::NotMarked),
            (&["NONE"], InternetChannel::NotMarked),
            (&[""], InternetChannel::NotMarked),
            (&["Internet"], InternetChannel::Marked),
            (&["Paper", "internet"], InternetChannel::Marked),
        ];
        for (channels, expected) in cases {
            assert_eq!(
                screen_voter(&voter(Some(true), channels)),
                VoterScreening::Eligible(expected),
                "{channels:?}"
            );
        }
    }

    #[test]
    fn voters_marked_via_the_internet_are_validated_without_a_new_mark() {
        for prior_valid_vote in [false, true] {
            assert_eq!(
                pre_send_decision(InternetChannel::Marked, prior_valid_vote),
                PreSendDecision::Validate(InternetChannelUpdate::Leave),
                "prior_valid_vote={prior_valid_vote}"
            );
        }
    }

    #[test]
    fn re_votes_of_unmarked_voters_are_validated_and_marked() {
        assert_eq!(
            pre_send_decision(InternetChannel::NotMarked, true),
            PreSendDecision::Validate(InternetChannelUpdate::Mark)
        );
    }

    #[test]
    fn first_votes_of_unmarked_voters_need_set_voted() {
        assert_eq!(
            pre_send_decision(InternetChannel::NotMarked, false),
            PreSendDecision::SendSetVoted
        );
    }
}
