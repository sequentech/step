// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GET_AREA_WITH_AREA_CONTESTS} from "@/queries/GetAreaWithAreaContest"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"
import {Chip, IconButton} from "@mui/material"
import {adminTheme} from "@sequentech/ui-essentials"
import React, {useEffect} from "react"
import {Identifier, RaRecord, useGetList, useGetOne, useRecordContext} from "react-admin"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define teh actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

interface ContestItemProps {
    record: RaRecord<Identifier>
}

export const ContestItem: React.FC<ContestItemProps> = (props) => {
    const {record} = props

    const {data} = useGetOne("sequent_backend_contest", {id: record})

    return <>{data ? <Chip label={data?.name} /> : null}</>
}
