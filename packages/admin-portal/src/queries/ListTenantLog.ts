// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

/** The react-admin resource of the tenant's log. */
export const TENANT_LOG_RESOURCE = "tenant_log"

/**
 * The tenant's log: the activity that belongs to none of its election
 * events. The server takes the tenant from the session.
 */
export const LIST_TENANT_LOG = gql`
    query listTenantLog(
        $limit: Int
        $offset: Int
        $filter: ElectoralLogFilter
        $order_by: ElectoralLogOrderBy
    ) {
        listTenantLog(limit: $limit, offset: $offset, filter: $filter, order_by: $order_by) {
            items {
                id
                created
                statement_timestamp
                statement_kind
                message
                user_id
                username
            }
            total {
                aggregate {
                    count
                }
            }
        }
    }
`
