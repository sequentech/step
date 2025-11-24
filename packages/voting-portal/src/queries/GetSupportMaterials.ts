// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_SUPPORT_MATERIALS = gql`
    query GetSupportMaterials($electionEventId: uuid!, $tenantId: uuid!) {
        sequent_backend_support_material(
            where: {
                _and: {
                    is_hidden: {_eq: false}
                    election_event_id: {_eq: $electionEventId}
                    tenant_id: {_eq: $tenantId}
                }
            }
        ) {
            data
            document_id
            id
            annotations
            created_at
            election_event_id
            kind
            labels
            last_updated_at
            tenant_id
        }
    }
`
