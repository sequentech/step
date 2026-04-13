// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_CERTIFICATE_AUTHORITY = gql`
    mutation ImportCertificateAuthority($pemContent: String!) {
        import_certificate_authority(pem_content: $pemContent) {
            inserted_count
            skipped_count
            errors
        }
    }
`
