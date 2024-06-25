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
            alias
            presentation
            is_archived
            elections {
                id
                name
                alias
                presentation
                election_event_id
                image_document_id
                contests {
                    id
                    name
                    alias
                    presentation
                    election_event_id
                    election_id
                    created_at
                    candidates {
                        id
                        name
                        alias
                        contest_id
                        election_event_id
                        presentation
                        image_document_id
                    }
                    image_document_id
                }
            }
        }
    }
`
