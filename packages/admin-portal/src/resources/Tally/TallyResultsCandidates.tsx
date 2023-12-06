// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList} from "react-admin"

import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface TallyResultsCandidatesProps {
    contestId: string
    tally: Sequent_Backend_Tally_Session | undefined
}

export const TallyResultsCandidates: React.FC<TallyResultsCandidatesProps> = (props) => {
    const {contestId, tally} = props
    const [tenantId] = useTenantStore()
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Candidate>>([])

    const {data: candidates} = useGetList<Sequent_Backend_Candidate>("sequent_backend_candidate", {
        filter: {contest_id: contestId, tenant_id: tenantId, election_event_id: tally?.election_event_id},
    })

    useEffect(() => {
        if (contestId && tally) {
            setContestsData(candidates || [])
        }
    }, [contestId, tally, candidates])


    return (
        <>
            {contestsData?.map((candidate, index) => (
                <div key={index}>{candidate.name}</div>
            ))}
        </>
    )
}
