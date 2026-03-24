// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const DELETE_CERTIFICATE_AUTHORITY = gql`
    mutation DeleteCertificateAuthority($id: uuid!) {
        delete_sequent_backend_certificate_authority_by_pk(id: $id) {
            id
        }
    }
`
