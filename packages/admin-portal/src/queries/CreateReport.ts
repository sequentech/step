// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CREATE_REPORT = gql`
    mutation InsertReport($object: sequent_backend_report_insert_input!) {
        insert_sequent_backend_report(objects: [$object]) {
            affected_rows
        }
    }
`
