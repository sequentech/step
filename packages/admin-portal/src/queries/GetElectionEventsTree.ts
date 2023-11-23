// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const FETCH_ELECTION_EVENTS_TREE = gql`
    query election_events_tree($tenantId: uuid!, $isArchived: Boolean!) {
        sequent_backend_election_event(
            where: {is_archived: {_eq: $isArchived}, _and: {tenant_id: {_eq: $tenantId}}}
        ) {
            id
            name
            is_archived
            elections {
                id
                name
                contests {
                    id
                    name
                    candidates {
                        id
                        name
                    }
                }
            }
        }
    }
`
