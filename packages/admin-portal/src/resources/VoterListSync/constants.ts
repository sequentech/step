// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ESyncChangeCategory} from "./types"

export const CATEGORY_COLORS: Record<
    ESyncChangeCategory,
    "default" | "success" | "warning" | "error" | "info"
> = {
    [ESyncChangeCategory.VOTED_INTERNET]: "info",
    [ESyncChangeCategory.VOTED_OTHER_CHANNEL]: "warning",
    [ESyncChangeCategory.DISABLED_DELETE_CALL]: "warning",
    [ESyncChangeCategory.DELETION_REVERTED]: "info",
    [ESyncChangeCategory.PROFILE_UPDATE]: "default",
    [ESyncChangeCategory.VOTER_ADDED]: "success",
    [ESyncChangeCategory.REENABLED]: "success",
    [ESyncChangeCategory.VOTED_UNMARKED]: "warning",
    [ESyncChangeCategory.ROW_FAILURE]: "error",
}

// Acceptance criteria: the apply-patch confirmation dialog highlights the
// categories that touch voted status or disable voters.
export const HIGHLIGHTED_CATEGORIES = new Set<ESyncChangeCategory>([
    ESyncChangeCategory.VOTED_INTERNET,
    ESyncChangeCategory.VOTED_OTHER_CHANNEL,
    ESyncChangeCategory.VOTED_UNMARKED,
    ESyncChangeCategory.DISABLED_DELETE_CALL,
])
