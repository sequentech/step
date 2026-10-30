// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {applySlate, BallotSelection, IContest, ISlate, ISlateProblem} from "@sequentech/ui-core"
import {exceedsMaximum} from "./SelectionLimits"
import type {IResolvedSlate, ISlateContest} from "./Slates"

export interface ISlateContestChange {
    contestId: string
    added: string[]
    removed: string[]
}

export interface ISlateChoices {
    selection: BallotSelection
    changes: ISlateContestChange[]
}

export class SlateChoicesError extends Error {
    constructor(message: string) {
        super(message)
        this.name = "SlateChoicesError"
    }
}

const describeProblems = (error: unknown): string => {
    const problems = Array.isArray(error) ? error : [error]
    return problems
        .map((problem) => (problem as Partial<ISlateProblem>)?.message ?? String(problem))
        .join("; ")
}

/** The slate has more candidates in a contest than the contest allows. */
export class SlateOverMaximumError extends SlateChoicesError {
    constructor(
        slate: IResolvedSlate,
        readonly slateContest: ISlateContest
    ) {
        super(
            `slate "${slate.id}" has ${slateContest.candidates.length} candidates in contest "${slateContest.contest.id}", which allows ${slateContest.contest.max_votes}`
        )
        this.name = "SlateOverMaximumError"
    }
}

/**
 * The ballot that choosing a slate produces, and what it changes.
 *
 * The rule is sequent-core's: every contest of the ballot ends with exactly
 * the slate's candidates, and a contest the slate has no candidate in is left
 * blank. Acclaimed contests keep their entries. The result is an ordinary
 * selection, the same one marking those candidates by hand gives. A slate
 * that cannot be applied in full throws and applies nothing.
 */
export const computeSlateChoices = (
    slate: IResolvedSlate,
    contests: Array<IContest>,
    current: BallotSelection
): ISlateChoices => {
    const members: ISlate["members"] = {}
    for (const slateContest of slate.contests) {
        const memberIds = new Set(slateContest.candidates.map((candidate) => candidate.id))
        if (exceedsMaximum(slateContest.contest, memberIds.size)) {
            throw new SlateOverMaximumError(slate, slateContest)
        }
    }
    for (const {contest, candidates} of slate.contests) {
        members[contest.id] = candidates.map((candidate) => candidate.id)
    }

    try {
        const {selection, changes} = applySlate(
            {id: slate.id, name: slate.name, members},
            contests,
            current
        )
        const marked = new Set([
            ...Object.keys(members),
            ...changes.map((change) => change.contest_id),
        ])
        return {
            // Choices exactly as the voting screen writes them when a candidate
            // is marked by hand.
            selection: selection.map((contest) =>
                marked.has(contest.contest_id)
                    ? {
                          ...contest,
                          choices: contest.choices.map(({id, selected}) => ({id, selected})),
                      }
                    : contest
            ),
            changes: changes.map((change) => ({
                contestId: change.contest_id,
                added: change.added,
                removed: change.removed,
            })),
        }
    } catch (error) {
        throw new SlateChoicesError(describeProblems(error))
    }
}

export const slateRemovesChoices = (choices: ISlateChoices): boolean =>
    choices.changes.some((change) => change.removed.length > 0)
