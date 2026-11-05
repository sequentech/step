// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {
    VOTING_STATUS_CHANNELS,
    TALLY_SHEET_VOTING_CHANNELS,
    PARTICIPATION_CHANNEL_ORDER,
    isVotingStatusChannel,
    isTallySheetVotingChannel,
    isKnownParticipationChannel,
    parseParticipationChannel,
} from "./VotingChannel"

it("keeps the seven supported participation channels in their public display order", () => {
    expect(PARTICIPATION_CHANNEL_ORDER).toEqual([
        "ONLINE",
        "KIOSK",
        "EARLY_VOTING",
        "TELEPHONE",
        "PAPER",
        "POSTAL",
        "IN_PERSON",
    ])
    for (const channel of VOTING_STATUS_CHANNELS) {
        expect(isVotingStatusChannel(channel)).toBe(true)
        expect(isTallySheetVotingChannel(channel)).toBe(false)
        expect(isKnownParticipationChannel(channel)).toBe(true)
        expect(parseParticipationChannel(channel)).toBe(channel)
    }
    for (const channel of TALLY_SHEET_VOTING_CHANNELS) {
        expect(isVotingStatusChannel(channel)).toBe(false)
        expect(isTallySheetVotingChannel(channel)).toBe(true)
        expect(isKnownParticipationChannel(channel)).toBe(true)
        expect(parseParticipationChannel(channel)).toBe(channel)
    }
})

it.each(["FUTURE_CHANNEL", "online", "", "__proto__"])(
    "preserves unknown backend channel %s without falsely recognizing it",
    (channel) => {
        expect(isKnownParticipationChannel(channel)).toBe(false)
        expect(parseParticipationChannel(channel)).toBe(channel)
    }
)
