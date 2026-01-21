// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_DOCUMENT = gql`
    query GetDocument($id: uuid, $tenantId: uuid) {
        sequent_backend_document(where: {_and: {tenant_id: {_eq: $tenantId}, id: {_eq: $id}}}) {
            name
        }
    }
`
