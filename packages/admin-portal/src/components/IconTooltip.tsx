// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Tooltip} from "@mui/material"
import {Icon} from "@sequentech/ui-essentials"
import {IconDefinition} from "@fortawesome/fontawesome-svg-core"
import {theme} from "@sequentech/ui-essentials"
import {makeStyles} from "@mui/styles"

interface IconTooltipProps {
    icon: IconDefinition
    info: string
    infoContainerWidth?: string
}
const useStyles = makeStyles(() => ({
    tooltip: {
        backgroundColor: `${theme.palette.blue.light}`,
        color: "rgba(0, 0, 0)",
        width: "220px",
        fontSize: `${theme.typography.pxToRem(12)}`,
        padding: "16px",
        display: "flex",
        flexDirection: "column",
        gap: "8px",
    },
    arrow: {
        color: `${theme.palette.blue.light}`,
        fontSize: "20px",
        transform: "translate3d(190px, 0px, 0px) !important",
    },
}))

const IconTooltip = ({icon, info, infoContainerWidth = "250px"}: IconTooltipProps) => {
    const classes = useStyles()
    return (
        <Tooltip
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
                <Icon icon={icon} />
            </Box>
        </Tooltip>
    )
}

export default IconTooltip
