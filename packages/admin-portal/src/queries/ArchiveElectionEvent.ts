// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const ARCHIVE_ELECTION_EVENT = gql`
    mutation ArchiveElectionEvent($electionEventId: String!) {
        archive_election_event(election_event_id: $electionEventId) {
            id
        }
    }
`
