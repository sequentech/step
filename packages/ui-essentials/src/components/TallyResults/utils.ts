// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {formatPercentOne} from "@sequentech/ui-core"
import {defaultResultsAndParticipationLabels} from "./types"
import type {
    CandidateResultRow,
    NumericValue,
    ResultsAndParticipationLabelOverrides,
    ResultsAndParticipationLabels,
} from "./types"

export const mergeLabels = (
    labels?: ResultsAndParticipationLabelOverrides
): ResultsAndParticipationLabels => ({
    ...defaultResultsAndParticipationLabels,
    ...labels,
    channelNames: {
        ...defaultResultsAndParticipationLabels.channelNames,
        ...labels?.channelNames,
    },
})

export const toFiniteNumber = (value: NumericValue): number | null => {
    if (typeof value === "number") {
        return Number.isFinite(value) ? value : null
    }

    if (typeof value === "string") {
        const trimmed = value.trim()
        if (!trimmed) return null

        const parsed = Number(trimmed)
        return Number.isFinite(parsed) ? parsed : null
    }

    return null
}

export const valueOrDash = (value: NumericValue): string | number => toFiniteNumber(value) ?? "-"

export const percentOrDash = (value: NumericValue): string => {
    const numeric = toFiniteNumber(value)
    return numeric !== null ? formatPercentOne(numeric) : "-"
}

export const sortCandidateResults = (
    left: CandidateResultRow,
    right: CandidateResultRow
): number => {
    const leftWinning = toFiniteNumber(left.winningPosition) ?? Number.MAX_SAFE_INTEGER
    const rightWinning = toFiniteNumber(right.winningPosition) ?? Number.MAX_SAFE_INTEGER

    if (leftWinning !== rightWinning) {
        return leftWinning - rightWinning
    }

    return (toFiniteNumber(right.castVotes) ?? 0) - (toFiniteNumber(left.castVotes) ?? 0)
}
