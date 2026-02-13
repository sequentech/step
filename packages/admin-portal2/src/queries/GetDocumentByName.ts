// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_DOCUMENT_BY_NAME = gql`
    query GetDocumentByName($name: String!, $tenantId: uuid!) {
        sequent_backend_document(where: {_and: {name: {_eq: $name}, tenant_id: {_eq: $tenantId}}}) {
            id
        }
    }
`
