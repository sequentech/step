// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const ARCHIVE_ELECTION_EVENT = gql`
    mutation ArchiveElectionEvent($election_event_id: String!) {
        archive_election_event(election_event_id: $election_event_id) {
            id
            error_msg
        }
    }
`
