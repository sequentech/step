// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const LIST_TRASHED_ELECTION_EVENTS = gql`
    query ListTrashedElectionEvents($limit: Int, $offset: Int) {
        list_trashed_election_events(limit: $limit, offset: $offset) {
            items {
                id
                presentation
                trashed_at
                purge_after
                purge_started_at
                archived_before_trash
            }
            total
            retention_days
        }
    }
`
