import {gql} from "@apollo/client"

export const INSERT_CAST_VOTE = gql`
mutation InsertCastVote($id: uuid, $electionId: uuid, $electionEventId: uuid, $tenantId: uuid, $voterIdString: String!, $content: String!) {
    insert_sequent_backend_cast_vote(objects: {
        id: $id,
        election_id: $electionId,
        election_event_id: $electionEventId,
        tenant_id: $tenantId,
        voter_id_string: $voterIdString,
        content: $content
    }) {
        returning {
            id
            election_id
            election_event_id
            tenant_id
            voter_id_string
        }
    }
  }
`