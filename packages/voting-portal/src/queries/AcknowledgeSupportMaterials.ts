// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const ACKNOWLEDGE_SUPPORT_MATERIALS = gql`
    mutation AcknowledgeSupportMaterials($electionEventId: uuid!, $documentIds: [String!]!) {
        acknowledge_support_materials(
            election_event_id: $electionEventId
            document_ids: $documentIds
        ) {
            election_event_id
            document_ids
        }
    }
`
