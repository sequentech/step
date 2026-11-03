// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const RECORD_TRANSMISSION_DOWNLOAD = gql`
    mutation RecordTransmissionDownload(
        $electionEventId: uuid!
        $electionId: uuid!
        $areaId: uuid!
        $tallySessionId: uuid!
        $documentId: uuid!
    ) {
        record_transmission_download(
            election_event_id: $electionEventId
            election_id: $electionId
            area_id: $areaId
            tally_session_id: $tallySessionId
            document_id: $documentId
        ) {
            file_name
            sha256
        }
    }
`
