// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GridComparatorFn} from "@mui/x-data-grid"
import {ParsedAnnotations, RunoffStatus} from "./types"
import {ICountingAlgorithm} from "@sequentech/ui-core"

/**
 * Comparator function for sorting winning positions in DataGrid.
 * Positions are sorted numerically, with non-numeric values sorted to the end.
 */
export const winningPositionComparator: GridComparatorFn<string> = (v1, v2) => {
    const maxInt = Number.MAX_SAFE_INTEGER

    // Convert stringified numbers to integers, non-numeric strings to maxInt
    const pos1 = isNaN(parseInt(v1)) ? maxInt : parseInt(v1)
    const pos2 = isNaN(parseInt(v2)) ? maxInt : parseInt(v2)

    return pos1 - pos2
}

/**
 * Parses and processes contest results based on counting algorithm.
 * Handles IRV/Runoff voting and other algorithms.
 *
 * @param annotations - Raw annotations string from general results
 * @param counting_algorithm - The counting algorithm used for this contest
 * @returns Parsed process results or null if parsing fails or no results available
 */
export const parseProcessResults = (
    annotations: string | null | undefined,
    counting_algorithm: ICountingAlgorithm
): RunoffStatus | unknown | null => {
    try {
        const parsedAnnotations: ParsedAnnotations | null = annotations
            ? (JSON.parse(annotations as string) as ParsedAnnotations)
            : null

        const results = parsedAnnotations?.process_results ?? null

        if (results && counting_algorithm) {
            switch (counting_algorithm) {
                case ICountingAlgorithm.INSTANT_RUNOFF: {
                    const runoffResults = results as RunoffStatus
                    return runoffResults
                }
                default:
                    console.log("Unknown counting algorithm process_results:", results)
                    return results
            }
        }

        return null
    } catch (error) {
        console.error("Error parsing process_results:", error)
        return null
    }
}
