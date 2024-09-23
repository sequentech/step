import {gql} from "@apollo/client"

export const GET_EVENT_LIST = gql`
    mutation GetEventList($tenantId: String!, $electionEventId: String!) {
        get_event_list(tenant_id: $tenantId, election_event_id: $electionEventId) {
            election
            schedule
            task_id
            tenant_id
            election_event_id
            event_type
            receivers
            template
            name
        }
    }
`
