// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CastVoteChannel, toVotersByChannelRows} from "./votersByChannelData"

describe("toVotersByChannelRows", () => {
    it("maps only channels with voters", () => {
        expect(
            toVotersByChannelRows([
                {channel: "ONLINE", count: 7},
                {channel: "TELEPHONE", count: 3},
            ])
        ).toEqual([
            {channel: CastVoteChannel.ONLINE, count: 7},
            {channel: CastVoteChannel.TELEPHONE, count: 3},
        ])
    })

    it("does not expose unsupported channels", () => {
        expect(
            toVotersByChannelRows([
                {channel: "ONLINE", count: 2},
                {channel: "PAPER", count: 3},
                {channel: "FUTURE_CHANNEL", count: 4},
            ])
        ).toEqual([{channel: CastVoteChannel.ONLINE, count: 2}])
    })

    it("returns no legend rows when no channel has voters", () => {
        expect(toVotersByChannelRows([])).toEqual([])
    })
})
