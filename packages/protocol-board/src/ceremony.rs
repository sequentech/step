// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a keys ceremony becomes when the board says something.
//!
//! Every rule the ceremony has lives in [`transition`]; it doesn't read nor
//! write anything, just takes input state and return the new state. The calling
//! task should load the ceremony, call this, and save what comes back.
//!
//! Two properties hold:
//!
//! - **Terminal states absorb.** `SUCCESS`, `FAILED` and `CANCELLED` ignore
//!   every event. A failure is never worked around: the administrator
//!   creates a new ceremony, which gets a new board.
//! - **Applying the same event twice changes nothing the second time.**
//!   Tasks are retried and boards are polled repeatedly, so a rule that fired
//!   twice would double a log line or reopen a decision.
//!
//! A trustee's phase only ever moves forward. The board reports the protocol
//! phases (e.g., waiting, shares posted, key generated) while the custody steps
//! that follow are reported by the trustees themselves, and a board poll
//! arriving afterwards must not pull them back.

use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyFailure,
    KeysCeremonyFailureReason, KeysCeremonyStatus, Log, Trustee, TrusteeStatus,
};

use crate::committee::Committee;
use crate::ids::BoardName;
use crate::view::{DkgStatus, DkgView};

/// The current time, in forms the ceremony records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamps {
    /// ISO 8601, as the ceremony logs and the failure record carry it.
    pub iso8601: String,
    /// Milliseconds since the Unix epoch, as `stop_date` carries it.
    pub unix_ms: String,
}

/// A keys ceremony's whole mutable state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeremonyState {
    pub execution: KeysCeremonyExecutionStatus,
    pub status: KeysCeremonyStatus,
    _private: (),
}
impl CeremonyState {
    pub fn new(
        execution: KeysCeremonyExecutionStatus,
        status: KeysCeremonyStatus,
    ) -> Self {
        Self {
            execution,
            status,
            _private: (),
        }
    }
}

/// A new ceremony's trustees, all waiting, in `Configuration` order.
pub fn initial_trustees(committee: &Committee) -> Vec<Trustee> {
    // Board readings are matched to the ceremony's trustees by position, so this
    // order is what ties [`DkgView::phases`] to the trustees it describes.
    committee
        .members()
        .iter()
        .map(|member| Trustee {
            name: member.name.clone(),
            status: TrusteeStatus::WAITING,
        })
        .collect()
}

/// Something the platform did out about a ceremony's board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    /// The ceremony's Configuration was posted on the board.
    Published { board: BoardName },
    /// The board was read.
    ReadBoard(DkgView),
}

/// Apply one event to a ceremony.
pub fn transition(
    state: &CeremonyState,
    policy: CeremoniesPolicy,
    event: PlatformEvent,
    now: &Timestamps,
) -> CeremonyState {
    let mut next = state.clone();

    match state.execution {
        // Before the ceremony starts there is no board to observe, and once it
        // has ended nothing reopens it.
        KeysCeremonyExecutionStatus::USER_CONFIGURATION
        | KeysCeremonyExecutionStatus::SUCCESS
        | KeysCeremonyExecutionStatus::FAILED
        | KeysCeremonyExecutionStatus::CANCELLED => {}

        KeysCeremonyExecutionStatus::STARTED => match event {
            PlatformEvent::Published { board } => {
                next.execution = KeysCeremonyExecutionStatus::IN_PROGRESS;
                log(
                    &mut next.status,
                    now,
                    format!(
                        "Posted the key generation configuration on board \
                         {board}"
                    ),
                );
            }
            // Nothing has been published, so a reading of a board says nothing
            // about this ceremony.
            PlatformEvent::ReadBoard(_) => {}
        },

        KeysCeremonyExecutionStatus::IN_PROGRESS => match event {
            // Publishing happens once, before the ceremony is in progress, no-op.
            PlatformEvent::Published { .. } => {}
            PlatformEvent::ReadBoard(view) => {
                // Advance the trustees per their currently viewed statuses.
                for (trustee, observed) in
                    next.status.trustees.iter_mut().zip(&view.trustee_statuses)
                {
                    if progress(observed) > progress(&trustee.status) {
                        trustee.status = observed.clone();
                        next.status.logs.push(Log {
                            created_date: now.iso8601.clone(),
                            log_text: format!(
                                "Trustee {} is now {observed}",
                                trustee.name
                            ),
                        });
                    }
                }

                match view.status {
                    DkgStatus::InProgress => {}
                    DkgStatus::Completed {
                        joint_public_key,
                        public_key_hash,
                    } => {
                        if next.status.public_key.is_none() {
                            next.status.public_key = Some(joint_public_key);
                            next.status.public_key_hash =
                                Some(public_key_hash.as_str().to_string());
                            log(
                                &mut next.status,
                                now,
                                "The trustees generated the joint public key"
                                    .to_string(),
                            );
                        }
                        // Under the manual policy the ceremony stays in
                        // progress until every trustee has taken custody of
                        // its key; the automated policy has no such step.
                        if policy == CeremoniesPolicy::AUTOMATED_CEREMONIES {
                            next.execution =
                                KeysCeremonyExecutionStatus::SUCCESS;
                            next.status.stop_date = Some(now.unix_ms.clone());
                        }
                    }
                    DkgStatus::Unusable { reason, detail } => {
                        fail(&mut next, reason, detail, now)
                    }
                }
            }
        },
    }

    next
}

fn log(status: &mut KeysCeremonyStatus, now: &Timestamps, log_text: String) {
    status.logs.push(Log {
        created_date: now.iso8601.clone(),
        log_text,
    });
}

fn fail(
    next: &mut CeremonyState,
    reason: KeysCeremonyFailureReason,
    detail: String,
    now: &Timestamps,
) {
    next.execution = KeysCeremonyExecutionStatus::FAILED;
    next.status.stop_date = Some(now.unix_ms.clone());
    log(
        &mut next.status,
        now,
        format!("Keys ceremony failed ({reason}): {detail}"),
    );
    next.status.failure = Some(KeysCeremonyFailure {
        reason,
        detail,
        failed_at: now.iso8601.clone(),
    });
}

/// How far along a trustee status is. The board only ever reports the first
/// three; the custody steps come from the trustees.
fn progress(status: &TrusteeStatus) -> u8 {
    match status {
        TrusteeStatus::WAITING => 0,
        TrusteeStatus::SHARES_POSTED => 1,
        TrusteeStatus::KEY_GENERATED => 2,
        TrusteeStatus::KEY_RETRIEVED => 3,
        TrusteeStatus::KEY_CHECKED => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::HashHex;
    use uuid::Uuid;
    use wbraid::messages::newtypes::hash_bytes;

    const ISO: &str = "2026-09-22T10:00:00Z";
    const UNIX_MS: &str = "1790064000000";

    fn now() -> Timestamps {
        Timestamps {
            iso8601: ISO.to_string(),
            unix_ms: UNIX_MS.to_string(),
        }
    }

    fn board() -> BoardName {
        BoardName::for_dkg(&Uuid::nil())
    }

    fn published() -> PlatformEvent {
        PlatformEvent::Published { board: board() }
    }

    fn key_hash() -> HashHex {
        HashHex::of(&hash_bytes(b"the joint key"))
    }

    fn started() -> CeremonyState {
        CeremonyState::new(
            KeysCeremonyExecutionStatus::STARTED,
            KeysCeremonyStatus {
                stop_date: None,
                public_key: None,
                public_key_hash: None,
                failure: None,
                logs: vec![],
                trustees: ["trustee1", "trustee2"]
                    .into_iter()
                    .map(|name| Trustee {
                        name: name.to_string(),
                        status: TrusteeStatus::WAITING,
                    })
                    .collect(),
            },
        )
    }

    fn in_progress() -> CeremonyState {
        let mut state = started();
        state.execution = KeysCeremonyExecutionStatus::IN_PROGRESS;
        state
    }

    fn reading(phases: [TrusteeStatus; 2], status: DkgStatus) -> PlatformEvent {
        PlatformEvent::ReadBoard(DkgView::new_unchecked(&phases, status))
    }

    fn dealing() -> PlatformEvent {
        reading(
            [TrusteeStatus::SHARES_POSTED, TrusteeStatus::WAITING],
            DkgStatus::InProgress,
        )
    }

    fn completed() -> PlatformEvent {
        reading(
            [TrusteeStatus::KEY_GENERATED, TrusteeStatus::KEY_GENERATED],
            DkgStatus::Completed {
                joint_public_key: "am9pbnQta2V5".to_string(),
                public_key_hash: key_hash(),
            },
        )
    }

    fn unusable(reason: KeysCeremonyFailureReason) -> PlatformEvent {
        reading(
            [TrusteeStatus::KEY_GENERATED, TrusteeStatus::SHARES_POSTED],
            DkgStatus::Unusable {
                reason,
                detail: "what the board said".to_string(),
            },
        )
    }

    fn every_event() -> Vec<PlatformEvent> {
        vec![
            published(),
            dealing(),
            completed(),
            unusable(KeysCeremonyFailureReason::BOARD_CONFIGURATION_MISMATCH),
            unusable(KeysCeremonyFailureReason::INVALID_BOARD_CONTENT),
        ]
    }

    const POLICIES: [CeremoniesPolicy; 2] = [
        CeremoniesPolicy::MANUAL_CEREMONIES,
        CeremoniesPolicy::AUTOMATED_CEREMONIES,
    ];

    fn apply(
        state: &CeremonyState,
        policy: CeremoniesPolicy,
        event: PlatformEvent,
    ) -> CeremonyState {
        transition(state, policy, event, &now())
    }

    #[test]
    fn publishing_the_configuration_starts_the_protocol() {
        let next =
            apply(&started(), CeremoniesPolicy::MANUAL_CEREMONIES, published());

        assert_eq!(next.execution, KeysCeremonyExecutionStatus::IN_PROGRESS);
        assert_eq!(next.status.logs.len(), 1);
        assert_eq!(next.status.logs[0].created_date, ISO);
        assert!(next.status.logs[0].log_text.contains(board().as_str()));
        assert!(next.status.stop_date.is_none());
    }

    #[test]
    fn a_board_reading_before_the_configuration_is_posted_changes_nothing() {
        for event in [dealing(), completed()] {
            assert_eq!(
                apply(
                    &started(),
                    CeremoniesPolicy::AUTOMATED_CEREMONIES,
                    event
                ),
                started()
            );
        }
    }

    #[test]
    fn trustee_phases_follow_the_board_by_position_and_are_logged_once_each() {
        let first = apply(
            &in_progress(),
            CeremoniesPolicy::MANUAL_CEREMONIES,
            dealing(),
        );
        assert_eq!(
            first.status.trustees[0].status,
            TrusteeStatus::SHARES_POSTED
        );
        assert_eq!(first.status.trustees[1].status, TrusteeStatus::WAITING);
        assert_eq!(first.status.logs.len(), 1);
        assert!(first.status.logs[0].log_text.contains("trustee1"));

        let second = apply(
            &first,
            CeremoniesPolicy::MANUAL_CEREMONIES,
            reading(
                [TrusteeStatus::SHARES_POSTED, TrusteeStatus::SHARES_POSTED],
                DkgStatus::InProgress,
            ),
        );
        assert_eq!(second.status.logs.len(), 2);
        assert!(second.status.logs[1].log_text.contains("trustee2"));
    }

    /// The phases a trustee passes through, in the order it passes them.
    ///
    /// Written out here rather than derived from [`progress`] so that the two
    /// are independent statements of the same order.
    const PHASES_IN_ORDER: [TrusteeStatus; 5] = [
        TrusteeStatus::WAITING,
        TrusteeStatus::SHARES_POSTED,
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_CHECKED,
    ];

    #[test]
    fn a_board_reading_only_ever_moves_a_trustee_forward() {
        // The custody steps are reported by the trustees, not by the board, so
        // a later poll still showing KEY_GENERATED must leave them alone.
        for (position, current) in PHASES_IN_ORDER.iter().enumerate() {
            for (other, observed) in PHASES_IN_ORDER.iter().enumerate() {
                let mut state = in_progress();
                state.status.trustees[0].status = current.clone();
                let next = apply(
                    &state,
                    CeremoniesPolicy::MANUAL_CEREMONIES,
                    reading(
                        [observed.clone(), TrusteeStatus::WAITING],
                        DkgStatus::InProgress,
                    ),
                );

                let later = PHASES_IN_ORDER[position.max(other)].clone();
                assert_eq!(
                    next.status.trustees[0].status, later,
                    "{current} then {observed}"
                );
                assert_eq!(
                    next.status.logs.len(),
                    usize::from(other > position),
                    "{current} then {observed}"
                );
            }
        }
    }

    #[test]
    fn an_automated_ceremony_succeeds_as_soon_as_the_key_exists() {
        let next = apply(
            &in_progress(),
            CeremoniesPolicy::AUTOMATED_CEREMONIES,
            completed(),
        );

        assert_eq!(next.execution, KeysCeremonyExecutionStatus::SUCCESS);
        assert_eq!(next.status.public_key, Some("am9pbnQta2V5".to_string()));
        assert_eq!(
            next.status.public_key_hash,
            Some(key_hash().as_str().to_string())
        );
        assert_eq!(next.status.stop_date, Some(UNIX_MS.to_string()));
    }

    #[test]
    fn a_manual_ceremony_keeps_the_key_but_waits_for_the_custody_steps() {
        let next = apply(
            &in_progress(),
            CeremoniesPolicy::MANUAL_CEREMONIES,
            completed(),
        );

        assert_eq!(next.execution, KeysCeremonyExecutionStatus::IN_PROGRESS);
        assert_eq!(next.status.public_key, Some("am9pbnQta2V5".to_string()));
        assert!(next.status.stop_date.is_none());
    }

    #[test]
    fn an_unusable_board_ends_the_ceremony_with_its_reason() {
        for reason in [
            KeysCeremonyFailureReason::BOARD_CONFIGURATION_MISMATCH,
            KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
        ] {
            for policy in POLICIES {
                let next = apply(&in_progress(), policy, unusable(reason));

                assert_eq!(
                    next.execution,
                    KeysCeremonyExecutionStatus::FAILED,
                    "{reason}"
                );
                assert_eq!(
                    next.status.failure,
                    Some(KeysCeremonyFailure {
                        reason,
                        detail: "what the board said".to_string(),
                        failed_at: ISO.to_string(),
                    })
                );
                assert_eq!(next.status.stop_date, Some(UNIX_MS.to_string()));
                assert_eq!(next.status.public_key, None);
                // The phases observed on the way are still recorded.
                assert_eq!(
                    next.status.trustees[1].status,
                    TrusteeStatus::SHARES_POSTED
                );
                assert!(next
                    .status
                    .logs
                    .last()
                    .is_some_and(|line| line.log_text.contains("failed")));
            }
        }
    }

    #[test]
    fn terminal_states_absorb_every_observation() {
        for execution in [
            KeysCeremonyExecutionStatus::USER_CONFIGURATION,
            KeysCeremonyExecutionStatus::SUCCESS,
            KeysCeremonyExecutionStatus::FAILED,
            KeysCeremonyExecutionStatus::CANCELLED,
        ] {
            let mut state = in_progress();
            state.execution = execution.clone();
            for policy in POLICIES {
                for event in every_event() {
                    assert_eq!(
                        apply(&state, policy.clone(), event),
                        state,
                        "{execution}"
                    );
                }
            }
        }
    }

    #[test]
    fn applying_the_same_event_twice_changes_nothing_the_second_time() {
        for start in [started(), in_progress()] {
            for policy in POLICIES {
                for event in every_event() {
                    let once = apply(&start, policy.clone(), event.clone());
                    let twice = apply(&once, policy.clone(), event.clone());
                    assert_eq!(
                        twice, once,
                        "{event:?} on {:?}",
                        start.execution
                    );
                }
            }
        }
    }

    #[test]
    fn the_log_gains_one_line_per_thing_that_happened() {
        let mut state = started();
        for (event, lines) in [
            (published(), 1),
            (dealing(), 2),
            // Nothing new on the board.
            (dealing(), 2),
            // Two trustees move and the key is taken.
            (completed(), 5),
        ] {
            state = apply(
                &state,
                CeremoniesPolicy::AUTOMATED_CEREMONIES,
                event.clone(),
            );
            assert_eq!(state.status.logs.len(), lines, "{event:?}");
            assert!(state
                .status
                .logs
                .iter()
                .all(|line| line.created_date == ISO));
        }
        assert_eq!(state.execution, KeysCeremonyExecutionStatus::SUCCESS);
    }
}
