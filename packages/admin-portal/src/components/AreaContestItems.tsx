// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {GET_AREA_WITH_AREA_CONTESTS} from "@/queries/GetAreaWithAreaContest"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"
import {Chip, IconButton} from "@mui/material"
import {adminTheme} from "@sequentech/ui-essentials"
import React, {useEffect} from "react"
import {Identifier, RaRecord, useGetList, useRecordContext} from "react-admin"

/*  
        In the component where you want to use the actions column:
        
        - define the functions and the actions custom column to be showned
        - define teh actions array with the actions to be showned
        - add the ActionsColumn as a column to the list as the final column as a normal one

        Format: {icon: <Icon />, action: (id: Identifier) => void}
    */

interface AreaContestItemsProps {
    record: RaRecord<Identifier>
}

export const AreaContestItems: React.FC<AreaContestItemsProps> = (props) => {
    const {record} = props
    const {data} = useQuery(GET_AREA_WITH_AREA_CONTESTS, {
        variables: {
            electionEventId: record.election_event_id,
            areaId: record.id,
        },
    })

    const aliasRenderer = useAliasRenderer()

    return (
        <>
            {data && data.sequent_backend_area_contest
                ? data.sequent_backend_area_contest.map((item: any, index: number) => (
                      <Chip key={index} label={aliasRenderer(item?.contest)} />
                  ))
                : null}
        </>
    )
}
