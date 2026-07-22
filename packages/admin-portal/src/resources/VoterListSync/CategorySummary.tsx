// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Stack, Typography} from "@mui/material"
import {ESyncChangeCategory} from "./types"
import {CATEGORY_LABELS} from "./constants"

interface CategorySummaryProps {
    /** Backend-computed per-category counts (the import row's `summary`
     * column) — never re-derived from a paginated page of items, which would
     * silently undercount once the item grids are server-paginated. */
    counts: Partial<Record<ESyncChangeCategory, number>>
    highlighted?: Set<ESyncChangeCategory>
}

/** Renders the per-category counts, e.g. "marks N voters as voted via other
 * channels, disables M voters, updates K profiles, adds J voters" from the
 * reconciliation confirmation dialog acceptance criteria. */
export const CategorySummary: React.FC<CategorySummaryProps> = ({counts, highlighted}) => {
    const entries = Object.entries(counts).filter(([, count]) => (count ?? 0) > 0) as Array<
        [ESyncChangeCategory, number]
    >

    return (
        <Stack direction="row" gap={1} flexWrap="wrap">
            {entries.map(([category, count]) => (
                <Box
                    key={category}
                    sx={{
                        border: "1px solid",
                        borderColor: highlighted?.has(category) ? "warning.main" : "divider",
                        borderRadius: 1,
                        px: 1.5,
                        py: 1,
                    }}
                >
                    <Typography variant="caption" color="text.secondary">
                        {CATEGORY_LABELS[category]}
                    </Typography>
                    <Typography variant="h6">{count}</Typography>
                </Box>
            ))}
        </Stack>
    )
}

export default CategorySummary
