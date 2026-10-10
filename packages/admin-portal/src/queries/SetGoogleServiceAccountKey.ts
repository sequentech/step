// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const SET_GOOGLE_SERVICE_ACCOUNT_KEY = gql`
    mutation SetGoogleServiceAccountKey($serviceAccountKey: jsonb!) {
        set_google_service_account_key(service_account_key: $serviceAccountKey) {
            client_email
        }
    }
`
