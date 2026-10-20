// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const BALLOT_BOX_LOG = gql`
    query BallotBoxLog(
        $electionEventId: uuid!
        $electionId: uuid
        $prefix: String
        $cursor: String
        $limit: Int
    ) {
        ballot_box_log(
            election_event_id: $electionEventId
            election_id: $electionId
            prefix: $prefix
            cursor: $cursor
            limit: $limit
        ) {
            payload
        }
    }
`
export const BALLOT_BOX_OWN = gql`
    query BallotBoxOwn($electionEventId: uuid!, $electionId: uuid) {
        ballot_box_own(election_event_id: $electionEventId, election_id: $electionId) {
            payload
        }
    }
`
export const BALLOT_BOX_ENTRY = gql`
    query BallotBoxEntry($electionEventId: uuid!, $electionId: uuid!, $ballotId: String!) {
        ballot_box_entry(
            election_event_id: $electionEventId
            election_id: $electionId
            ballot_id: $ballotId
        ) {
            payload
        }
    }
`
export const BALLOT_BOX_KEYS = gql`
    query BallotBoxKeys($electionEventId: uuid!, $electionId: uuid!, $ballotId: String) {
        ballot_box_keys(
            election_event_id: $electionEventId
            election_id: $electionId
            ballot_id: $ballotId
        ) {
            payload
        }
    }
`
