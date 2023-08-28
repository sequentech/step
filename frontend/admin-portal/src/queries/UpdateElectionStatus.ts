// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_ELECTION_STATUS = gql`
    mutation UpdateElectionStatus(
        $electionId: uuid!
        $electionEventId: uuid!
        $tenantId: uuid!
        $status: jsonb!
    ) {
        update_sequent_backend_election(
            where: {
                id: {_eq: $electionId}
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
            }
            _set: {status: $status}
        ) {
            returning {
                id
            }
        }
    }
`
