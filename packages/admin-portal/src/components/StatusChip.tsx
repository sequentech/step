import {GET_AREA_WITH_AREA_CONTESTS} from "@/queries/GetAreaWithAreaContest"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"
import {Chip, IconButton} from "@mui/material"
import {adminTheme} from "@sequentech/ui-essentials"
import React, {useEffect} from "react"
import {Identifier, RaRecord, useGetList, useRecordContext} from "react-admin"

interface TrusteeItemsProps {
    status: string
}

const StyledChips = styled.div`
    display: flex;
    padding: 1px 7px;
    flex-direction: row;
    align-items: flex-start;
    gap: 4px;
`

const StyledChip = styled.div`
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 4px;
    background: ${(props: TrusteeItemsProps) =>
            props.status === ITallyExecutionStatus.STARTED
            ? "#d32f2f"
            : props.status === ITallyExecutionStatus.CONNECTED
            ? "#43E3A1"
            : props.status === ITallyExecutionStatus.IN_PROGRESS
            ? "#43E3A1"
            : props.status === ITallyExecutionStatus.SUCCESS
            ? "#43E3A1"
            : props.status === ITallyExecutionStatus.CANCELLED
            ? "#43E3A1"
            : "#d32f2f"};
    padding: 1px 7px;
`

const StyledChipLabel = styled.div`
    color: #fff;
    font-family: Roboto;
    font-size: 12px;
    font-style: normal;
    font-weight: 400;
    line-height: 18px;
`

export const StatusChip: React.FC<TrusteeItemsProps> = (props) => {
    const {status} = props

    return (
        <StyledChips>
            <StyledChip status={status}>
                <StyledChipLabel>{status?.length ? status.toUpperCase() : "-"}</StyledChipLabel>
            </StyledChip>
        </StyledChips>
    )
}
