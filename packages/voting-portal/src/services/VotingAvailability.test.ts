// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {EEarlyVotingPolicy, EVotingStatus} from "@sequentech/ui-core"
import {isElectionOpenForVoting} from "./VotingAvailability"

const {OPEN, CLOSED, NOT_STARTED, PAUSED} = EVotingStatus
const earlyArea = {allow_early_voting: EEarlyVotingPolicy.ALLOW_EARLY_VOTING}

test.each([OPEN, CLOSED, PAUSED, NOT_STARTED])(
    "online voting respects election status %s",
    (status) => {
        expect(
            isElectionOpenForVoting({
                electionStatus: {voting_status: status},
                eventStatus: {voting_status: OPEN},
                isKiosk: false,
            })
        ).toBe(status === OPEN)
    }
)

test.each([OPEN, CLOSED, PAUSED, NOT_STARTED])(
    "kiosk voting respects its own status %s",
    (status) => {
        expect(
            isElectionOpenForVoting({
                electionStatus: {voting_status: OPEN, kiosk_voting_status: status},
                eventStatus: {voting_status: OPEN, kiosk_voting_status: OPEN},
                isKiosk: true,
            })
        ).toBe(status === OPEN)
    }
)

test("kiosk voting works while online voting is closed", () => {
    expect(
        isElectionOpenForVoting({
            electionStatus: {voting_status: CLOSED, kiosk_voting_status: OPEN},
            eventStatus: {voting_status: CLOSED, kiosk_voting_status: OPEN},
            isKiosk: true,
        })
    ).toBe(true)
})

test.each([false, true])("early voting requires an eligible area: %s", (eligible) => {
    expect(
        isElectionOpenForVoting({
            electionStatus: {voting_status: NOT_STARTED, early_voting_status: OPEN},
            eventStatus: {voting_status: NOT_STARTED, early_voting_status: OPEN},
            areaPresentation: eligible ? earlyArea : undefined,
            isKiosk: false,
        })
    ).toBe(eligible)
})

test.each([CLOSED, PAUSED])("early voting cannot reopen online status %s", (status) => {
    expect(
        isElectionOpenForVoting({
            electionStatus: {voting_status: status, early_voting_status: OPEN},
            eventStatus: {early_voting_status: OPEN},
            areaPresentation: earlyArea,
            isKiosk: false,
        })
    ).toBe(false)
})

test.each([false, true])("closed event blocks a new ballot, kiosk=%s", (isKiosk) => {
    expect(
        isElectionOpenForVoting({
            electionStatus: {
                voting_status: OPEN,
                kiosk_voting_status: OPEN,
                early_voting_status: OPEN,
            },
            eventStatus: {
                voting_status: CLOSED,
                kiosk_voting_status: CLOSED,
                early_voting_status: CLOSED,
            },
            areaPresentation: earlyArea,
            isKiosk,
        })
    ).toBe(false)
})

test.each([false, true])(
    "explicitly disabled channels cannot start ballots, kiosk=%s",
    (isKiosk) => {
        expect(
            isElectionOpenForVoting({
                electionStatus: {voting_status: OPEN, kiosk_voting_status: OPEN},
                eventStatus: {voting_status: OPEN, kiosk_voting_status: OPEN},
                channels: {online: false, kiosk: false},
                isKiosk,
            })
        ).toBe(false)
    }
)

test.each([false, true])(
    "telephone voting does not open a browser channel, kiosk=%s",
    (isKiosk) => {
        expect(
            isElectionOpenForVoting({
                electionStatus: {telephone_voting_status: OPEN},
                eventStatus: {telephone_voting_status: OPEN},
                isKiosk,
            })
        ).toBe(false)
    }
)

test("missing status fails closed", () => {
    expect(isElectionOpenForVoting({isKiosk: false})).toBe(false)
})
