// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Popover} from "@mui/material"
import {Icon} from "@sequentech/ui-essentials"
import {IconDefinition} from "@fortawesome/fontawesome-svg-core"

interface IconTooltipProps {
    icon: IconDefinition
    info: string
    infoContainerWidth?: string
}

const IconTooltip = ({icon, info, infoContainerWidth = "250px"}: IconTooltipProps) => {
    const [anchorEl, setAnchorEl] = React.useState<HTMLElement | null>(null)

    const handlePopoverOpen = (event: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(event.currentTarget)
    }

    const handlePopoverClose = () => {
        setAnchorEl(null)
    }

    const open = Boolean(anchorEl)

    return (
        <>
            <Box
                aria-owns={open ? "mouse-over-popover" : undefined}
                onMouseEnter={handlePopoverOpen}
                onMouseLeave={handlePopoverClose}
            >
                <Icon icon={icon} />
            </Box>
            <Popover
                id="mouse-over-popover"
                sx={{
                    "pointerEvents": "none",
                    "& .MuiPopover-paper": {
                        width: infoContainerWidth,
                        padding: "6px",
                    },
                }}
                open={open}
                anchorEl={anchorEl}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "left",
                }}
                transformOrigin={{
                    vertical: "top",
                    horizontal: "left",
                }}
                onClose={handlePopoverClose}
                disableRestoreFocus
            >
                <Box component="span">{info}</Box>
            </Popover>
        </>
    )
}

export default IconTooltip
