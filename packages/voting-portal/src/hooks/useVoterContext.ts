// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useMemo} from "react"
import {useApolloClient, useQuery} from "@apollo/client/react"
import {useParams} from "react-router-dom"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_VOTER_CONTEXT} from "../queries/GetVoterContext"
import {GET_ELECTIONS} from "../queries/GetElections"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GetElectionsQuery} from "../gql/graphql"

export function useVoterContext() {
    const {tenantId, eventId} = useParams<{tenantId: string; eventId: string}>()
    const {globalSettings} = useContext(SettingsContext)
    const client = useApolloClient()
    const result = useQuery(GET_VOTER_CONTEXT, {
        variables: {tenantId: tenantId || "", electionEventId: eventId || ""},
        skip: globalSettings.DISABLE_AUTH || !tenantId || !eventId,
    })
    const elections = useMemo<GetElectionsQuery | undefined>(() => {
        if (!result.data) return undefined
        const byId = new Map<string, GetElectionsQuery["sequent_backend_election"][number]>()
        for (const style of result.data.sequent_backend_ballot_style) {
            if (style.election) byId.set(style.election.id, style.election)
        }
        return {sequent_backend_election: Array.from(byId.values())}
    }, [result.data])

    // Existing direct-entry and gold-reauth queries remain valid. Seed their
    // exact cache entries so moving between screens reuses this response.
    useEffect(() => {
        if (!result.data || !elections) return
        client.writeQuery({
            query: GET_ELECTION_EVENT,
            variables: {tenantId, electionEventId: eventId},
            data: result.data,
        })
        const ids = result.data.sequent_backend_ballot_style.map((style) => style.election_id)
        client.writeQuery({query: GET_ELECTIONS, variables: {electionIds: ids}, data: elections})
        for (const election of elections.sequent_backend_election) {
            client.writeQuery({
                query: GET_ELECTIONS,
                variables: {electionIds: [election.id]},
                data: {sequent_backend_election: [election]},
            })
        }
    }, [client, result.data, elections, tenantId, eventId])

    return {...result, elections}
}
