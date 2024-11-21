// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SEND_TRANSMISSION_PACKAGE = gql`
    mutation SendTransmissionPackage($electionId: uuid!, $tallySessionId: uuid!, $areaId: uuid!) {
        send_transmission_package(
            election_id: $electionId
            tally_session_id: $tallySessionId
            area_id: $areaId
        ) {
            id
        }
    }
`
