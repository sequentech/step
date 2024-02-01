// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {tallyQueryData} from "@/atoms/tally-candidates"
import {GetTallyDataQuery} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GET_TALLY_DATA} from "@/queries/GetTallyData"
import {useQuery} from "@apollo/client"
import {useSetAtom} from "jotai"
import React, {useContext, useEffect} from "react"

export interface ResultsDataLoaderProps {
    resultsEventId: string
    electionEventId: string
}

export const ResultsDataLoader: React.FC<ResultsDataLoaderProps> = ({
    resultsEventId,
    electionEventId,
}) => {
    const [tenantId] = useTenantStore()
    const setTallyQueryData = useSetAtom(tallyQueryData)
    const {globalSettings} = useContext(SettingsContext)

    const {data: tallyData, startPolling} = useQuery<GetTallyDataQuery>(GET_TALLY_DATA, {
        variables: {
            resultsEventId,
            electionEventId,
            tenantId,
        },
    })

    useEffect(() => {
        setTallyQueryData(tallyData ?? null)
    }, [tallyData])

    useEffect(() => {
        startPolling(globalSettings.QUERY_POLL_INTERVAL_MS)
    }, [startPolling, globalSettings.QUERY_POLL_INTERVAL_MS])

    return <></>
}
