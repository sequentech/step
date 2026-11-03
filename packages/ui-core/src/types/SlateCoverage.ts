// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ESlateCoverageKind {
    COMPLETE = "complete",
    PARTIAL = "partial",
}

export interface ISlateContestCoverage {
    contest_id: string
    candidate_ids: Array<string>
    seats: number
}

export interface ISlateCoverage {
    slate_id: string
    kind: ESlateCoverageKind
    covered: Array<ISlateContestCoverage>
    uncovered_contest_ids: Array<string>
    members: number
    seats: number
}
