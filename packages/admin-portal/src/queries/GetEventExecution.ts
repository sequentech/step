// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_EVENT_EXECUTION = gql`
    query GetEventExecution($tenantId: uuid!, $scheduledEventId: uuid!) {
        sequent_backend_event_execution(
            where: {scheduled_event_id: {_eq: $scheduledEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            id
            tenant_id
            election_event_id
            scheduled_event_id
            labels
            annotations
            execution_state
            execution_payload
            result_payload
            started_at
            ended_at
        }
    }
`
