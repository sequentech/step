// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SET_CUSTOM_URL = gql`
    mutation SetCustomUrl($tenantId: String!, origin: String!, redirect_to: String!) {
        set_custom_url(tenant_id: $tenantId, origin: $origin, redirect_to: $redirect_to) {
           message
        }
    }
`
