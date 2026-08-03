// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TallySheetVotingChannel, VotingStatusChannel} from "@sequentech/ui-core"
import {toVotersByChannelRows} from "./votersByChannelData"

describe("toVotersByChannelRows", () => {
    it("maps only channels with voters", () => {
        expect(
            toVotersByChannelRows([
                {channel: VotingStatusChannel.Online, count: 7},
                {channel: VotingStatusChannel.Telephone, count: 3},
            ])
        ).toEqual([
            {channel: VotingStatusChannel.Online, count: 7},
            {channel: VotingStatusChannel.Telephone, count: 3},
        ])
    })

    it("does not expose unsupported channels", () => {
        expect(
            toVotersByChannelRows([
                {channel: VotingStatusChannel.Online, count: 2},
                {channel: TallySheetVotingChannel.Paper, count: 3},
                {channel: "FUTURE_CHANNEL", count: 4},
            ])
        ).toEqual([{channel: VotingStatusChannel.Online, count: 2}])
    })

    it("returns no legend rows when no channel has voters", () => {
        expect(toVotersByChannelRows([])).toEqual([])
    })
})
