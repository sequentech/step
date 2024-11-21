// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const INSERT_TEMPLATE = gql`
    mutation InsertTemplate($object: sequent_backend_template_insert_input!) {
        insert_sequent_backend_template(objects: [$object]) {
            affected_rows
        }
    }
`
