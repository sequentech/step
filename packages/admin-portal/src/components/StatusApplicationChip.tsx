// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {IApplicationsStatus} from "@/types/applications"
import styled from "@emotion/styled"
import {statusColor} from "@/resources/Tally/constants"

interface TrusteeItemsProps {
    status: string
}

const StyledChips = styled.div`
    display: flex;
    padding: 1px 7px;
    flex-direction: row;
    align-items: center;
    gap: 4px;
`

const StyledChip = styled.div`
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 4px;
    background: ${(props: TrusteeItemsProps) =>
        statusColor(props.status ?? IApplicationsStatus.PENDING)};
    padding: 1px 7px;
`

const StyledChipLabel = styled.div`
    color: #fff;
    font-family: Roboto;
    font-size: 12px;
    font-style: normal;
    font-weight: 500;
    line-height: 18px;
`

export const StatusApplicationChip: React.FC<TrusteeItemsProps> = (props) => {
    const {status} = props

    return (
        <StyledChips>
            <StyledChip status={status}>
                <StyledChipLabel>{status?.length ? status.toUpperCase() : "-"}</StyledChipLabel>
            </StyledChip>
        </StyledChips>
    )
}
