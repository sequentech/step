import React from "react"
import {Box, styled, Typography} from "@mui/material"
import {IconButton, theme} from "@sequentech/ui-essentials"
import {IconDefinition} from "@fortawesome/free-solid-svg-icons"

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

const StyledTypography1 = styled(Typography)<{uppercase?: boolean}>`
    text-align: center;
    margin-top: 12px;
    margin-bottom: 0;
    ${({uppercase}) => uppercase && "text-transform: uppercase;"}
`

const StyledTypography2 = styled(Typography)<{uppercase?: boolean}>`
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
    icon: IconDefinition
    count: number | string
    label: string
}) {
    return (
        <CardContainer>
            <IconButton icon={icon} fontSize="38px" />
            <StyledTypography1 fontSize="24px">{count}</StyledTypography1>
            <StyledTypography2 fontSize="12px" uppercase={true}>
                {label}
            </StyledTypography2>
        </CardContainer>
    )
}
