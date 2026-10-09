// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTIONS = gql`
    query GetElections($electionIds: [uuid!]!) {
        sequent_backend_election(where: {id: {_in: $electionIds}}) {
            created_at
            description
            election_event_id
            eml
            id
            is_consolidated_ballot_encoding
            labels
            last_updated_at
            name
            num_allowed_revotes
            presentation
            spoil_ballot_option
            status
            tenant_id
            alias
        }
    }
`
