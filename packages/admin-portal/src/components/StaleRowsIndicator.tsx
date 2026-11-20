// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactNode} from "react"
import {useListContext} from "react-admin"
import {Box, LinearProgress} from "@mui/material"

const LOADING_BAR_EDGES = ["top", "bottom"] as const

/**
 * While another page, filter or sort loads, a list keeps showing the previous rows: dimmed rows
 * and progress bars show that they are stale. The rows can be taller than the screen, so there
 * is a bar by the filters and one by the pagination.
 */
export const StaleRowsIndicator: React.FC<{children: ReactNode}> = ({children}) => {
    const {isPlaceholderData} = useListContext()
    return (
        <Box sx={{position: "relative"}} aria-busy={!!isPlaceholderData}>
            {isPlaceholderData &&
                LOADING_BAR_EDGES.map((edge) => (
                    <LinearProgress
                        key={edge}
                        className="stale-rows-loading"
                        sx={{position: "absolute", [edge]: 0, left: 0, right: 0, zIndex: 1}}
                    />
                ))}
            <Box sx={{opacity: isPlaceholderData ? 0.4 : 1, transition: "opacity 150ms"}}>
                {children}
            </Box>
        </Box>
    )
}
