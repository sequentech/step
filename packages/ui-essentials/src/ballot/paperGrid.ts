// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ECandidatesLayout, IContest} from "@sequentech/ui-core"

export const PAPER_GRID_DEFAULT_COLUMNS = 4
export const PAPER_GRID_MAX_SMALL_SCREEN_COLUMNS = 2
export const PAPER_GRID_MAX_MEDIUM_SCREEN_COLUMNS = 3
export const PAPER_GRID_MAX_REGULAR_ROWS = 12
export const PAPER_GRID_MAX_COMPACT_ROWS = 24

export enum EPaperGridDensity {
    REGULAR = "regular",
    COMPACT = "compact",
    DENSE = "dense",
}

export interface IPaperGridColumns {
    small: number
    medium: number
    large: number
}

/**
 * The layout a contest's candidates are drawn with.
 *
 * A ranked contest needs a position picker per candidate, which a paper cell has
 * no room for, so it keeps the list whatever was configured.
 */
export const getCandidatesLayout = (
    contest: IContest,
    isPreferential: boolean
): ECandidatesLayout => {
    const configured = contest.presentation?.candidates_layout ?? ECandidatesLayout.LIST
    return isPreferential ? ECandidatesLayout.LIST : configured
}

/** `presentation.columns` when it was set, four (the printed ballot's) otherwise. */
export const getPaperGridColumns = (contest: IContest): number => {
    const columns = Math.floor(contest.presentation?.columns ?? PAPER_GRID_DEFAULT_COLUMNS)
    return columns >= 1 ? columns : PAPER_GRID_DEFAULT_COLUMNS
}

/** How many columns fit at each width; phones get at most two. */
export const getResponsivePaperGridColumns = (columns: number): IPaperGridColumns => ({
    small: Math.min(columns, PAPER_GRID_MAX_SMALL_SCREEN_COLUMNS),
    medium: Math.min(columns, PAPER_GRID_MAX_MEDIUM_SCREEN_COLUMNS),
    large: columns,
})

/** Rows needed to fill `columns` columns top to bottom, one column at a time. */
export const getPaperGridRows = (candidateCount: number, columns: number): number =>
    Math.max(1, Math.ceil(candidateCount / Math.max(1, columns)))

/** Rows get shorter as a column gets longer, as they do on paper. */
export const getPaperGridDensity = (candidateCount: number, columns: number): EPaperGridDensity => {
    const rows = getPaperGridRows(candidateCount, columns)
    if (rows <= PAPER_GRID_MAX_REGULAR_ROWS) {
        return EPaperGridDensity.REGULAR
    }
    if (rows <= PAPER_GRID_MAX_COMPACT_ROWS) {
        return EPaperGridDensity.COMPACT
    }
    return EPaperGridDensity.DENSE
}
