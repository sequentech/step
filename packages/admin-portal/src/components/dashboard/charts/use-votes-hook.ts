import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import {GetCastVotesQuery} from "@/gql/graphql"
import {GET_CAST_VOTES, GET_CAST_VOTES_BY_DATERANGE} from "@/queries/GetCastVotes"

export function useVotesHook({
    electionEventId,
    startDate,
    endDate,
}: {
    electionEventId: string
    startDate?: Date
    endDate?: Date
}) {
    const [tenantId] = useTenantStore()
    const hasDateRange = !!startDate && !!endDate
    const query = hasDateRange ? GET_CAST_VOTES_BY_DATERANGE : GET_CAST_VOTES
    const variables = hasDateRange
        ? {
              electionEventId,
              tenantId,
              startDate: startDate?.toISOString() ?? null,
              endDate: endDate?.toISOString() ?? null,
          }
        : {
              electionEventId,
              tenantId,
          }

    return useQuery<GetCastVotesQuery>(query, {
        variables,
        pollInterval: 500,
    })
}
