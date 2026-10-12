// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {EMessageChannel} from "@/types/messaging"
import {IVoterCountStatus} from "@/queries/GetMessagingVoterCounts"

// Send Notification's "Each voter's channel": per channel, the voters the
// send reaches and who it sends from, read the way the send reads them. The
// whole event and the voted filters read the event's last count (status:
// when); selected voters are counted in the request.
export const GET_MESSAGING_AUDIENCE_PREVIEW = gql`
    query GetMessagingAudiencePreview(
        $electionEventId: uuid!
        $audienceSelection: String
        $voterIds: [String!]
    ) {
        messaging_audience_preview(
            election_event_id: $electionEventId
            audience_selection: $audienceSelection
            voter_ids: $voterIds
        ) {
            total
            will_send
            status {
                computed_at
                pending
                outcome
            }
            channels {
                channel
                voters
                not_offered
                sends_from
                account_id
                account_name
            }
        }
    }
`

/** Who sends a channel's notices: an account, the deployment's own sender, or nobody. */
export enum ESendsFrom {
    ACCOUNT = "ACCOUNT",
    DEPLOYMENT = "DEPLOYMENT",
    NONE = "NONE",
    /**
     * The account the event names was deleted: the send refuses these
     * notices, never sending them through the deployment's sender.
     */
    ACCOUNT_MISSING = "ACCOUNT_MISSING",
}

export interface IAudienceChannel {
    /** `null`: voters with no contact channel, who get nothing. */
    channel: EMessageChannel | null
    voters: number
    /** Voters on the channel whose Post doesn't offer it for notices: not sent to them. */
    not_offered: number
    /**
     * Absent on the no-channel row, and on a channel none of whose voters
     * may get a notice (their Posts don't offer it): nothing is sent there.
     */
    sends_from?: ESendsFrom | null
    account_id?: string | null
    account_name?: string | null
}

export interface IAudiencePreview {
    total: number
    /** Voters a notice is sent to: on a channel their Post offers. */
    will_send: number
    channels: IAudienceChannel[]
    /**
     * When the counts were counted. Selected voters are counted in the
     * request: `computed_at` is now, or absent with `outcome` TIMED_OUT
     * when counting them took too long (and nothing else is shown).
     */
    status: IVoterCountStatus
}

export interface IGetMessagingAudiencePreviewResult {
    messaging_audience_preview?: IAudiencePreview | null
}
