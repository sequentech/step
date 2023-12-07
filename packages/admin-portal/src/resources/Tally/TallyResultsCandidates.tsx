// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface TallyResultsCandidatesProps {
    contestId: string
    electionId: string
}

export const TallyResultsCandidates: React.FC<TallyResultsCandidatesProps> = (props) => {
    const {contestId, electionId} = props
    const [tenantId] = useTenantStore()
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Candidate>>([])

    const {data: election} = useGetOne("sequent_backend_election", {
        id: electionId,
        meta: {tenant_id: tenantId},
    })

    const {data: candidates} = useGetList<Sequent_Backend_Candidate>("sequent_backend_candidate", {
        filter: {contest_id: contestId, tenant_id: tenantId, election_event_id: election?.election_event_id},
    })

    useEffect(() => {
        if (election) {
            setContestsData(candidates || [])
        }
    }, [election, candidates])


    return (
        <>
            {contestsData?.map((candidate, index) => (
                <div key={index}>{candidate.name}</div>
            ))}
        </>
    )
}
