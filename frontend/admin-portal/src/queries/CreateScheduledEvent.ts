// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_SCHEDULED_EVENT = gql`
    mutation CreateScheduledEvent(
        $tenantId: String!,
        $electionEventId: String!,
        $eventProcessor: String!,
        $cronConfig: String,
        $eventPayload: jsonb!,
        $createdBy: String!
    ) {
        createScheduledEvent(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            event_processor: $eventProcessor
            cron_config: $cronConfig
            event_payload: $eventPayload
            created_by: $createdBy
        ) {
            id
            tenant_id
            election_event_id
            created_at
            stopped_at
            labels
            annotations
            event_processor
            cron_config
            event_payload
            created_by
        }
    }
`
