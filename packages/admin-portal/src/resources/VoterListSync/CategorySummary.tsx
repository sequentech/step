// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {Box, Stack, Typography} from "@mui/material"
import {ESyncChangeCategory, SyncDiffRow} from "./types"
import {CATEGORY_LABELS} from "./constants"

interface CategorySummaryProps {
    rows: SyncDiffRow[]
    highlighted?: Set<ESyncChangeCategory>
}

/** Aggregate per-category counts, e.g. "marks N voters as voted via other
 * channels, disables M voters, updates K profiles, adds J voters" from the
 * reconciliation confirmation dialog acceptance criteria. */
export const CategorySummary: React.FC<CategorySummaryProps> = ({rows, highlighted}) => {
    const counts = useMemo(() => {
        const byCategory = new Map<ESyncChangeCategory, number>()
        rows.forEach((row) => byCategory.set(row.category, (byCategory.get(row.category) ?? 0) + 1))
        return Array.from(byCategory.entries())
    }, [rows])

    return (
        <Stack direction="row" gap={1} flexWrap="wrap">
            {counts.map(([category, count]) => (
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
