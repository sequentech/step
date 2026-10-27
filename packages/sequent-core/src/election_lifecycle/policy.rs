// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Pure election lifecycle policy shared by server checks and their tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "default_features",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "default_features", serde(rename_all = "snake_case"))]
pub enum ElectoralPhase {
    Preparation,
    Execution,
    Evaluation,
    PostProcessing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "default_features",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "default_features", serde(rename_all = "snake_case"))]
pub enum SafetyState {
    Operational,
    Recoverable,
    Unrecoverable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "default_features",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct LifecycleState {
    pub phase: ElectoralPhase,
    pub safety: SafetyState,
    pub generation: i64,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleRefusal {
    WrongPhase,
    Unrecoverable,
    OutsideElectionPeriod,
    EmptyBoxCheck,
    SelfTestRequired,
    ServiceUnavailable,
    Maintenance,
    ResultNotDetermined,
}
impl std::fmt::Display for LifecycleRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for LifecycleRefusal {}

pub fn opening_allowed(
    state: &LifecycleState,
    now: i64,
    start: i64,
    end: i64,
    test_passed: bool,
    boxes_empty: bool,
    services_live: bool,
) -> Result<(), LifecycleRefusal> {
    if state.safety == SafetyState::Unrecoverable {
        return Err(LifecycleRefusal::Unrecoverable);
    }
    if !matches!(
        state.phase,
        ElectoralPhase::Preparation | ElectoralPhase::Execution
    ) {
        return Err(LifecycleRefusal::WrongPhase);
    }
    if start >= end || now < start || now >= end {
        return Err(LifecycleRefusal::OutsideElectionPeriod);
    }
    if !services_live {
        return Err(LifecycleRefusal::ServiceUnavailable);
    }
    if state.phase == ElectoralPhase::Preparation && !boxes_empty {
        return Err(LifecycleRefusal::EmptyBoxCheck);
    }
    if !test_passed {
        return Err(LifecycleRefusal::SelfTestRequired);
    }
    Ok(())
}
pub fn voter_allowed(
    state: &LifecycleState,
    test_current: bool,
    services_live: bool,
) -> Result<(), LifecycleRefusal> {
    if state.phase != ElectoralPhase::Execution {
        return Err(LifecycleRefusal::WrongPhase);
    }
    if state.safety != SafetyState::Operational {
        return Err(LifecycleRefusal::Maintenance);
    }
    if !services_live {
        return Err(LifecycleRefusal::ServiceUnavailable);
    }
    if !test_current {
        return Err(LifecycleRefusal::SelfTestRequired);
    }
    Ok(())
}
pub fn post_processing_allowed(
    state: &LifecycleState,
    result_determined: bool,
) -> Result<(), LifecycleRefusal> {
    if state.phase != ElectoralPhase::Evaluation {
        return Err(LifecycleRefusal::WrongPhase);
    }
    if !result_determined && state.safety != SafetyState::Unrecoverable {
        return Err(LifecycleRefusal::ResultNotDetermined);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(phase: ElectoralPhase, safety: SafetyState) -> LifecycleState {
        LifecycleState {
            phase,
            safety,
            generation: 2,
            reason: None,
        }
    }
    #[test]
    fn start_never_ignores_signed_election_period() {
        let s = state(ElectoralPhase::Preparation, SafetyState::Operational);
        for now in [99, 200, 201] {
            assert_eq!(
                opening_allowed(&s, now, 100, 200, true, true, true),
                Err(LifecycleRefusal::OutsideElectionPeriod)
            );
        }
        assert!(opening_allowed(&s, 100, 100, 200, true, true, true).is_ok());
    }
    #[test]
    fn first_open_checks_empty_boxes_and_suite() {
        let s = state(ElectoralPhase::Preparation, SafetyState::Recoverable);
        assert_eq!(
            opening_allowed(&s, 101, 100, 200, true, false, true),
            Err(LifecycleRefusal::EmptyBoxCheck)
        );
        assert_eq!(
            opening_allowed(&s, 101, 100, 200, false, true, true),
            Err(LifecycleRefusal::SelfTestRequired)
        );
    }
    #[test]
    fn recovery_never_reopens_terminated_or_unrecoverable_execution() {
        for phase in [
            ElectoralPhase::Preparation,
            ElectoralPhase::Execution,
            ElectoralPhase::Evaluation,
            ElectoralPhase::PostProcessing,
        ] {
            assert!(opening_allowed(
                &state(phase, SafetyState::Unrecoverable),
                101,
                100,
                200,
                true,
                true,
                true
            )
            .is_err());
        }
        for phase in
            [ElectoralPhase::Evaluation, ElectoralPhase::PostProcessing]
        {
            assert_eq!(
                opening_allowed(
                    &state(phase, SafetyState::Operational),
                    101,
                    100,
                    200,
                    true,
                    true,
                    true
                ),
                Err(LifecycleRefusal::WrongPhase)
            );
        }
    }
    #[test]
    fn recoverable_execution_needs_fresh_suite_but_not_empty_boxes() {
        let s = state(ElectoralPhase::Execution, SafetyState::Recoverable);
        assert!(opening_allowed(&s, 101, 100, 200, true, false, true).is_ok());
        assert_eq!(
            opening_allowed(&s, 101, 100, 200, false, false, true),
            Err(LifecycleRefusal::SelfTestRequired)
        );
    }
    #[test]
    fn dead_service_blocks_opening_and_existing_voters() {
        let s = state(ElectoralPhase::Execution, SafetyState::Operational);
        assert_eq!(
            opening_allowed(&s, 101, 100, 200, true, false, false),
            Err(LifecycleRefusal::ServiceUnavailable)
        );
        assert_eq!(
            voter_allowed(&s, true, false),
            Err(LifecycleRefusal::ServiceUnavailable)
        );
    }
    #[test]
    fn only_operational_execution_with_current_suite_admits_voters() {
        for phase in [
            ElectoralPhase::Preparation,
            ElectoralPhase::Execution,
            ElectoralPhase::Evaluation,
            ElectoralPhase::PostProcessing,
        ] {
            for safety in [
                SafetyState::Operational,
                SafetyState::Recoverable,
                SafetyState::Unrecoverable,
            ] {
                assert_eq!(
                    voter_allowed(&state(phase, safety), true, true).is_ok(),
                    phase == ElectoralPhase::Execution
                        && safety == SafetyState::Operational
                );
            }
        }
        assert_eq!(
            voter_allowed(
                &state(ElectoralPhase::Execution, SafetyState::Operational),
                false,
                true
            ),
            Err(LifecycleRefusal::SelfTestRequired)
        );
    }
    #[test]
    fn an_unrecoverable_event_can_be_exported_for_investigation_after_board_termination(
    ) {
        assert!(post_processing_allowed(
            &state(ElectoralPhase::Evaluation, SafetyState::Unrecoverable),
            false
        )
        .is_ok());
        assert_eq!(
            post_processing_allowed(
                &state(ElectoralPhase::Execution, SafetyState::Unrecoverable),
                false
            ),
            Err(LifecycleRefusal::WrongPhase)
        );
    }
    #[test]
    fn postprocessing_requires_completed_counting_and_cannot_reset_phase() {
        assert_eq!(
            post_processing_allowed(
                &state(ElectoralPhase::Evaluation, SafetyState::Operational),
                false
            ),
            Err(LifecycleRefusal::ResultNotDetermined)
        );
        assert!(post_processing_allowed(
            &state(ElectoralPhase::Evaluation, SafetyState::Operational),
            true
        )
        .is_ok());
        for phase in [
            ElectoralPhase::Preparation,
            ElectoralPhase::Execution,
            ElectoralPhase::PostProcessing,
        ] {
            assert_eq!(
                post_processing_allowed(
                    &state(phase, SafetyState::Operational),
                    true
                ),
                Err(LifecycleRefusal::WrongPhase)
            );
        }
    }
}
