// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, styled} from "@mui/material"

/** Shared bounds for the standard dashboard and its supporting sections. */
export const StandardDashboardContent = styled(Box)({
    width: "100%",
    maxWidth: 1024,
    minWidth: 0,
    marginInline: "auto",
})

/** Charts share the available width and stack with a gap in narrower containers. */
export const StandardDashboardCharts = styled(Box)({
    display: "grid",
    gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 420px), 1fr))",
    gap: 16,
    minWidth: 0,
    marginTop: 16,
})
