// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {BallotSelection, IDecodedVoteContest} from "@sequentech/ui-core"
import type {IResolvedSlate, ISlateContest} from "./Slates"

const SELECTED = 0
const UNSELECTED = -1

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

const applyToContest = (
    slate: IResolvedSlate,
    {contest, candidates}: ISlateContest,
    current: IDecodedVoteContest
): {next: IDecodedVoteContest; change: ISlateContestChange} => {
    const memberIds = candidates.map((candidate) => candidate.id)
    const members = new Set(memberIds)
    if (members.size === 0 || members.size !== memberIds.length) {
        throw new SlateChoicesError(
            `slate "${slate.id}" has no candidates or repeats one in contest "${contest.id}"`
        )
    }
    if (members.size > contest.max_votes) {
        throw new SlateChoicesError(
            `slate "${slate.id}" has ${members.size} candidates in contest "${contest.id}", which allows ${contest.max_votes}`
        )
    }
    const choiceIds = new Set(current.choices.map((choice) => choice.id))
    const missing = memberIds.find((id) => !choiceIds.has(id))
    if (missing) {
        throw new SlateChoicesError(
            `candidate "${missing}" of slate "${slate.id}" is not a choice of contest "${contest.id}"`
        )
    }

    const wasSelected = new Set(
        current.choices.filter((choice) => choice.selected > UNSELECTED).map((choice) => choice.id)
    )
    const removed = current.choices
        .filter((choice) => wasSelected.has(choice.id) && !members.has(choice.id))
        .map((choice) => choice.id)
    if (current.is_explicit_invalid) {
        const invalidCandidate = contest.candidates.find(
            (candidate) => candidate.presentation?.is_explicit_invalid
        )
        if (invalidCandidate && !removed.includes(invalidCandidate.id)) {
            removed.push(invalidCandidate.id)
        }
    }

    return {
        next: {
            ...current,
            is_explicit_invalid: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: current.choices.map((choice) => ({
                id: choice.id,
                selected: members.has(choice.id) ? SELECTED : UNSELECTED,
            })),
        },
        change: {
            contestId: contest.id,
            added: memberIds.filter((id) => !wasSelected.has(id)),
            removed,
        },
    }
}

/**
 * The ballot that choosing a slate produces, and what it changes.
 *
 * In each contest the slate covers, its candidates replace the current marks,
 * so the result never exceeds the contest's maximum. Contests it does not
 * cover keep their choices. Everything is checked before anything is returned:
 * a slate that cannot be applied in full throws and applies nothing.
 */
export const computeSlateChoices = (
    slate: IResolvedSlate,
    current: BallotSelection
): ISlateChoices => {
    if (slate.contests.length === 0) {
        throw new SlateChoicesError(`slate "${slate.id}" has no candidates on this ballot`)
    }

    const applied = new Map<string, IDecodedVoteContest>()
    const changes: ISlateContestChange[] = []
    for (const slateContest of slate.contests) {
        const contestId = slateContest.contest.id
        const currentContest = current.find((contest) => contest.contest_id === contestId)
        if (!currentContest || applied.has(contestId)) {
            throw new SlateChoicesError(
                `contest "${contestId}" of slate "${slate.id}" is missing or repeated`
            )
        }
        const {next, change} = applyToContest(slate, slateContest, currentContest)
        applied.set(contestId, next)
        if (change.added.length > 0 || change.removed.length > 0) {
            changes.push(change)
        }
    }

    return {
        selection: current.map((contest) => ({
            ...(applied.get(contest.contest_id) ?? contest),
            is_blank_ballot: false,
            is_decline_to_vote: false,
        })),
        changes,
    }
}

export const slateRemovesChoices = (choices: ISlateChoices): boolean =>
    choices.changes.some((change) => change.removed.length > 0)
