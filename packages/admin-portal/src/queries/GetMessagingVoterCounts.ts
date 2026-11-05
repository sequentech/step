// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {EMessageChannel} from "@/types/messaging"

// An event's voters per channel, as last counted in the background (a
// realm takes time to count), with when they were counted.
export const GET_MESSAGING_VOTER_COUNTS = gql`
    query GetMessagingVoterCounts($electionEventId: uuid!) {
        messaging_voter_counts(election_event_id: $electionEventId) {
            total
            channels {
                channel
                voters
                not_offered
            }
            status {
                computed_at
                pending
                outcome
            }
        }
    }
`

// The Recount button: asks for the event's voters to be counted again. A
// recount already on its way isn't asked for twice.
export const RECOUNT_MESSAGING_VOTERS = gql`
    mutation RecountMessagingVoters($electionEventId: uuid!) {
        messaging_recount_voters(election_event_id: $electionEventId) {
            requested
            status {
                computed_at
                pending
                outcome
            }
        }
    }
`

/** How the last recount of an event's voters ended. */
export enum ERecountOutcome {
    COUNTED = "COUNTED",
    /** It took longer than its time limit: the previous counts stay. */
    TIMED_OUT = "TIMED_OUT",
    /** It failed: the previous counts stay. */
    FAILED = "FAILED",
}

export interface IVoterCountStatus {
    /** ISO 8601; `null`: the event's voters were never counted. */
    computed_at?: string | null
    /** A recount was asked for and hasn't finished. */
    pending: boolean
    outcome?: ERecountOutcome | null
}

export interface IVoterChannelCount {
    /** `null`: voters with no contact channel. */
    channel: EMessageChannel | null
    voters: number
    /** Of them, voters whose Post doesn't offer the channel for notices. */
    not_offered: number
}

export interface IVoterCounts {
    total: number
    channels: IVoterChannelCount[]
    status: IVoterCountStatus
}

export interface IGetMessagingVoterCountsResult {
    messaging_voter_counts?: IVoterCounts | null
}

export interface IRecountMessagingVotersResult {
    messaging_recount_voters?: {requested: boolean; status: IVoterCountStatus} | null
}
