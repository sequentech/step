// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPLOAD_SIGNATURE = gql`
    mutation UploadSignature(
        $electionId: uuid!
        $tallySessionId: uuid!
        $areaId: uuid!
        $documentId: uuid!
        $password: String!
    ) {
        upload_signature(
            election_id: $electionId
            tally_session_id: $tallySessionId
            area_id: $areaId
            document_id: $documentId
            password: $password
        ) {
            id
        }
    }
`
