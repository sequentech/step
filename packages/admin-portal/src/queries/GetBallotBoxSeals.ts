// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

/**
 * The seals of the ballot boxes of some elections (VOTE-FREEZE): one row per
 * election and area, inserted when the election closes. No row means the
 * ballot box is open. The manifest and the signed message are not exposed to
 * the portal; the seal record serves them. A published seal has its record's
 * document (`public_document_id`, public or private by the event's Seal
 * Record Publication policy) and, only when public, its `public_path`.
 */
export const GET_BALLOT_BOX_SEALS = gql`
    query GetBallotBoxSeals($electionEventId: uuid!, $electionIds: [uuid!]!) {
        sequent_backend_ballot_box_seal(
            where: {election_event_id: {_eq: $electionEventId}, election_id: {_in: $electionIds}}
            order_by: [{election_id: asc}, {area_id: asc}]
        ) {
            id
            election_id
            area_id
            area {
                id
                name
            }
            status
            closed_at
            grace_deadline
            closed_by
            sealed_at
            ballots_in_box
            ballots_counted
            seal_hash
            failure_reason
            public_document_id
            public_path
            published_at
            last_attempt_at
            waiting_reason
            area_name
        }
    }
`

/**
 * Whether an election event has any ballot box seal (VOTE-FREEZE): such an
 * event, and each of its sealed elections, can't be deleted.
 */
export const GET_EVENT_BALLOT_BOX_SEALS = gql`
    query GetEventBallotBoxSeals($electionEventId: uuid!) {
        sequent_backend_ballot_box_seal(
            where: {election_event_id: {_eq: $electionEventId}}
            limit: 1
        ) {
            id
        }
    }
`

/** The failed seals of an election event: incidents its pages list (D6). */
export const GET_FAILED_BALLOT_BOX_SEALS = gql`
    query GetFailedBallotBoxSeals($electionEventId: uuid!) {
        sequent_backend_ballot_box_seal(
            where: {election_event_id: {_eq: $electionEventId}, status: {_eq: "failed"}}
            order_by: [{election_id: asc}, {area_id: asc}]
        ) {
            id
            election_id
            area_id
            area {
                id
                name
            }
            area_name
            election_name
            failure_reason
        }
    }
`

/**
 * The ballot boxes an election will have (VOTE-FREEZE): the areas of its
 * published ballot styles, the same set the close seals. The card lists them
 * as Open until voting closes.
 */
export const GET_BALLOT_BOX_AREAS = gql`
    query GetBallotBoxAreas($electionEventId: uuid!, $electionId: uuid!) {
        sequent_backend_ballot_style(
            where: {
                election_event_id: {_eq: $electionEventId}
                election_id: {_eq: $electionId}
                deleted_at: {_is_null: true}
                area_id: {_is_null: false}
                ballot_publication: {
                    _or: [{is_generated: {_eq: true}}, {published_at: {_is_null: false}}]
                }
            }
        ) {
            area_id
        }
    }
`

/** The names of some areas. */
export const GET_BALLOT_BOX_AREA_NAMES = gql`
    query GetBallotBoxAreaNames($areaIds: [uuid!]!) {
        sequent_backend_area(where: {id: {_in: $areaIds}}) {
            id
            name
        }
    }
`
