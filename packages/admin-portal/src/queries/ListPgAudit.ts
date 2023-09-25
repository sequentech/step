// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

//query listPgaudit($limit: Int, $offset: Int) {
//  listPgaudit(limit: $limit, offset: $offset) {

export const getList = (fields: any) => gql`
query listPgaudit {
  listPgaudit {
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
