import {GET_AREA_WITH_AREA_CONTESTS} from "@/queries/GetAreaWithAreaContest"
import {useQuery} from "@apollo/client"
import {Chip} from "@mui/material"
import { translateElection } from '@sequentech/ui-essentials'
import React from "react"
import {Identifier, RaRecord} from "react-admin"
import { useTranslation } from 'react-i18next'

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
    const {i18n} = useTranslation()
    const {data} = useQuery(GET_AREA_WITH_AREA_CONTESTS, {
        variables: {
            electionEventId: record.election_event_id,
            areaId: record.id,
        },
    })

    return (
        <>
            {data && data.sequent_backend_area_contest
                ? data.sequent_backend_area_contest.map((item: any, index: number) => (
                      <Chip key={index} label={translateElection(item?.contest, "name", i18n.language)} />
                  ))
                : null}
        </>
    )
}
