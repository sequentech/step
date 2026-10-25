// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// The tenant's public trustee records, with the public keys needed to decide
// reuse. Only public fields are selected; secret custody material is never read
// here. Used by the COMELEC import to reuse existing approved trustees (preserving
// their ids) and to preflight whether approved public keys are present.
export const GET_TENANT_TRUSTEES = gql`
    query TenantTrustees($tenantId: uuid!) {
        sequent_backend_trustee(where: {tenant_id: {_eq: $tenantId}}) {
            id
            name
            public_key
            share_encryption_public_key
        }
    }
`
