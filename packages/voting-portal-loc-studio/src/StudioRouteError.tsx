// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {isRouteErrorResponse, useRouteError} from "react-router-dom"

export const StudioRouteError: React.FC = () => {
    const error = useRouteError()
    const message = isRouteErrorResponse(error)
        ? error.statusText || String(error.data)
        : error instanceof Error
          ? error.message
          : "Unknown preview error"

    return (
        <Box className="loc-studio-preview-error">
            <Typography className="loc-studio-preview-error-title">
                Preview could not render this screen
            </Typography>
            <Typography className="loc-studio-preview-error-message">{message}</Typography>
            <Typography className="loc-studio-help">
                Try another screen in the left panel. If this persists, re-import with publications
                enabled so ballot contests are included.
            </Typography>
        </Box>
    )
}
