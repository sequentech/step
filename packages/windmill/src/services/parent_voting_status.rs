// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::VotingStatus;

/// Whether a successfully checked Post transition opens its parent channel.
/// This never changes a sibling Post or revives an ended parent channel.
pub(super) fn should_open_parent(
    current: VotingStatus,
    requested: VotingStatus,
    post_changed: bool,
) -> bool {
    post_changed
        && requested == VotingStatus::OPEN
        && matches!(current, VotingStatus::NOT_STARTED | VotingStatus::PAUSED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_post_resume_restores_paused_parent() {
        assert!(should_open_parent(
            VotingStatus::PAUSED,
            VotingStatus::OPEN,
            true
        ));
    }

    #[test]
    fn a_refused_or_unchanged_post_cannot_open_its_parent() {
        for parent in [VotingStatus::NOT_STARTED, VotingStatus::PAUSED] {
            assert!(!should_open_parent(parent, VotingStatus::OPEN, false));
        }
    }

    #[test]
    fn closed_parent_cannot_reopen_even_after_a_post_change() {
        for changed in [false, true] {
            assert!(!should_open_parent(
                VotingStatus::CLOSED,
                VotingStatus::OPEN,
                changed
            ));
        }
    }

    #[test]
    fn only_guarded_opening_of_an_unstarted_or_paused_parent_changes_it() {
        let states = [
            VotingStatus::NOT_STARTED,
            VotingStatus::OPEN,
            VotingStatus::PAUSED,
            VotingStatus::CLOSED,
        ];
        let permitted = [
            (VotingStatus::NOT_STARTED, VotingStatus::OPEN, true),
            (VotingStatus::PAUSED, VotingStatus::OPEN, true),
        ];
        for parent in states {
            for requested in states {
                for changed in [false, true] {
                    assert_eq!(
                        should_open_parent(parent, requested, changed),
                        permitted.contains(&(parent, requested, changed)),
                        "parent={parent:?}, requested={requested:?}, changed={changed}"
                    );
                }
            }
        }
    }
}
