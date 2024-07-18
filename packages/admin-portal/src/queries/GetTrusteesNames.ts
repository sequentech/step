// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_TRUSTEES_NAMES = gql`
    query TrusteeNames($tenantId: uuid!) {
        sequent_backend_trustee(where: {tenant_id: {_eq: $tenantId}}) {
            id
            name
        }
    }
`
