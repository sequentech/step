// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection, IDecodedVoteContest} from "@sequentech/ui-core"

const NOT_SELECTED = -1

const hasSameFlags = (a: IDecodedVoteContest, b: IDecodedVoteContest): boolean =>
    Boolean(a.is_explicit_invalid) === Boolean(b.is_explicit_invalid) &&
    Boolean(a.is_decline_to_vote) === Boolean(b.is_decline_to_vote) &&
    Boolean(a.is_blank_ballot) === Boolean(b.is_blank_ballot)

const getMarks = (contest: IDecodedVoteContest): Map<string, string> =>
    new Map(
        contest.choices
            .filter((choice) => choice.selected > NOT_SELECTED)
            .map((choice) => [choice.id, `${choice.selected}:${choice.write_in_text ?? ""}`])
    )

const hasSameMarks = (a: IDecodedVoteContest, b: IDecodedVoteContest): boolean => {
    const marksOfA = getMarks(a)
    const marksOfB = getMarks(b)
    return (
        marksOfA.size === marksOfB.size &&
        Array.from(marksOfA).every(([id, mark]) => marksOfB.get(id) === mark)
    )
}

/**
 * Whether two ballot selections are the same vote: the same candidates with
 * the same ranks and write-in texts, and the same contest flags. The order of
 * contests and choices does not matter, and neither do validation messages.
 */
export const isSameBallotSelection = (a: BallotSelection, b: BallotSelection): boolean =>
    a.length === b.length &&
    a.every((contest) => {
        const other = b.find((entry) => entry.contest_id === contest.contest_id)
        return !!other && hasSameFlags(contest, other) && hasSameMarks(contest, other)
    })
