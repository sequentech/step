// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const READINESS_CHECK = gql`
    mutation ReadinessCheck($official_event_id: String!, $tested_event_id: String!) {
        readiness_check(official_event_id: $official_event_id, tested_event_id: $tested_event_id)
    }
`

// A read-only rehearsal comparison. The production mutation above stays
// unchanged and sends no mode, so the strict signed check is never relabelled.
export const READINESS_REHEARSAL = gql`
    mutation ReadinessRehearsal(
        $official_event_id: String!
        $tested_event_id: String!
        $comparison: ReadinessComparison!
    ) {
        readiness_check(
            official_event_id: $official_event_id
            tested_event_id: $tested_event_id
            comparison: $comparison
        )
    }
`

// Archived rehearsal events remain eligible evidence. Hasura applies permission labels.
export const GET_READINESS_EVENTS = gql`
    query GetReadinessEvents($tenantId: uuid!, $officialEventId: uuid!) {
        sequent_backend_election_event(
            where: {tenant_id: {_eq: $tenantId}, id: {_neq: $officialEventId}}
            order_by: {created_at: desc}
        ) {
            id
            presentation
            is_archived
        }
    }
`
