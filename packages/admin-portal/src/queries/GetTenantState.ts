// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// The selected tenant's current row, so the COMELEC import can generate a supported
// tenant_configuration that applies the approved public settings while preserving the
// target's id/slug and all unrelated state. Read-only; no secret custody material.
export const GET_TENANT_STATE = gql`
    query TenantState($tenantId: uuid!) {
        sequent_backend_tenant(where: {id: {_eq: $tenantId}}) {
            id
            slug
            created_at
            updated_at
            labels
            annotations
            is_active
            voting_channels
            settings
            test
        }
    }
`
