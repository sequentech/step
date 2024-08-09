// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_TRANSMISSION_PACKAGE = gql`
    mutation CreateTransmissionPackage($electionEventId: uuid!, $tallySessionId: uuid!, $areaId: uuid!) {
        create_transmission_package(election_event_id: $electionEventId, tally_session_id: $tallySessionId, area_id: $areaId) {
            id
        }
    }
`
