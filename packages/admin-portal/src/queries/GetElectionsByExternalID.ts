import {gql} from "@apollo/client"

export const GET_ELECTIONS_BY_EXTERNAL_ID = gql`
    query GetElectionsByExternalId($external_ids: [String!]!, $election_event_id: uuid) {
        sequent_backend_election(
            where: {
                external_id: {_in: $external_ids}
                _and: [{election_event_id: {_eq: $election_event_id}}]
            }
        ) {
            id
            external_id
            election_event_id
            presentation
        }
    }
`
