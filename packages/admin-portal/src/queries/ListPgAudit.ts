// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
import {PgAuditTable} from "@/gql/graphql"

export const getPgauditVariables = (input: any) => {
    return {
        ...input,
        filter: input?.where?._and?.reduce((acc: any, condition: any) => {
            Object.keys(condition).forEach((key) => {
                acc[key] = condition[key]?._eq
            })
            return acc
        }, {}),
    }
}

export const getPgAudit = (fields: any, audit_table: string) => {
    const auditTableText = audit_table.length ? ` = "${audit_table}"` : ""
    return gql`
        query listPgaudit(
            $limit: Int
            $offset: Int
            $filter: PgAuditFilter
            $order_by: PgAuditOrderBy
            $audit_table: PgAuditTable ${auditTableText}
        ) {
            listPgaudit(
                limit: $limit
                offset: $offset
                filter: $filter
                order_by: $order_by
                audit_table: $audit_table
            ) {
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
