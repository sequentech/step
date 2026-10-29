// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BallotSelection,
    checkIsInvalidVote,
    IContest,
    IDecodedVoteContest,
    isAcclaimedContest,
} from "@sequentech/ui-core"
import type {ISlateChoices, ISlateContestChange} from "./SlateChoices"

const UNSELECTED = -1

/** The marks of a contest, with its explicit invalid option when it is marked. */
const getSelectedIds = (contest: IContest, entry: IDecodedVoteContest): string[] => {
    const ids = entry.choices
        .filter((choice) => choice.selected > UNSELECTED)
        .map((choice) => choice.id)
    const invalidCandidate = contest.candidates.find(checkIsInvalidVote)
    if (entry.is_explicit_invalid && invalidCandidate && !ids.includes(invalidCandidate.id)) {
        ids.push(invalidCandidate.id)
    }
    return ids
}

/**
 * The ballot without any choice, and what that removes. Acclaimed contests
 * keep their entries. The choices are left as the voting screen writes them
 * when a candidate is unmarked by hand.
 */
export const computeClearChoices = (
    contests: Array<IContest>,
    current: BallotSelection
): ISlateChoices => {
    const changes: ISlateContestChange[] = []
    const selection = current.map((entry): IDecodedVoteContest => {
        const contest = contests.find((item) => item.id === entry.contest_id)
        const flags = {...entry, is_blank_ballot: false, is_decline_to_vote: false}
        if (!contest || isAcclaimedContest(contest)) {
            return flags
        }
        const removed = getSelectedIds(contest, entry)
        if (removed.length === 0) {
            return flags
        }
        changes.push({contestId: contest.id, added: [], removed})
        return {
            ...flags,
            is_explicit_invalid: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: entry.choices.map(({id}) => ({id, selected: UNSELECTED})),
        }
    })
    return {selection, changes}
}

export const clearRemovesChoices = (choices: ISlateChoices): boolean => choices.changes.length > 0
