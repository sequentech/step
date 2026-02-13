// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {electionStatusColor} from "@/resources/Tally/constants"
import {ITallyElectionStatus} from "@/types/ceremonies"
import {styled} from "@mui/material/styles"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define the actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

interface ElectionStatusItemProps {
    name: ITallyElectionStatus | undefined
}

const StyledChips = styled("div")`
    display: flex;
    padding: 1px 7px;
    flex-direction: row;
    align-items: flex-start;
    gap: 4px;
`

const StyledChip = styled("div")`
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 4px;
    background: ${(props: {name: string | undefined}) =>
        electionStatusColor(props.name ?? ITallyElectionStatus.WAITING)};
    padding: 1px 7px;
`

const StyledChipLabel = styled("div")`
    color: #fff;
    font-family: Roboto;
    font-size: 12px;
    font-style: normal;
    font-weight: 400;
    line-height: 18px;
`

export const ElectionStatusItem: React.FC<ElectionStatusItemProps> = (props) => {
    const {name} = props

    return (
        <StyledChips>
            <StyledChip name={name}>
                <StyledChipLabel>{name ?? "-"}</StyledChipLabel>
            </StyledChip>
        </StyledChips>
    )
}
