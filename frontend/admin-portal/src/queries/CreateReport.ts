// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_REPORT = gql`
    mutation CreateReport(
        $template: String!
        $tenantId: String!
        $electionEventId: String!
        $name: String!
        $format: String!
    ) {
        renderReport(
            template: $template
            tenant_id: $tenantId
            election_event_id: $electionEventId
            name: $name
            format: $format
        ) {
            id
            election_event_id
            tenant_id
            name
            size
            media_type
        }
    }
`
