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

/**
 * Who last wrote the event's configuration. Read on its own, so a role that
 * can't select `updated_by` only loses the notice below, never the tab. A
 * configuration copied from the old annotation that didn't validate is marked
 * `MIGRATED_INVALID_BY` (windmill `mark_migrated_configs`).
 */
export const GET_EVENT_MESSAGING_CONFIG_AUTHOR = gql`
    query GetEventMessagingConfigAuthor($tenantId: uuid!, $electionEventId: uuid!) {
        sequent_backend_messaging_event_config(
            where: {tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $electionEventId}}
        ) {
            election_event_id
            updated_by
        }
    }
`

export const MIGRATED_INVALID_BY = "migration: needs saving again"

export interface IGetEventMessagingConfigAuthorResult {
    sequent_backend_messaging_event_config: Array<{
        election_event_id: string
        updated_by?: string | null
    }>
}
