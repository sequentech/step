// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EEarlyVotingPolicy,
    EVotingStatus,
    IAreaPresentation,
    IElectionEventStatus,
    IElectionStatus,
    IVotingChannelsConfig,
} from "@sequentech/ui-core"

/** Whether a new ballot can be started. The server checks grace at submission. */
export function isElectionOpenForVoting({
    electionStatus,
    eventStatus,
    channels,
    areaPresentation,
    isKiosk,
}: {
    electionStatus?: Partial<IElectionStatus> | null
    eventStatus?: Partial<IElectionEventStatus> | null
    channels?: Partial<IVotingChannelsConfig> | null
    areaPresentation?: IAreaPresentation | null
    isKiosk: boolean
}): boolean {
    if (isKiosk) {
        return (
            channels?.kiosk !== false &&
            electionStatus?.kiosk_voting_status === EVotingStatus.OPEN &&
            eventStatus?.kiosk_voting_status === EVotingStatus.OPEN
        )
    }
    if (channels?.online === false) return false
    return (
        (electionStatus?.voting_status === EVotingStatus.OPEN &&
            eventStatus?.voting_status === EVotingStatus.OPEN) ||
        (electionStatus?.voting_status === EVotingStatus.NOT_STARTED &&
            areaPresentation?.allow_early_voting === EEarlyVotingPolicy.ALLOW_EARLY_VOTING &&
            electionStatus?.early_voting_status === EVotingStatus.OPEN &&
            eventStatus?.early_voting_status === EVotingStatus.OPEN)
    )
}
