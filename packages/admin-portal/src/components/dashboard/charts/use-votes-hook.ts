// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import {GetCastVotesQuery} from "@/gql/graphql"
import {
    GET_CAST_VOTES,
    GET_CAST_VOTES_BY_DATERANGE,
    GET_CAST_VOTES_FOR_ELECTION,
} from "@/queries/GetCastVotes"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useContext} from "react"

export function useVotesHook({
    electionEventId,
    electionId,
    startDate,
    endDate,
}: {
    electionEventId: string
    electionId?: string
    startDate?: Date
    endDate?: Date
}) {
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const hasDateRange = !!startDate && !!endDate
    const query = hasDateRange
        ? GET_CAST_VOTES_BY_DATERANGE
        : electionId
        ? GET_CAST_VOTES_FOR_ELECTION
        : GET_CAST_VOTES
    const variables = hasDateRange
        ? {
              electionEventId,
              tenantId,
              startDate: startDate?.toISOString() ?? null,
              endDate: endDate?.toISOString() ?? null,
          }
        : electionId
        ? {
              electionEventId,
              electionId,
              tenantId,
          }
        : {
              electionEventId,
              tenantId,
          }

    return useQuery<GetCastVotesQuery>(query, {
        variables,
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })
}
