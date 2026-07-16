// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Paper, Typography} from "@mui/material"
import ReactApexChart, {Props as ApexChartProps} from "react-apexcharts"
import {TALLY_RESULTS_PIE_PANEL_WIDTH} from "./constants"

export const Chart = ((ReactApexChart as unknown as {default?: React.ComponentType<ApexChartProps>})
    .default ?? ReactApexChart) as React.ComponentType<ApexChartProps>

export const ChartPanel: React.FC<{
    title: string
    children: React.ReactNode
    className?: string
}> = ({title, children, className}) => (
    <Paper
        className={["seq-tally-results-chart-panel", className].filter(Boolean).join(" ")}
        variant="outlined"
        sx={{
            p: 2,
            width: "100%",
            maxWidth: {xs: "100%", md: TALLY_RESULTS_PIE_PANEL_WIDTH},
        }}
    >
        <Typography
            className="seq-tally-results-chart-panel__title"
            variant="body1"
            sx={{color: "grey.600", fontSize: "16px", fontWeight: 400}}
        >
            {title}
        </Typography>
        <Box
            className="seq-tally-results-chart-panel__body"
            sx={{borderTop: "1px solid", borderColor: "divider", mt: 2, pt: 2}}
        >
            {children}
        </Box>
    </Paper>
)
