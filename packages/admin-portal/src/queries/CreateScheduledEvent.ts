// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_SCHEDULED_EVENT = gql`
    mutation CreateScheduledEvent(
        $tenantId: String!
        $electionEventId: uuid
        $eventProcessor: String!
        $cronConfig: String
        $eventPayload: jsonb!
    ) {
        createScheduledEvent(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            event_processor: $eventProcessor
            cron_config: $cronConfig
            event_payload: $eventPayload
        ) {
            id
        }
    }
`
