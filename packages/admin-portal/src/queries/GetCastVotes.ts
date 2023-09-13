// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_CAST_VOTES = gql`
    query GetCastVotes(
        $electionEventId: uuid
        $tenantId: uuid
        $startDate: timestamptz!
        $endDate: timestamptz!
    ) {
        sequent_backend_cast_vote(
            where: {
                created_at: {_gte: $startDate, _lte: $endDate}
                election_event_id: {_eq: $electionEventId}
                tenant_id: {_eq: $tenantId}
            }
        ) {
            id
            tenant_id
            election_id
            area_id
            created_at
            last_updated_at
            election_event_id
        }
    }
`
