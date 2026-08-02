// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_CERTIFICATE_AUTHORITIES = gql`
    query GetCertificateAuthorities($electionEventId: uuid!) {
        sequent_backend_certificate_authority(
            where: {election_event_id: {_eq: $electionEventId}}
            order_by: {created_at: asc}
        ) {
            id
            common_name
            issuer_common_name
            subject
            issuer
            not_before
            not_after
            fingerprint_sha256
            serial_number
            created_at
        }
    }
`
