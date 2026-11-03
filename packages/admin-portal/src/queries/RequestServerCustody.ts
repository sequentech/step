// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const REQUEST_SERVER_CUSTODY = gql`
    mutation RequestServerCustody($object: jsonb!) {
        request_server_custody(object: $object)
    }
`
