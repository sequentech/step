// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Paper, Typography} from "@mui/material"
import ReactApexChart, {Props as ApexChartProps} from "react-apexcharts"

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
            maxWidth: {xs: "100%", md: 450},
        }}
    >
        <Typography
            className="seq-tally-results-chart-panel__title"
            variant="subtitle1"
            sx={{fontWeight: 600}}
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
