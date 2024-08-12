// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPLOAD_SIGNATURE = gql`
    mutation UploadSignature(
        $electionId: uuid!
        $tallySessionId: uuid!
        $areaId: uuid!
        $signature: String!
    ) {
        upload_signature(
            election_id: $electionId
            tally_session_id: $tallySessionId
            area_id: $areaId
            signature: $signature
        ) {
            id
        }
    }
`
