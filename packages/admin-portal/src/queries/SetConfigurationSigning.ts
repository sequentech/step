// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

/** Check and save the tenant's configuration package signing trust (hasura
 * action set_configuration_signing, harvest POST /set-configuration-signing).
 * A setting with errors comes back with them and is not saved. */
export const SET_CONFIGURATION_SIGNING = gql`
    mutation SetConfigurationSigning($tenantId: String!, $configurationSigning: jsonb!) {
        set_configuration_signing(
            tenant_id: $tenantId
            configuration_signing: $configurationSigning
        ) {
            saved
            check
        }
    }
`

/** The tenant's settings as stored, read back after a save: the canonical
 * configuration_signing (certificate-only roots) the server wrote. */
export const GET_TENANT_SETTINGS = gql`
    query GetTenantSettings($tenantId: uuid!) {
        sequent_backend_tenant_by_pk(id: $tenantId) {
            id
            settings
        }
    }
`
