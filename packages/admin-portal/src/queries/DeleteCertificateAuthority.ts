// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const DELETE_CERTIFICATE_AUTHORITY = gql`
    mutation DeleteCertificateAuthority($id: uuid!, $electionEventId: uuid!) {
        delete_certificate_authority(id: $id, election_event_id: $electionEventId) {
            deleted
        }
    }
`
