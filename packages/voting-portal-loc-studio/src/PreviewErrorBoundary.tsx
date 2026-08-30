// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"

interface PreviewErrorBoundaryProps {
    resetKey: string | number
    children: React.ReactNode
}

interface PreviewErrorBoundaryState {
    error: Error | null
}

export class PreviewErrorBoundary extends React.Component<
    PreviewErrorBoundaryProps,
    PreviewErrorBoundaryState
> {
    state: PreviewErrorBoundaryState = {error: null}

    static getDerivedStateFromError(error: Error): PreviewErrorBoundaryState {
        return {error}
    }

    componentDidUpdate(prevProps: PreviewErrorBoundaryProps): void {
        if (prevProps.resetKey !== this.props.resetKey && this.state.error) {
            this.setState({error: null})
        }
    }

    render(): React.ReactNode {
        if (this.state.error) {
            return (
                <Box className="loc-studio-preview-error">
                    <Typography className="loc-studio-preview-error-title">
                        Preview could not render this screen
                    </Typography>
                    <Typography className="loc-studio-preview-error-message">
                        {this.state.error.message}
                    </Typography>
                    <Typography className="loc-studio-help">
                        Try another screen in the left panel, or upload a publications export that
                        includes published ballot styles.
                    </Typography>
                </Box>
            )
        }
        return this.props.children
    }
}
