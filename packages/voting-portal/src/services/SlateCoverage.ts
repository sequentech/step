// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ESlateCoverageKind, ICandidate, IContest, ISlateCoverage} from "@sequentech/ui-core"

export enum ESlateCoverageLabel {
    FULL = "full",
    SINGLE_CONTEST = "single-contest",
    PARTIAL = "partial",
}

export interface ISlateCoverageLabel {
    label: ESlateCoverageLabel
    contest?: IContest
}

export interface ISlateOffice {
    contest: IContest
    candidates: Array<ICandidate>
}

export enum ESlateOfficesLayout {
    ALIGNED = "aligned",
    STACKED = "stacked",
}

const findContest = (contests: Array<IContest>, contestId: string): IContest | undefined =>
    contests.find((contest) => contest.id === contestId)

export const getSlateCoverageLabel = (
    coverage: ISlateCoverage,
    contests: Array<IContest>
): ISlateCoverageLabel => {
    if (coverage.kind === ESlateCoverageKind.COMPLETE) {
        return {label: ESlateCoverageLabel.FULL}
    }
    const onlyContest =
        coverage.covered.length === 1 && coverage.uncovered_contest_ids.length > 0
            ? findContest(contests, coverage.covered[0].contest_id)
            : undefined
    if (onlyContest) {
        return {label: ESlateCoverageLabel.SINGLE_CONTEST, contest: onlyContest}
    }
    return {label: ESlateCoverageLabel.PARTIAL}
}

export const getCoveredContests = (
    coverage: ISlateCoverage,
    contests: Array<IContest>
): Array<IContest> =>
    coverage.covered.flatMap((covered) => findContest(contests, covered.contest_id) ?? [])

export const getUncoveredContests = (
    coverage: ISlateCoverage,
    contests: Array<IContest>
): Array<IContest> =>
    coverage.uncovered_contest_ids.flatMap((contestId) => findContest(contests, contestId) ?? [])

/**
 * The offices of a slate card, in ballot order. The aligned layout lists every
 * contest the voter can vote in, with no candidates where the slate has none, so
 * that the same office sits at the same level on every card. The stacked layout
 * lists only the contests the slate has candidates in.
 */
export const getSlateOffices = (
    coverage: ISlateCoverage,
    contests: Array<IContest>,
    layout: ESlateOfficesLayout
): Array<ISlateOffice> =>
    contests.flatMap((contest) => {
        const covered = coverage.covered.find((entry) => entry.contest_id === contest.id)
        if (covered) {
            const candidates = covered.candidate_ids.flatMap(
                (candidateId) =>
                    contest.candidates.find((candidate) => candidate.id === candidateId) ?? []
            )
            return [{contest, candidates}]
        }
        const isUncovered = coverage.uncovered_contest_ids.includes(contest.id)
        return isUncovered && layout === ESlateOfficesLayout.ALIGNED
            ? [{contest, candidates: []}]
            : []
    })

export const isContestCoveredBySlate = (coverage: ISlateCoverage, contestId: string): boolean =>
    coverage.covered.some((covered) => covered.contest_id === contestId)
