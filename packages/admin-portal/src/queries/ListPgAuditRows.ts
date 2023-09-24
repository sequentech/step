// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_LIST_PGAUDIT_ROW = gql`
    {
        listPgaudit(limit: $limit, offset: $offset) {
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

/*
Typical queries in ra-data-hasura are like follows (source: https://github.com/hasura/ra-data-hasura):

params:
{
  "pagination": { "page": 1, "perPage": 5 },
  "sort": { "field": "name", "order": "DESC" },
  "filter": {
    "ids": [101, 102]
  }
}

query:
query person($limit: Int, $offset: Int, $order_by: [person_order_by!]!, $where: person_bool_exp) {
  items: person(limit: $limit, offset: $offset, order_by: $order_by, where: $where) {
    id
    name
    address_id
  }
  total: person_aggregate(limit: $limit, offset: $offset, order_by: $order_by, where: $where) {
    aggregate {
      count
    }
  }
}
*/