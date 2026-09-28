// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a tally session becomes when its boards, or a trustee, say something.
//!
//! Every rule the session has lives in [`transition`]; it doesn't read nor
//! write anything, just takes input state and returns the new state. The
//! calling task should load the session under its row lock, call this, and
//! save what comes back.
//!
//! The properties are the keys ceremony's:
//!
//! - **Terminal states absorb.** `FAILED` and `CANCELLED` ignore every event,
//!   and `SUCCESS` every event but a trustee's halt. A failure is never worked
//!   around: the administrator creates a new tally session, which gets new
//!   boards.
//! - **Applying the same event twice changes nothing the second time.** Tasks
//!   are retried, boards are polled repeatedly and trustees repeat their
//!   reports.
//! - **A halt beats success.** A trustee halts on its own view of a board,
//!   which the platform may have read as complete an instant earlier; what
//!   tells the two apart is a rewrite, a withholding or an equivocation, which
//!   is what the halt exists to catch.
//!
//! A board's phase only ever moves forward. Completing the session is not a
//! transition: once every board is decrypted, the task decodes the payloads,
//! runs the results and records `SUCCESS` itself.

use std::collections::BTreeSet;

use sequent_core::types::ceremonies::{
    Log, TallyBoardPhase, TallyBoardStatus, TallyCeremonyStatus, TallyElection,
    TallyElectionStatus, TallyExecutionStatus, TallyFailure,
    TallyFailureReason, TallyTrusteeStatus,
};
use tracing::warn;

use crate::ceremony::Timestamps;
use crate::ids::BoardName;

/// What a mixing board adds to its election's progress, out of one.
const MIXING_PROGRESS: f64 = 0.2;
/// What a decrypting board adds to its election's progress, out of one.
const DECRYPTING_PROGRESS: f64 = 0.4;

/// A tally session's whole mutable state.
#[derive(Debug, Clone, PartialEq)]
pub struct TallySessionState {
    pub execution: TallyExecutionStatus,
    pub status: TallyCeremonyStatus,
    _private: (),
}

impl TallySessionState {
    pub fn new(
        execution: TallyExecutionStatus,
        status: TallyCeremonyStatus,
    ) -> Self {
        Self {
            execution,
            status,
            _private: (),
        }
    }
}

/// One tally board of a session, as it is created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyBoardRef {
    pub name: BoardName,
    pub election_id: String,
    pub area_id: String,
    /// `None` when the board carries whole ballots rather than one contest.
    pub contest_id: Option<String>,
    pub batch: i64,
}

/// One reading of a tally board, as the session records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TallyBoardReading {
    Phase(TallyBoardPhase),
    /// Nothing will ever be decrypted from this board.
    Unusable {
        reason: TallyFailureReason,
        detail: String,
    },
}

/// Something the platform did or learnt about a tally session's boards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TallyEvent {
    /// The session's boards were created, once, when its ballots were
    /// extracted.
    BoardsCreated(Vec<TallyBoardRef>),
    /// The session's boards were read.
    ReadBoards(Vec<(BoardName, TallyBoardReading)>),
    /// A trustee reported that it halted its session over one of the boards.
    TrusteeHalted {
        /// The trustee's name, as the session lists it.
        trustee: String,
        /// braid's error, as the trustee reported it.
        detail: String,
    },
}

/// Apply one event to a tally session.
pub fn transition(
    state: &TallySessionState,
    event: TallyEvent,
    now: &Timestamps,
) -> TallySessionState {
    let mut next = state.clone();

    match state.execution {
        // No board exists before the session is in progress, and once it has
        // failed or been cancelled nothing reopens it.
        TallyExecutionStatus::STARTED
        | TallyExecutionStatus::CONNECTED
        | TallyExecutionStatus::FAILED
        | TallyExecutionStatus::CANCELLED => {}

        // The boards are done; a re-run reads them without changing the
        // status.
        TallyExecutionStatus::AWAITING_INPUT
        | TallyExecutionStatus::SUCCESS => match event {
            TallyEvent::TrusteeHalted { trustee, detail } => {
                halt(&mut next, &trustee, detail, now)
            }
            TallyEvent::BoardsCreated(_) | TallyEvent::ReadBoards(_) => {}
        },

        TallyExecutionStatus::IN_PROGRESS => match event {
            TallyEvent::BoardsCreated(boards) => {
                record_boards(&mut next, boards, now)
            }
            TallyEvent::ReadBoards(readings) => {
                read_boards(&mut next, readings, now)
            }
            TallyEvent::TrusteeHalted { trustee, detail } => {
                halt(&mut next, &trustee, detail, now)
            }
        },
    }

    next
}

/// The session's boards, every one waiting for its `Ballots`. Recorded once.
fn record_boards(
    next: &mut TallySessionState,
    boards: Vec<TallyBoardRef>,
    now: &Timestamps,
) {
    if next.status.boards.is_some() {
        return;
    }
    let count = boards.len();
    let boards = boards
        .into_iter()
        .map(|board| TallyBoardStatus {
            name: board.name.to_string(),
            election_id: board.election_id,
            area_id: board.area_id,
            contest_id: board.contest_id,
            batch: board.batch,
            phase: TallyBoardPhase::BALLOTS_PENDING,
        })
        .collect::<Vec<_>>();
    next.status.elections_status =
        elections_status(&next.status.elections_status, &boards);
    next.status.boards = Some(boards);
    log(
        &mut next.status,
        now,
        format!("The ballots were extracted into {count} tally boards"),
    );
}

/// Advance each board the reading names to its phase, or fail the session on
/// the first board that cannot be used.
fn read_boards(
    next: &mut TallySessionState,
    readings: Vec<(BoardName, TallyBoardReading)>,
    now: &Timestamps,
) {
    let listed = next.status.boards.as_deref().unwrap_or_default();
    let mut advances = Vec::new();
    let mut unusable = None;
    for (board, reading) in readings {
        let Some(position) = listed
            .iter()
            .position(|status| status.name == board.as_str())
        else {
            warn!(
                %board,
                "a reading names a board the tally session does not list"
            );
            continue;
        };
        match reading {
            TallyBoardReading::Phase(phase) => advances.push((position, phase)),
            TallyBoardReading::Unusable { reason, detail } => {
                if unusable.is_none() {
                    unusable = Some((board, reason, detail));
                }
            }
        }
    }

    if let Some((board, reason, detail)) = unusable {
        fail(next, reason, format!("board {board}: {detail}"), now);
        return;
    }

    let Some(boards) = next.status.boards.as_mut() else {
        return;
    };
    let mut changes = Vec::new();
    for (position, observed) in advances {
        let Some(status) = boards.get_mut(position) else {
            continue;
        };
        if rank(observed) > rank(status.phase) {
            status.phase = observed;
            changes.push(format!("Board {} is now {observed}", status.name));
        }
    }
    let elections = elections_status(&next.status.elections_status, boards);
    next.status.elections_status = elections;
    for change in changes {
        log(&mut next.status, now, change);
    }
}

/// A trustee halted: it is marked halted and the session fails. A trustee the
/// session does not list changes nothing.
fn halt(
    next: &mut TallySessionState,
    trustee: &str,
    detail: String,
    now: &Timestamps,
) {
    let Some(halted) = next
        .status
        .trustees
        .iter_mut()
        .find(|listed| listed.name == trustee)
    else {
        return;
    };
    halted.status = TallyTrusteeStatus::HALTED;
    log(
        &mut next.status,
        now,
        format!("Trustee {trustee} is now {}", TallyTrusteeStatus::HALTED),
    );
    fail(
        next,
        TallyFailureReason::TRUSTEE_HALTED,
        format!("{trustee}: {detail}"),
        now,
    );
}

fn fail(
    next: &mut TallySessionState,
    reason: TallyFailureReason,
    detail: String,
    now: &Timestamps,
) {
    next.execution = TallyExecutionStatus::FAILED;
    next.status.stop_date = Some(now.unix_ms.clone());
    log(
        &mut next.status,
        now,
        format!("Tally failed ({reason}): {detail}"),
    );
    next.status.failure = Some(TallyFailure {
        reason,
        detail,
        failed_at: now.iso8601.clone(),
    });
}

fn log(status: &mut TallyCeremonyStatus, now: &Timestamps, log_text: String) {
    status.logs.push(Log {
        created_date: now.iso8601.clone(),
        log_text,
    });
}

/// How far along a board phase is.
fn rank(phase: TallyBoardPhase) -> u8 {
    match phase {
        TallyBoardPhase::BALLOTS_PENDING => 0,
        TallyBoardPhase::BALLOTS_POSTED => 1,
        TallyBoardPhase::MIXING => 2,
        TallyBoardPhase::DECRYPTING => 3,
        TallyBoardPhase::DECRYPTED => 4,
    }
}

/// Every election's status and progress, over its boards, ordered by election.
/// The elections are the ones already listed and the ones the boards name.
fn elections_status(
    listed: &[TallyElection],
    boards: &[TallyBoardStatus],
) -> Vec<TallyElection> {
    listed
        .iter()
        .map(|election| election.election_id.as_str())
        .chain(boards.iter().map(|board| board.election_id.as_str()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|election_id| {
            let phases = boards
                .iter()
                .filter(|board| board.election_id == election_id)
                .map(|board| board.phase)
                .collect::<Vec<_>>();
            election_status(election_id, &phases)
        })
        .collect()
}

/// An election with no board has nothing to decrypt: it is done.
fn election_status(
    election_id: &str,
    phases: &[TallyBoardPhase],
) -> TallyElection {
    let count = |wanted: TallyBoardPhase| {
        phases.iter().filter(|phase| **phase == wanted).count()
    };
    let decrypted = count(TallyBoardPhase::DECRYPTED);
    let decrypting = count(TallyBoardPhase::DECRYPTING);
    let mixing = count(TallyBoardPhase::MIXING);

    let status = if decrypted == phases.len() {
        TallyElectionStatus::SUCCESS
    } else if decrypting > 0 {
        TallyElectionStatus::DECRYPTING
    } else if mixing > 0 {
        TallyElectionStatus::MIXING
    } else {
        TallyElectionStatus::WAITING
    };
    let progress = if phases.is_empty() {
        100.0
    } else {
        (100.0
            * (MIXING_PROGRESS * mixing as f64
                + DECRYPTING_PROGRESS * decrypting as f64
                + decrypted as f64)
            / phases.len() as f64)
            .clamp(0.0, 100.0)
    };
    TallyElection {
        election_id: election_id.to_string(),
        status,
        progress,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::ceremonies::TallyTrustee;
    use uuid::Uuid;

    const ISO: &str = "2026-09-29T10:00:00Z";
    const UNIX_MS: &str = "1790676000000";
    const LATER_ISO: &str = "2026-09-29T10:05:00Z";
    const LATER_UNIX_MS: &str = "1790676300000";

    fn now() -> Timestamps {
        Timestamps {
            iso8601: ISO.to_string(),
            unix_ms: UNIX_MS.to_string(),
        }
    }

    fn later() -> Timestamps {
        Timestamps {
            iso8601: LATER_ISO.to_string(),
            unix_ms: LATER_UNIX_MS.to_string(),
        }
    }

    fn board(batch: i64) -> BoardName {
        BoardName::for_tally(&Uuid::nil(), batch)
    }

    /// Two boards for the first election and one for the second; the third
    /// election has no contest area, so no board.
    fn boards() -> Vec<TallyBoardRef> {
        [(1, "election1"), (2, "election1"), (3, "election2")]
            .into_iter()
            .map(|(batch, election_id)| TallyBoardRef {
                name: board(batch),
                election_id: election_id.to_string(),
                area_id: "area".to_string(),
                contest_id: Some("contest".to_string()),
                batch,
            })
            .collect()
    }

    fn waiting(election_id: &str) -> TallyElection {
        TallyElection {
            election_id: election_id.to_string(),
            status: TallyElectionStatus::WAITING,
            progress: 0.0,
        }
    }

    fn state(execution: TallyExecutionStatus) -> TallySessionState {
        TallySessionState::new(
            execution,
            TallyCeremonyStatus {
                stop_date: None,
                logs: vec![],
                trustees: ["trustee1", "trustee2"]
                    .into_iter()
                    .map(|name| TallyTrustee {
                        name: name.to_string(),
                        status: TallyTrusteeStatus::WAITING,
                    })
                    .collect(),
                elections_status: ["election1", "election2", "election3"]
                    .into_iter()
                    .map(waiting)
                    .collect(),
                boards: None,
                failure: None,
            },
        )
    }

    fn in_progress() -> TallySessionState {
        state(TallyExecutionStatus::IN_PROGRESS)
    }

    fn apply(
        state: &TallySessionState,
        event: TallyEvent,
    ) -> TallySessionState {
        transition(state, event, &now())
    }

    fn created() -> TallySessionState {
        apply(&in_progress(), TallyEvent::BoardsCreated(boards()))
    }

    fn phase(
        batch: i64,
        phase: TallyBoardPhase,
    ) -> (BoardName, TallyBoardReading) {
        (board(batch), TallyBoardReading::Phase(phase))
    }

    fn unusable(
        batch: i64,
        reason: TallyFailureReason,
    ) -> (BoardName, TallyBoardReading) {
        (
            board(batch),
            TallyBoardReading::Unusable {
                reason,
                detail: "what the board said".to_string(),
            },
        )
    }

    fn read(readings: Vec<(BoardName, TallyBoardReading)>) -> TallyEvent {
        TallyEvent::ReadBoards(readings)
    }

    fn every_board(phase_of_all: TallyBoardPhase) -> TallyEvent {
        read((1..=3).map(|batch| phase(batch, phase_of_all)).collect())
    }

    const HALT_DETAIL: &str = "the board was rewritten";

    fn halted(trustee: &str) -> TallyEvent {
        TallyEvent::TrusteeHalted {
            trustee: trustee.to_string(),
            detail: HALT_DETAIL.to_string(),
        }
    }

    /// Every event but a trustee's halt: what the platform does with the
    /// boards themselves.
    fn board_events() -> Vec<TallyEvent> {
        vec![
            TallyEvent::BoardsCreated(boards()),
            TallyEvent::BoardsCreated(Vec::new()),
            read(vec![
                phase(1, TallyBoardPhase::MIXING),
                phase(3, TallyBoardPhase::DECRYPTED),
            ]),
            every_board(TallyBoardPhase::DECRYPTED),
            read(vec![unusable(2, TallyFailureReason::INVALID_BOARD_CONTENT)]),
        ]
    }

    fn every_event() -> Vec<TallyEvent> {
        let mut events = board_events();
        events.push(halted("trustee1"));
        events
    }

    fn phases(state: &TallySessionState) -> Vec<TallyBoardPhase> {
        state
            .status
            .boards
            .as_ref()
            .expect("the boards are recorded")
            .iter()
            .map(|board| board.phase)
            .collect()
    }

    fn election(state: &TallySessionState, election_id: &str) -> TallyElection {
        state
            .status
            .elections_status
            .iter()
            .find(|election| election.election_id == election_id)
            .cloned()
            .expect("the election is listed")
    }

    /// A session whose boards are all decrypted and whose results the task
    /// has recorded.
    fn completed(execution: TallyExecutionStatus) -> TallySessionState {
        let mut state =
            apply(&created(), every_board(TallyBoardPhase::DECRYPTED));
        state.execution = execution;
        state
    }

    #[test]
    fn nothing_happens_before_the_session_is_in_progress() {
        for execution in [
            TallyExecutionStatus::STARTED,
            TallyExecutionStatus::CONNECTED,
        ] {
            let start = state(execution.clone());
            for event in every_event() {
                assert_eq!(apply(&start, event.clone()), start, "{event:?}");
            }
        }
    }

    #[test]
    fn creating_the_boards_records_each_one_waiting_for_its_ballots() {
        let next = created();

        assert_eq!(next.execution, TallyExecutionStatus::IN_PROGRESS);
        let recorded = next.status.boards.clone().unwrap();
        assert_eq!(
            recorded
                .iter()
                .map(|board| (board.name.clone(), board.batch))
                .collect::<Vec<_>>(),
            vec![
                (board(1).to_string(), 1),
                (board(2).to_string(), 2),
                (board(3).to_string(), 3)
            ]
        );
        assert_eq!(recorded[2].election_id, "election2");
        assert_eq!(phases(&next), vec![TallyBoardPhase::BALLOTS_PENDING; 3]);
        assert_eq!(next.status.logs.len(), 1);
        assert!(next.status.logs[0].log_text.contains('3'));
        assert_eq!(election(&next, "election1"), waiting("election1"));
        // No board to decrypt: nothing to wait for.
        let election3 = election(&next, "election3");
        assert_eq!(election3.status, TallyElectionStatus::SUCCESS);
        assert_eq!(election3.progress, 100.0);
    }

    #[test]
    fn the_boards_are_recorded_once() {
        let first = created();
        let mut other = boards();
        other.truncate(1);
        assert_eq!(apply(&first, TallyEvent::BoardsCreated(other)), first);
        assert_eq!(apply(&first, TallyEvent::BoardsCreated(Vec::new())), first);
    }

    #[test]
    fn a_session_with_nothing_to_decrypt_is_decrypted_vacuously() {
        let next = apply(&in_progress(), TallyEvent::BoardsCreated(Vec::new()));

        assert_eq!(next.execution, TallyExecutionStatus::IN_PROGRESS);
        assert_eq!(next.status.boards, Some(Vec::new()));
        for election in &next.status.elections_status {
            assert_eq!(election.status, TallyElectionStatus::SUCCESS);
            assert_eq!(election.progress, 100.0);
        }
        // Recorded as extracted, so the boards are never created again.
        assert_eq!(apply(&next, TallyEvent::BoardsCreated(boards())), next);
    }

    #[test]
    fn board_phases_follow_the_readings_and_are_logged_once_each() {
        let next = apply(
            &created(),
            read(vec![
                phase(1, TallyBoardPhase::MIXING),
                phase(3, TallyBoardPhase::BALLOTS_POSTED),
            ]),
        );

        // Board 2 was not read and keeps its phase.
        assert_eq!(
            phases(&next),
            vec![
                TallyBoardPhase::MIXING,
                TallyBoardPhase::BALLOTS_PENDING,
                TallyBoardPhase::BALLOTS_POSTED
            ]
        );
        let lines = &next.status.logs[1..];
        assert_eq!(lines.len(), 2);
        assert!(lines[0].log_text.contains(board(1).as_str()));
        assert!(lines[1].log_text.contains(board(3).as_str()));
        assert!(lines.iter().all(|line| line.created_date == ISO));
    }

    /// The phases a board passes through, in the order it passes them.
    ///
    /// Written out here rather than derived from [`rank`] so that the two
    /// are independent statements of the same order.
    const PHASES_IN_ORDER: [TallyBoardPhase; 5] = [
        TallyBoardPhase::BALLOTS_PENDING,
        TallyBoardPhase::BALLOTS_POSTED,
        TallyBoardPhase::MIXING,
        TallyBoardPhase::DECRYPTING,
        TallyBoardPhase::DECRYPTED,
    ];

    #[test]
    fn a_reading_only_ever_moves_a_board_forward() {
        // The board service may lose a message a later reading then misses;
        // a phase the session recorded is never taken back.
        for (position, current) in PHASES_IN_ORDER.iter().enumerate() {
            for (other, observed) in PHASES_IN_ORDER.iter().enumerate() {
                let mut start = created();
                start.status.boards.as_mut().unwrap()[0].phase = *current;
                let next = apply(&start, read(vec![phase(1, *observed)]));

                assert_eq!(
                    phases(&next)[0],
                    PHASES_IN_ORDER[position.max(other)],
                    "{current} then {observed}"
                );
                assert_eq!(
                    next.status.logs.len() - start.status.logs.len(),
                    usize::from(other > position),
                    "{current} then {observed}"
                );
            }
        }
    }

    #[test]
    fn a_reading_of_a_board_the_session_does_not_list_is_ignored() {
        let unlisted = (
            BoardName::for_tally(&Uuid::new_v4(), 1),
            TallyBoardReading::Phase(TallyBoardPhase::DECRYPTED),
        );
        let unlisted_unusable = (
            BoardName::for_tally(&Uuid::new_v4(), 1),
            TallyBoardReading::Unusable {
                reason: TallyFailureReason::INVALID_BOARD_CONTENT,
                detail: "someone else's board".to_string(),
            },
        );

        let next = apply(
            &created(),
            read(vec![
                unlisted.clone(),
                unlisted_unusable.clone(),
                phase(2, TallyBoardPhase::MIXING),
            ]),
        );
        assert_eq!(next.execution, TallyExecutionStatus::IN_PROGRESS);
        assert_eq!(
            phases(&next),
            vec![
                TallyBoardPhase::BALLOTS_PENDING,
                TallyBoardPhase::MIXING,
                TallyBoardPhase::BALLOTS_PENDING
            ]
        );

        // Before the boards are recorded, no reading names one of them.
        let before = in_progress();
        assert_eq!(
            apply(
                &before,
                read(vec![unlisted, phase(1, TallyBoardPhase::DECRYPTED)])
            ),
            before
        );
        assert_eq!(apply(&before, read(vec![unlisted_unusable])), before);
    }

    #[test]
    fn an_unusable_board_fails_the_session_with_the_first_reason() {
        for (first, second) in [
            (
                TallyFailureReason::BOARD_CONFIGURATION_MISMATCH,
                TallyFailureReason::INVALID_BOARD_CONTENT,
            ),
            (
                TallyFailureReason::INVALID_BOARD_CONTENT,
                TallyFailureReason::BOARD_CONFIGURATION_MISMATCH,
            ),
        ] {
            let start = created();
            let next = apply(
                &start,
                read(vec![
                    phase(1, TallyBoardPhase::MIXING),
                    unusable(3, first),
                    unusable(2, second),
                ]),
            );

            assert_eq!(next.execution, TallyExecutionStatus::FAILED, "{first}");
            let failure = next.status.failure.clone().unwrap();
            assert_eq!(failure.reason, first);
            assert!(failure.detail.contains(board(3).as_str()));
            assert!(failure.detail.contains("what the board said"));
            assert_eq!(failure.failed_at, ISO);
            assert_eq!(next.status.stop_date, Some(UNIX_MS.to_string()));
            assert_eq!(next.status.logs.len(), start.status.logs.len() + 1);
            assert!(next
                .status
                .logs
                .last()
                .is_some_and(|line| line.log_text.contains("failed")));
            assert_eq!(phases(&next), phases(&start));
        }
    }

    #[test]
    fn an_election_progresses_with_its_boards() {
        use TallyBoardPhase::*;
        for (election1, status, progress) in [
            (
                [BALLOTS_PENDING, BALLOTS_POSTED],
                TallyElectionStatus::WAITING,
                0.0,
            ),
            ([MIXING, BALLOTS_POSTED], TallyElectionStatus::MIXING, 10.0),
            ([MIXING, MIXING], TallyElectionStatus::MIXING, 20.0),
            ([DECRYPTING, MIXING], TallyElectionStatus::DECRYPTING, 30.0),
            ([DECRYPTED, MIXING], TallyElectionStatus::MIXING, 60.0),
            (
                [DECRYPTED, DECRYPTING],
                TallyElectionStatus::DECRYPTING,
                70.0,
            ),
            (
                [DECRYPTED, BALLOTS_POSTED],
                TallyElectionStatus::WAITING,
                50.0,
            ),
            ([DECRYPTED, DECRYPTED], TallyElectionStatus::SUCCESS, 100.0),
        ] {
            let next = apply(
                &created(),
                read(vec![phase(1, election1[0]), phase(2, election1[1])]),
            );
            let first = election(&next, "election1");
            assert_eq!(first.status, status, "{election1:?}");
            assert!(
                (first.progress - progress).abs() < 1e-9,
                "{election1:?}: {}",
                first.progress
            );
            // The other elections' boards did not move.
            assert_eq!(election(&next, "election2"), waiting("election2"));
            assert_eq!(
                election(&next, "election3").status,
                TallyElectionStatus::SUCCESS
            );
        }
    }

    #[test]
    fn an_election_the_boards_name_is_listed_too() {
        let mut start = in_progress();
        start.status.elections_status.clear();
        let next = apply(&start, TallyEvent::BoardsCreated(boards()));
        assert_eq!(
            next.status
                .elections_status
                .iter()
                .map(|election| election.election_id.as_str())
                .collect::<Vec<_>>(),
            vec!["election1", "election2"]
        );
    }

    #[test]
    fn a_halt_marks_the_trustee_and_fails_the_session_even_after_success() {
        let mixing = apply(&created(), every_board(TallyBoardPhase::MIXING));
        for start in [
            mixing,
            completed(TallyExecutionStatus::AWAITING_INPUT),
            completed(TallyExecutionStatus::SUCCESS),
        ] {
            let next = transition(&start, halted("trustee2"), &later());

            assert_eq!(
                next.execution,
                TallyExecutionStatus::FAILED,
                "{:?}",
                start.execution
            );
            assert_eq!(
                next.status.trustees[1].status,
                TallyTrusteeStatus::HALTED
            );
            assert_eq!(next.status.trustees[0], start.status.trustees[0]);
            assert_eq!(
                next.status.failure,
                Some(TallyFailure {
                    reason: TallyFailureReason::TRUSTEE_HALTED,
                    detail: format!("trustee2: {HALT_DETAIL}"),
                    failed_at: LATER_ISO.to_string(),
                })
            );
            assert_eq!(next.status.stop_date, Some(LATER_UNIX_MS.to_string()));
            assert_eq!(next.status.logs.len(), start.status.logs.len() + 2);
            // What the boards reached stays on the record.
            assert_eq!(next.status.boards, start.status.boards);
            assert_eq!(
                next.status.elections_status,
                start.status.elections_status
            );
        }
    }

    #[test]
    fn terminal_states_absorb_every_event_but_a_finished_tally_takes_a_halt() {
        let failed = apply(
            &created(),
            read(vec![unusable(1, TallyFailureReason::INVALID_BOARD_CONTENT)]),
        );
        assert_eq!(failed.execution, TallyExecutionStatus::FAILED);
        let mut cancelled = created();
        cancelled.execution = TallyExecutionStatus::CANCELLED;
        for start in [failed, cancelled] {
            for event in every_event() {
                assert_eq!(apply(&start, event.clone()), start, "{event:?}");
            }
        }

        for execution in [
            TallyExecutionStatus::AWAITING_INPUT,
            TallyExecutionStatus::SUCCESS,
        ] {
            let finished = completed(execution.clone());
            for event in board_events() {
                assert_eq!(
                    apply(&finished, event.clone()),
                    finished,
                    "{execution}: {event:?}"
                );
            }
            assert_eq!(
                apply(&finished, halted("trustee1")).execution,
                TallyExecutionStatus::FAILED
            );
        }
    }

    #[test]
    fn a_halt_naming_a_trustee_the_session_does_not_list_changes_nothing() {
        for start in [created(), completed(TallyExecutionStatus::SUCCESS)] {
            assert_eq!(apply(&start, halted("trustee3")), start);
        }
    }

    #[test]
    fn only_the_first_halt_is_recorded() {
        let first = apply(&created(), halted("trustee2"));
        let second = apply(&first, halted("trustee1"));
        assert_eq!(second, first);
        assert_eq!(
            second.status.trustees[0].status,
            TallyTrusteeStatus::WAITING
        );
    }

    #[test]
    fn applying_the_same_event_twice_changes_nothing_the_second_time() {
        for start in [
            state(TallyExecutionStatus::STARTED),
            in_progress(),
            created(),
            completed(TallyExecutionStatus::SUCCESS),
        ] {
            for event in every_event() {
                let once = apply(&start, event.clone());
                let twice = apply(&once, event.clone());
                assert_eq!(twice, once, "{event:?} on {:?}", start.execution);
            }
        }
    }

    #[test]
    fn the_log_gains_one_line_per_thing_that_happened() {
        let mut state = in_progress();
        for (event, lines) in [
            (TallyEvent::BoardsCreated(boards()), 1),
            (every_board(TallyBoardPhase::BALLOTS_POSTED), 4),
            // Nothing new on the boards.
            (every_board(TallyBoardPhase::BALLOTS_POSTED), 4),
            (read(vec![phase(2, TallyBoardPhase::MIXING)]), 5),
            // A trustee halts and the session fails.
            (halted("trustee1"), 7),
            // Recorded already.
            (halted("trustee1"), 7),
        ] {
            state = apply(&state, event.clone());
            assert_eq!(state.status.logs.len(), lines, "{event:?}");
        }
        assert_eq!(state.execution, TallyExecutionStatus::FAILED);
    }
}
