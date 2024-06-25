// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GET_AREA_WITH_AREA_CONTESTS} from "@/queries/GetAreaWithAreaContest"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"
import {Chip, IconButton} from "@mui/material"
import {adminTheme} from "@sequentech/ui-essentials"
import React, {useEffect} from "react"
import {Identifier, RaRecord, useGetList, useRecordContext} from "react-admin"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define the actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

interface TrusteeItemsProps {
    record: RaRecord<Identifier>
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
    border-radius: 14px;
    background: #ebebeb;
    padding: 7px;
`

const StyledChipLabel = styled.div`
    font-family: Roboto;
    font-size: 12px;
    font-style: normal;
    font-weight: 400;
    line-height: 18px;
`

export const TrusteeItems: React.FC<TrusteeItemsProps> = (props) => {
    const {record} = props
    const {data} = useQuery(GET_TRUSTEES_NAMES, {
        variables: {
            tenantId: record.tenant_id,
        },
    })

    return (
        <StyledChips>
            {data && data.sequent_backend_trustee
                ? data.sequent_backend_trustee.map((item: any, index: number) => (
                      <StyledChip key={index}>
                          <StyledChipLabel>{item.name}</StyledChipLabel>
                      </StyledChip>
                  ))
                : null}
        </StyledChips>
    )
}
