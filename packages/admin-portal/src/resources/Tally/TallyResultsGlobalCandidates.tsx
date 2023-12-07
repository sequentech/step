// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Results_Area_Contest_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface TallyResultsGlobalCandidatesProps {
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
}

export const TallyResultsGlobalCandidates: React.FC<TallyResultsGlobalCandidatesProps> = (
    props
) => {
    const {contestId, electionId, electionEventId, tenantId} = props
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Candidate>>([])

    const {data: election} = useGetOne("sequent_backend_election", {
        id: electionId,
        meta: {tenant_id: tenantId},
    })

    const {data: candidates} = useGetList<Sequent_Backend_Candidate>("sequent_backend_candidate", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            contest_id: contestId,
            tenant_id: tenantId,
            election_event_id: election?.election_event_id,
        },
    })

    const {data: general} = useGetList<Sequent_Backend_Results_Contest>(
        "sequent_backend_results_contest",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    const {data: results} = useGetList<Sequent_Backend_Results_Contest_Candidate>(
        "sequent_backend_results_contest_candidate",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    useEffect(() => {
        if (election) {
            setContestsData(candidates || [])
        }
    }, [election, candidates])

    return (
        <>
            {contestsData?.map((candidate, index) => (
                <>
                    <div key={index}>{candidate.name}</div>
                </>
            ))}
        </>
    )
}
