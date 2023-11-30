import {useTenantStore} from "@/providers/TenantContextProvider"
import {FETCH_ELECTION_EVENTS_TREE} from "@/queries/GetElectionEventsTree"
import {useQuery} from "@apollo/client"

export function useTreeMenuData(isArchivedElectionEvents: boolean) {
    const [tenantId] = useTenantStore()

    return useQuery(FETCH_ELECTION_EVENTS_TREE, {
        variables: {
            tenantId: tenantId,
            isArchived: isArchivedElectionEvents,
        },
    })
}
