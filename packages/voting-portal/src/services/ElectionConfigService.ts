// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Where these live now.
 *
 * The presentation helpers moved into `@sequentech/ui-essentials` with the ballot
 * components that call them — they are pure reads on `IContest`/`ICandidate`, and
 * the shared ballot cannot compile without them. This keeps the portal's own
 * screens and services importing from the path they always did.
 *
 * Named rather than `export *`, so this module exports what it always exported and
 * not the whole of another package's barrel.
 */

export {
    findUrlByTitle,
    getImageUrl,
    getLinkUrl,
    checkIsCategoryList,
    checkIsExplicitBlankVote,
    checkIsWriteIn,
    checkIsInvalidVote,
    checkPositionIsTop,
    checkAllowWriteIns,
    checkCustomCandidatesOrder,
    checkShuffleCategories,
    checkShuffleCategoryList,
    getCheckableOptions,
    checkIsRadioSelection,
} from "@sequentech/ui-essentials"
