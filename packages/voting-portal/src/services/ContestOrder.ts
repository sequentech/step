// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IContest} from "@sequentech/ui-core"

/**
 * The contests in the order the ballot shows them: by their configured
 * position, with the contests without one last. The ballot style lists them
 * in another order.
 */
export const sortContestsInBallotOrder = (contests: Array<IContest>): Array<IContest> =>
    [...contests].sort(
        (a, b) =>
            (a.presentation?.sort_order ?? Infinity) - (b.presentation?.sort_order ?? Infinity)
    )
