// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {Box, Tooltip} from "@mui/material"
import {Icon} from "@sequentech/ui-essentials"
import {IconDefinition} from "@fortawesome/fontawesome-svg-core"
import {theme} from "@sequentech/ui-essentials"
const PREFIX = "IconTooltip"

const classes = {
    tooltip: `${PREFIX}-tooltip`,
    arrow: `${PREFIX}-arrow`,
}

const StyledTooltip = styled(Tooltip)(() => ({
    [`& .${classes.tooltip}`]: {
        backgroundColor: `${theme.palette.blue.light}`,
        color: "rgba(0, 0, 0)",
        width: "220px",
        fontSize: `${theme.typography.pxToRem(12)}`,
        padding: "16px",
        display: "flex",
        flexDirection: "column",
        gap: "8px",
    },

    [`& .${classes.arrow}`]: {
        color: `${theme.palette.blue.light}`,
        fontSize: "20px",
        transform: "translate3d(190px, 0px, 0px) !important",
    },
}))

interface IconTooltipProps {
    icon: IconDefinition
    info: string
    infoContainerWidth?: string
}

const IconTooltip = ({icon, info, infoContainerWidth = "250px"}: IconTooltipProps) => {
    return (
        <StyledTooltip
            title={info}
            arrow
            placement="bottom-end"
            classes={{tooltip: classes.tooltip, arrow: classes.arrow}}
            slotProps={{
                popper: {
                    modifiers: [
                        {
                            name: "offset",
                            options: {
                                offset: [30, 0],
                            },
                        },
                    ],
                },
            }}
        >
            <Box sx={{width: "30px"}}>
                <Icon icon={icon as any} />
            </Box>
        </StyledTooltip>
    )
}

export default IconTooltip
