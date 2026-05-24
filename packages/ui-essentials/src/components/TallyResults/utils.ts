// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Lifted from admin-portal/src/resources/Tally/utils.ts (winningPositionComparator only).
// Adaptations:
//   U1: drop convertSequentContestToIContest / convertContestsArray
//       (admin-only graphql conversion helpers).
//   U2: drop parseProcessResults (admin-only annotation parser; workbench
//       computes process_results directly via velvet).

import {GridComparatorFn} from "@mui/x-data-grid"

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
