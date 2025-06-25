// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import styled from "@emotion/styled"
import {statusColor} from "@/resources/Tally/constants"
import {Chip} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"

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

export const StatusChip: React.FC<TrusteeItemsProps> = (props) => {
    const {status} = props

    return (
        <StyledChips>
            <Chip
                sx={{
                    backgroundColor: statusColor(props.status ?? ITallyExecutionStatus.STARTED),
                    color: theme.palette.background.default,
                }}
                label={status?.length ? status.toUpperCase() : "-"}
            />
        </StyledChips>
    )
}
