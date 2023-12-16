// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const getPgAudit = (fields: any) => gql`
    query listPgaudit($limit: Int, $offset: Int, $order_by: PgAuditOrderBy) {
        listPgaudit(limit: $limit, offset: $offset, order_by: $order_by) {
            items {
                id
                audit_type
                class
                command
                dbname
                id
                server_timestamp
                session_id
                statement
                user
            }
            total {
                aggregate {
                    count
                }
            }
        }
    }
`
