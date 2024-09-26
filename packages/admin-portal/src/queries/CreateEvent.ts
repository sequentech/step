// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_EVENT = gql`
    mutation CreateEvent(
        $id: String!
        $tenantId: String!
        $electionEventId: uuid
        $eventProcessor: String!
        $cronConfig: jsonb
        $eventPayload: jsonb!
    ) {
        create_event(
            id: $id
            tenant_id: $tenantId
            election_event_id: $electionEventId
            event_processor: $eventProcessor
            cron_config: $cronConfig
            event_payload: $eventPayload
        ) {
            tenant_id
            election_event_id
            event_processor
            cron_config
            event_payload
            created_by
        }
    }
`
