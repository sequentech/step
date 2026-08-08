// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EVotingStatus, IElectionEvent, IElectionEventStatus} from "@sequentech/ui-core"

/**
 * Voting has finished when at least one channel actually ran and every channel
 * that ran has reached CLOSED.
 *
 * Channels still at NOT_STARTED are ignored rather than counted as closed. A
 * status row always carries all four channels and the backend only ever writes
 * the ones the event enables, so an online-only event keeps kiosk, early and
 * telephone at NOT_STARTED for its whole life; requiring all four to be CLOSED
 * would hide the results link forever.
 *
 * Testing for "no channel is OPEN" is equally wrong in the other direction: it
 * holds before voting starts and while it is paused, which is what put the
 * results link on events that had not started. See META-12780.
 */
export const isVotingClosedForChannels = (statuses: Array<EVotingStatus | undefined>): boolean => {
    const channelsThatRan = statuses.filter(
        (status): status is EVotingStatus => Boolean(status) && EVotingStatus.NOT_STARTED !== status
    )
    return (
        channelsThatRan.length > 0 &&
        channelsThatRan.every((status) => EVotingStatus.CLOSED === status)
    )
}

export const isElectionEventVotingClosed = (electionEvent?: IElectionEvent): boolean => {
    const status = electionEvent?.status as IElectionEventStatus | null
    return isVotingClosedForChannels([
        status?.voting_status,
        status?.kiosk_voting_status,
        status?.early_voting_status,
        status?.telephone_voting_status,
    ])
}
