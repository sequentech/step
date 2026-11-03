// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_TALLY_KEY_RESTORE_STATE = gql`
    query ListTallyKeyRestoreState($tenantId: uuid!, $electionEventId: uuid!) {
        sequent_backend_tally_session(
            where: {tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $electionEventId}}
            order_by: {created_at: desc}
        ) {
            id
            execution_status
        }
        sequent_backend_tally_session_execution(
            where: {tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $electionEventId}}
            distinct_on: tally_session_id
            order_by: [{tally_session_id: asc}, {created_at: desc_nulls_last}, {id: desc}]
        ) {
            tally_session_id
            status
        }
    }
`
