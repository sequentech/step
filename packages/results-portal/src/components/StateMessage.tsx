// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Button, Stack, Typography} from "@mui/material"

interface StateMessageProps {
    className?: string
    title: string
    message: string
    actionLabel?: string
    onAction?: () => void
}

export const StateMessage: React.FC<StateMessageProps> = ({
    className,
    title,
    message,
    actionLabel,
    onAction,
}) => (
    <Box
        className={["seq-results-state-message", className].filter(Boolean).join(" ")}
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
        <Stack className="seq-results-state-message__content" spacing={2} alignItems="flex-start">
            <Typography className="seq-results-state-message__title" component="h1" variant="h4">
                {title}
            </Typography>
            <Typography className="seq-results-state-message__message" color="text.secondary">
                {message}
            </Typography>
            {actionLabel && onAction && (
                <Button
                    className="seq-results-state-message__action"
                    variant="contained"
                    onClick={onAction}
                >
                    {actionLabel}
                </Button>
            )}
        </Stack>
    </Box>
)
