// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Candidate_Extended} from "../resources/Tally/types"

/**
 * Sorts candidates based on winning position and cast votes
 * Candidates with winning positions are sorted first by their position
 * Candidates without winning positions are sorted by cast votes (descending)
 */
export function sortCandidates(
    a: Sequent_Backend_Candidate_Extended,
    b: Sequent_Backend_Candidate_Extended
): number {
    if (a.winning_position && b.winning_position) {
        return a.winning_position - b.winning_position
    } else if (a.winning_position) {
        return -1
    } else if (b.winning_position) {
        return 1
    } else {
        return (b.cast_votes ?? 0) - (a.cast_votes ?? 0)
    }
}
