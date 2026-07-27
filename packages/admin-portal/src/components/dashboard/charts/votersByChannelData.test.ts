// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {toVotersByChannelRows, VotingChanel} from "./votersByChannelData"

describe("toVotersByChannelRows", () => {
    it("maps persisted channel counts and fills absent channels with zero", () => {
        expect(
            toVotersByChannelRows([
                {channel: "ONLINE", count: 7},
                {channel: "TELEPHONE", count: 3},
            ])
        ).toEqual([
            {channel: VotingChanel.Online, count: 7},
            {channel: VotingChanel.Kiosk, count: 0},
            {channel: VotingChanel.EarlyVoting, count: 0},
            {channel: VotingChanel.Telephone, count: 3},
        ])
    })
})
