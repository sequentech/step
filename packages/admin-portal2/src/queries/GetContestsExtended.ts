// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_CONTESTS_EXTENDED = gql`
    query sequent_backend_contest_extended(
        $electionEventId: uuid!
        $contestId: uuid!
        $tenantId: uuid!
    ) {
        sequent_backend_area_contest(
            where: {
                _and: {
                    election_event_id: {_eq: $electionEventId}
                    contest_id: {_eq: $contestId}
                    tenant_id: {_eq: $tenantId}
                }
            }
        ) {
            area {
                id
                name
            }
        }
    }
`
