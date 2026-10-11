// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const ELECTORAL_LOG_CONSOLE_PAGE = gql`
    query ElectoralLogConsolePage(
        $tenantId: String
        $electionEventId: String!
        $table: String!
        $filters: jsonb
        $order: String
        $after: String
        $limit: Int
    ) {
        electoral_log_console_page(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            table: $table
            filters: $filters
            order: $order
            after: $after
            limit: $limit
        )
    }
`

export const ELECTORAL_LOG_CONSOLE_RECORD = gql`
    query ElectoralLogConsoleRecord($tenantId: String, $electionEventId: String!, $position: Int!) {
        electoral_log_console_record(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            position: $position
        )
    }
`

export const ELECTORAL_LOG_CONSOLE_QUERY = gql`
    query ElectoralLogConsoleQuery($tenantId: String!, $electionEventId: String!, $sql: String!) {
        electoral_log_console_query(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            sql: $sql
        )
    }
`

export const ELECTORAL_LOG_CONSOLE_TENANTS = gql`
    query ElectoralLogConsoleTenants {
        electoral_log_console_tenants
    }
`
