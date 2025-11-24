// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Chip} from "@mui/material"
import React from "react"
import {Identifier, RaRecord, useGetOne} from "react-admin"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define teh actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

interface AreaItemProps {
    record: RaRecord<Identifier>
}

export const AreaItem: React.FC<AreaItemProps> = (props) => {
    const {record} = props

    const {data} = useGetOne("sequent_backend_area", {id: record})

    return <>{data ? <Chip label={data?.name} /> : null}</>
}
