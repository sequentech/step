// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, styled, SvgIconTypeMap, Typography, Icon} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {IconDefinition} from "@fortawesome/free-solid-svg-icons"
import {OverridableComponent} from "@mui/material/OverridableComponent"
import FenceIcon from "@mui/icons-material/Fence"
import DeleteIcon from "@mui/icons-material/Delete"

const CardContainer = styled(Box)<{selected?: boolean}>`
    display: flex;
    flex-direction: column;
    padding: 16px;
    border-radius: 4px;
    border: 1px solid ${theme.palette.customGrey.light};
    color: ${({selected}) => (selected ? theme.palette.white : theme.palette.customGrey.main)};
    justify-content: center;
    text-align: center;
    width: 160px;
    height: 140px;
    ${({selected}) =>
        selected ? "background: linear-gradient(180deg, #0FADCF 0%, #0F054B 100%); " : ""}
`

const StyledTypography1 = styled(Typography)<{uppercase?: string}>`
    text-align: center;
    margin-top: 12px;
    margin-bottom: 0;
    ${({uppercase}) => uppercase && "text-transform: uppercase;"}
`

const StyledTypography2 = styled(Typography)<{uppercase?: string}>`
    text-align: center;
    margin-top: 0;
    margin-bottom: 0;
    ${({uppercase}) => uppercase && "text-transform: uppercase;"}
`

export default function StatItem({
    icon,
    count,
    label,
}: {
    icon: any
    count: number | string
    label: string
}) {
    const iconSize = 60

    return (
        <CardContainer>
            <Icon sx={{width: iconSize, height: iconSize, textAlign: "center", marginX: "auto"}}>
                {icon}
            </Icon>
            <StyledTypography1 fontSize="24px">{count}</StyledTypography1>
            <StyledTypography2 fontSize="12px" uppercase="true">
                {label}
            </StyledTypography2>
        </CardContainer>
    )
}
