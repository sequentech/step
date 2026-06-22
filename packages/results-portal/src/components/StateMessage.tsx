// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Button, Stack, Typography} from "@mui/material"

interface StateMessageProps {
    title: string
    message: string
    actionLabel?: string
    onAction?: () => void
}

export const StateMessage: React.FC<StateMessageProps> = ({
    title,
    message,
    actionLabel,
    onAction,
}) => (
    <Box
        sx={{
            width: "100%",
            maxWidth: 760,
            mx: "auto",
            my: {xs: 4, md: 8},
            px: {xs: 2, sm: 3},
            py: {xs: 4, sm: 5},
            border: "1px solid",
            borderColor: "divider",
            borderRadius: 1,
            bgcolor: "background.paper",
        }}
    >
        <Stack spacing={2} alignItems="flex-start">
            <Typography component="h1" variant="h4">
                {title}
            </Typography>
            <Typography color="text.secondary">{message}</Typography>
            {actionLabel && onAction && (
                <Button variant="contained" onClick={onAction}>
                    {actionLabel}
                </Button>
            )}
        </Stack>
    </Box>
)
