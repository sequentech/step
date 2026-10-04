// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {applySlate, BallotSelection, ISlate, ISlateProblem} from "@sequentech/ui-core"
import type {IResolvedSlate} from "./Slates"

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

/**
 * The ballot that choosing a slate produces, and what it changes.
 *
 * The rule is sequent-core's: in each contest the slate covers, its candidates
 * replace the current marks, and contests it does not cover keep their choices.
 * The result is an ordinary selection, the same one marking those candidates
 * by hand gives. A slate that cannot be applied in full throws and applies
 * nothing.
 */
export const computeSlateChoices = (
    slate: IResolvedSlate,
    current: BallotSelection
): ISlateChoices => {
    const members: ISlate["members"] = {}
    for (const {contest, candidates} of slate.contests) {
        members[contest.id] = candidates.map((candidate) => candidate.id)
    }

    try {
        const {selection, changes} = applySlate(
            {id: slate.id, name: slate.name, members},
            slate.contests.map(({contest}) => contest),
            current
        )
        const covered = new Set(Object.keys(members))
        return {
            // Choices exactly as the voting screen writes them when a candidate
            // is marked by hand.
            selection: selection.map((contest) =>
                covered.has(contest.contest_id)
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
