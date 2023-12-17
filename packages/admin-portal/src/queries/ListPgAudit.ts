// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const getPgauditVariables = (input: any) => {
    return {
        filter: input?.where?._and?.reduce((acc: any, condition: any) => {
            Object.keys(condition).forEach((key) => {
                acc[key] = condition[key]?._eq
            })
            return acc
        }, {}),
        ...input,
    }
}

export const getPgAudit = (fields: any) => {
    return gql`
        query listPgaudit(
            $limit: Int
            $offset: Int
            $filter: PgAuditFilter
            $order_by: PgAuditOrderBy
        ) {
            listPgaudit(limit: $limit, offset: $offset, filter: $filter, order_by: $order_by) {
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
}
