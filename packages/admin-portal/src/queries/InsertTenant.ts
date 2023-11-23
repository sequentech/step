// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const INSERT_TENANT = gql`
    mutation InsertTenant($slug: String!) {
        insertTenant(slug: $slug) {
            id
            slug
        }
    }
`
