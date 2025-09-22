// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_ALL_TENANTS = gql`
    query GetAllTenants {
        sequent_backend_tenant {
            id
            slug
        }
    }
`
