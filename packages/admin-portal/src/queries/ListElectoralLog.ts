// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

const validOrderBy = [
    "id",
    "created",
    "statement_timestamp",
    "statement_kind",
    "user_id",
    "username",
]

export const getElectoralLogVariables = (input: any) => {
    return {
        ...input,
        filter: input?.where?._and?.reduce((acc: any, condition: any) => {
            Object.keys(condition).forEach((key) => {
                if (key !== "election_event_id") {
                    acc[key] = condition[key]?._eq
                }
            })
            return acc
        }, {}),
        order_by:
            Object.fromEntries(
                Object.entries(input?.order_by || {}).filter(([key]) => validOrderBy.includes(key))
            ) ?? undefined,
    }
}

export const getElectoralLog = (fields: any) => {
    let election_event_id = fields?.filter?.election_event_id ?? ""
    return gql`
        query listElectoralLog(
            $limit: Int
            $offset: Int
            $filter: ElectoralLogFilter
            $election_event_id: String = "${election_event_id}"
            $order_by: ElectoralLogOrderBy
        ) {
            listElectoralLog(
                limit: $limit
                offset: $offset
                filter: $filter
                election_event_id: $election_event_id
                order_by: $order_by
            ) {
                items {
                    id
                    created
                    statement_timestamp
                    statement_kind
                    message
                    user_id
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
