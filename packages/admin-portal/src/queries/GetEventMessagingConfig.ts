// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// The event's messaging configuration. Only update_event_messaging_config
// writes it; the old messaging:config annotation is no longer read.
export const GET_EVENT_MESSAGING_CONFIG = gql`
    query GetEventMessagingConfig($tenantId: uuid!, $electionEventId: uuid!) {
        sequent_backend_messaging_event_config(
            where: {tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $electionEventId}}
        ) {
            election_event_id
            config
            version
            updated_at
        }
    }
`

export interface IGetEventMessagingConfigResult {
    sequent_backend_messaging_event_config: Array<{
        election_event_id: string
        config: unknown
        version?: number | null
        updated_at?: string | null
    }>
}
