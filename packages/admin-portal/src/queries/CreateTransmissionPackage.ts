// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_TRANSMISSION_PACKAGE = gql`
    mutation CreateTransmissionPackage(
        $electionId: uuid!
        $tallySessionId: uuid!
        $areaId: uuid!
        $force: Boolean!
    ) {
        create_transmission_package(
            election_id: $electionId
            tally_session_id: $tallySessionId
            area_id: $areaId
            force: $force
        ) {
            id
        }
    }
`
