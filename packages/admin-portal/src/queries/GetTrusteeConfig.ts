// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_TRUSTEE_CONFIG = gql`
    query GetTrusteeConfig($tenantId: uuid!, $name: String!) {
        sequent_backend_trustee(where: {tenant_id: {_eq: $tenantId}, name: {_eq: $name}}) {
            id
            name
            annotations
        }
    }
`
