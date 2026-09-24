// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useState, useRef} from "react"
import {useApolloClient, useQuery} from "@apollo/client/react"
import {useParams} from "react-router-dom"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_VOTER_STATUS} from "../queries/GetVoterStatus"
import {GET_ELECTIONS} from "../queries/GetElections"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GetBallotStylesQuery, GetCastVotesQuery} from "../gql/graphql"
import {
    loadPublicationList,
    loadSelectedBallot,
    PublicationDownloadError,
} from "../services/PublishedBallots"

type Loaded = Awaited<ReturnType<typeof loadPublicationList>> &
    GetBallotStylesQuery &
    GetCastVotesQuery

export function useVoterContext(selectedElectionId?: string, skip = false) {
    const {tenantId, eventId} = useParams<{tenantId: string; eventId: string}>()
    const {globalSettings} = useContext(SettingsContext)
    const client = useApolloClient()
    const result = useQuery(GET_VOTER_STATUS, {
        variables: {electionEventId: eventId || ""},
        skip: skip || globalSettings.DISABLE_AUTH || !tenantId || !eventId,
    })
    const [loaded, setLoaded] = useState<{
        source: typeof result.data
        selection?: string
        data: Loaded
    }>()
    const renewed = useRef(false)
    useEffect(() => {
        renewed.current = false
    }, [client, tenantId, eventId, selectedElectionId])
    const [downloadError, setDownloadError] = useState<Error>()
    useEffect(() => {
        if (!result.data || skip || globalSettings.DISABLE_AUTH) return
        let active = true
        setDownloadError(undefined)
        const load = async () => {
            let response: typeof result.data | undefined = result.data
            for (let attempt = 0; attempt < 2; attempt++) {
                try {
                    if (!response) return
                    const refs = response.get_ballot_files_urls
                    if (refs.event_id !== eventId) throw new Error("Published event scope mismatch")
                    const list = await loadPublicationList(client, refs, selectedElectionId)
                    const styles = selectedElectionId
                        ? [await loadSelectedBallot(client, refs, selectedElectionId)]
                        : []
                    if (!active) return
                    client.writeQuery({
                        query: GET_ELECTION_EVENT,
                        variables: {tenantId, electionEventId: eventId},
                        data: list,
                    })
                    for (const election of list.sequent_backend_election) {
                        client.writeQuery({
                            query: GET_ELECTIONS,
                            variables: {electionIds: [election.id]},
                            data: {sequent_backend_election: [election]},
                        })
                    }
                    client.writeQuery({
                        query: GET_ELECTIONS,
                        variables: {electionIds: list.sequent_backend_election.map((e) => e.id)},
                        data: list,
                    })
                    setLoaded({
                        source: response,
                        selection: selectedElectionId,
                        data: {
                            ...list,
                            sequent_backend_ballot_style: styles,
                            sequent_backend_cast_vote: response.sequent_backend_cast_vote,
                        },
                    })
                    return
                } catch (error) {
                    if (!active) return
                    if (
                        !renewed.current &&
                        attempt === 0 &&
                        error instanceof PublicationDownloadError &&
                        [401, 403].includes(error.status)
                    ) {
                        renewed.current = true
                        response = (await result.refetch()).data
                    } else {
                        throw error
                    }
                }
            }
        }
        void load().catch(() => {
            if (active) setDownloadError(new Error("Unable to load published ballot data"))
        })
        return () => {
            active = false
        }
    }, [
        client,
        result.data,
        selectedElectionId,
        tenantId,
        eventId,
        skip,
        globalSettings.DISABLE_AUTH,
    ])
    const data =
        loaded?.source === result.data && loaded?.selection === selectedElectionId
            ? loaded?.data
            : undefined
    return {
        data,
        elections: data,
        summaries: data?.summaries,
        error: result.error ?? downloadError,
        loading:
            !skip &&
            !globalSettings.DISABLE_AUTH &&
            !downloadError &&
            (result.loading || (!!result.data && !data)),
        refetch: result.refetch,
    }
}
