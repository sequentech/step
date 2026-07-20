// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Box,
    Chip,
    LinearProgress,
    Stack,
    Typography,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ETaskExecutionStatus} from "@sequentech/ui-core"

interface MockTaskProgressProps {
    title: string
    status: ETaskExecutionStatus
    logs: string[]
}

const statusColor = (status: ETaskExecutionStatus): "default" | "warning" | "success" | "error" => {
    switch (status) {
        case ETaskExecutionStatus.SUCCESS:
            return "success"
        case ETaskExecutionStatus.FAILED:
            return "error"
        case ETaskExecutionStatus.IN_PROGRESS:
        case ETaskExecutionStatus.STARTED:
            return "warning"
        default:
            return "default"
    }
}

/**
 * Presentational stand-in for the real task progress Widget
 * (components/Widget.tsx), which polls a task_execution row over GraphQL.
 * This one is driven entirely by props so the mock task runner in
 * mockSyncEngine.ts (runMockTask) can push logs into it without a backend.
 * Swap for the real Widget + WidgetsContextProvider (addWidget/setWidgetTaskId)
 * once GENERATE_RECONCILIATION_PATCHES / APPLY_RECONCILIATION_PATCH exist as
 * real Celery tasks.
 */
export const MockTaskProgress: React.FC<MockTaskProgressProps> = ({title, status, logs}) => {
    const isRunning =
        status === ETaskExecutionStatus.STARTED || status === ETaskExecutionStatus.IN_PROGRESS

    return (
        <Accordion defaultExpanded>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Stack
                    direction="row"
                    spacing={2}
                    alignItems="center"
                    sx={{width: "100%", pr: 2}}
                    justifyContent="space-between"
                >
                    <Typography sx={{fontWeight: (theme) => theme.typography.fontWeightBold}}>
                        {title}
                    </Typography>
                    <Chip size="small" label={status} color={statusColor(status)} />
                </Stack>
            </AccordionSummary>
            <AccordionDetails>
                <Stack spacing={1}>
                    {isRunning && <LinearProgress />}
                    <Box component="ul" sx={{margin: 0, paddingLeft: "1.25rem", listStyle: "disc"}}>
                        {logs.map((log, index) => (
                            <Typography key={index} component="li" variant="body2">
                                {log}
                            </Typography>
                        ))}
                    </Box>
                </Stack>
            </AccordionDetails>
        </Accordion>
    )
}

export default MockTaskProgress
