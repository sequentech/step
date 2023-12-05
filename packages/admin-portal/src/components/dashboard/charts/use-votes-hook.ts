import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import {GetCastVotesQuery} from "@/gql/graphql"
import {
    GET_CAST_VOTES,
    GET_CAST_VOTES_BY_DATERANGE,
    GET_CAST_VOTES_FOR_ELECTION,
} from "@/queries/GetCastVotes"

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
        pollInterval: 500,
    })
}
