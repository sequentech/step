// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_SUPPORT_MATERIALS_ACKNOWLEDGMENT = gql`
    query GetSupportMaterialsAcknowledgment($electionEventId: uuid!) {
        get_support_materials_acknowledgment(election_event_id: $electionEventId) {
            document_ids
        }
    }
`
