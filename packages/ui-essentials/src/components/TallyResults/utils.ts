// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {formatPercentOne} from "@sequentech/ui-core"
import {MAX_CANDIDATES_REPRESENTED} from "./constants"
import {defaultResultsAndParticipationLabels} from "./types"
import type {
    CandidateReference,
    CandidateResultRow,
    NumericValue,
    ResultsAndParticipationLabelOverrides,
    ResultsAndParticipationLabels,
} from "./types"

export const orderCandidateReferences = (
    references: CandidateReference[],
    candidates: CandidateResultRow[]
): CandidateReference[] => {
    const referenceById = new Map(references.map((reference) => [reference.id, reference]))
    const orderedReferences = candidates
        .map((candidate) => referenceById.get(candidate.id))
        .filter((reference): reference is CandidateReference => !!reference)
    const orderedIds = new Set(orderedReferences.map((reference) => reference.id))

    return [
        ...orderedReferences,
        ...references.filter((reference) => !orderedIds.has(reference.id)),
    ]
}

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

export const buildCandidateChartData = (
    results: CandidateResultRow[],
    labels: ResultsAndParticipationLabels
) => {
    const representedResults = results
        .map((candidate) => ({
            label: candidate.name || "-",
            value: toFiniteNumber(candidate.castVotes) ?? 0,
        }))
        .filter((item) => item.value > 0)

    if (representedResults.length > MAX_CANDIDATES_REPRESENTED) {
        const deletedItems = representedResults.splice(MAX_CANDIDATES_REPRESENTED)
        const othersSum = deletedItems.reduce((sum, item) => sum + item.value, 0)
        representedResults.push({label: labels.others, value: othersSum})
    }

    return representedResults
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
