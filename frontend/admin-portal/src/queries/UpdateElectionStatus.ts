// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_ELECTION_STATUS = gql`
    mutation UpdateElectionStatus(
        $electionId: uuid! = "f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
        $electionEventId: uuid! = "33f18502-a67c-4853-8333-a58630663559",
        $tenantId: uuid! = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        $status: jsonb! = {voting_status: "STARTED"}
    ) {
        update_sequent_backend_election(
            where: {
                id: {_eq: $electionId},
                election_event_id: {_eq: $electionEventId},
                tenant_id: {_eq: $tenantId}
            },
            _set: {
                status: $status
            }
        ) {
            returning {
                id
            }
        }
    }
`
