// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_DOCUMENT = gql`
    query GetDocument($ids: [uuid!], $tenantId: uuid, $electionEventId: uuid) {
        sequent_backend_document(
            where: {
                _and: {
                    tenant_id: {_eq: $tenantId}
                    election_event_id: {_eq: $electionEventId}
                    id: {_in: $ids}
                }
            }
        ) {
            id
            tenant_id
            election_event_id
            name
            media_type
            size
            labels
            annotations
            created_at
            last_updated_at
            is_public
        }
    }
`
