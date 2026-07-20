// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ESyncChangeCategory} from "./types"

export const CATEGORY_LABELS: Record<ESyncChangeCategory, string> = {
    [ESyncChangeCategory.VOTED_INTERNET]: "Voted via Internet",
    [ESyncChangeCategory.VOTED_OTHER_CHANNEL]: "Voted via other channel",
    [ESyncChangeCategory.DISABLED]: "Voter disabled",
    [ESyncChangeCategory.DELETION_REVERTED]: "Deletion reverted",
    [ESyncChangeCategory.PROFILE_UPDATE]: "Profile update",
    [ESyncChangeCategory.VOTER_ADDED]: "Voter added",
    [ESyncChangeCategory.ROW_FAILURE]: "Row failure",
}

export const CATEGORY_COLORS: Record<
    ESyncChangeCategory,
    "default" | "success" | "warning" | "error" | "info"
> = {
    [ESyncChangeCategory.VOTED_INTERNET]: "info",
    [ESyncChangeCategory.VOTED_OTHER_CHANNEL]: "warning",
    [ESyncChangeCategory.DISABLED]: "warning",
    [ESyncChangeCategory.DELETION_REVERTED]: "info",
    [ESyncChangeCategory.PROFILE_UPDATE]: "default",
    [ESyncChangeCategory.VOTER_ADDED]: "success",
    [ESyncChangeCategory.ROW_FAILURE]: "error",
}

// Acceptance criteria: the apply-patch confirmation dialog highlights the
// categories that touch voted status or disable voters.
export const HIGHLIGHTED_CATEGORIES = new Set<ESyncChangeCategory>([
    ESyncChangeCategory.VOTED_INTERNET,
    ESyncChangeCategory.VOTED_OTHER_CHANNEL,
    ESyncChangeCategory.DISABLED,
])

// MOCK: election events don't carry a CountyMun field yet. Hardcoded to match
// the sample reconciliation/patch files so the row-failure demo path lines up.
export const MOCK_ELECTION_EVENT_COUNTY_MUN = "0014"
