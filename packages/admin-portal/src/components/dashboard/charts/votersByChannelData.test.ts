// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {toVotersByChannelRows} from "./votersByChannelData"

// Admin Portal's Jest setup is CommonJS and cannot load ui-essentials' ESM bundle.
jest.mock("@sequentech/ui-essentials", () => ({
    VotingChannel: {
        ONLINE: "ONLINE",
        KIOSK: "KIOSK",
        EARLY_VOTING: "EARLY_VOTING",
        TELEPHONE: "TELEPHONE",
        PAPER: "PAPER",
        POSTAL: "POSTAL",
    },
}))

describe("toVotersByChannelRows", () => {
    it("maps persisted channel counts and fills absent channels with zero", () => {
        expect(
            toVotersByChannelRows([
                {channel: "ONLINE", count: 7},
                {channel: "TELEPHONE", count: 3},
            ])
        ).toEqual([
            {channel: "ONLINE", count: 7},
            {channel: "KIOSK", count: 0},
            {channel: "EARLY_VOTING", count: 0},
            {channel: "TELEPHONE", count: 3},
        ])
    })
})
