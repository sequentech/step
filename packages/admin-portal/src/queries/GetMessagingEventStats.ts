// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {IChannelMessageStats} from "@/types/messaging"

// Per channel of an event: messages sent in the last 24 hours, and messages
// per state of their latest attempt (so retries don't count twice), with
// Unknown older than 24 hours apart as unresolved. Voters per channel are
// counted apart (GetMessagingVoterCounts).
export const GET_MESSAGING_EVENT_STATS = gql`
    query GetMessagingEventStats($electionEventId: uuid!) {
        messaging_event_stats(election_event_id: $electionEventId) {
            channels {
                channel
                sent_last_24h
                queued
                accepted
                delivered
                failed
                unknown
                unresolved
            }
        }
    }
`

export interface IGetMessagingEventStatsResult {
    messaging_event_stats?: {channels: IChannelMessageStats[]} | null
}
