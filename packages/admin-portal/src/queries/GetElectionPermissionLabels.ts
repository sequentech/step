import {gql} from "@apollo/client"

export const GET_ELECTION_UNIQUE_PERMISSION_LABLES = gql`
    query GetDistinctPermissionLabels($tenantId: uuid!) {
        sequent_backend_election(
            distinct_on: annotations
            where: {annotations: {_is_null: false}, tenant_id: {_eq: $tenantId}}
        ) {
            annotations
        }
    }
`
