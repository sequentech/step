// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const TENANT_RESET_PREVIEW = gql`
    query TenantResetPreview($tenant_id: String!) {
        tenant_reset_preview(tenant_id: $tenant_id)
    }
`

export const RESET_TENANT = gql`
    mutation ResetTenant(
        $tenant_id: String!
        $confirmation: String!
        $erase_ballots_confirmation: String
    ) {
        reset_tenant(
            tenant_id: $tenant_id
            confirmation: $confirmation
            erase_ballots_confirmation: $erase_ballots_confirmation
        )
    }
`
