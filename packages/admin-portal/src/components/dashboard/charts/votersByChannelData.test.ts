// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {OTHER_VOTING_CHANNEL, toVotersByChannelRows} from "./votersByChannelData"

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

    it("groups unexpected persisted channels without losing voters", () => {
        expect(
            toVotersByChannelRows([
                {channel: "ONLINE", count: 2},
                {channel: "PAPER", count: 3},
                {channel: "FUTURE_CHANNEL", count: 4},
            ])
        ).toEqual([
            {channel: "ONLINE", count: 2},
            {channel: "KIOSK", count: 0},
            {channel: "EARLY_VOTING", count: 0},
            {channel: "TELEPHONE", count: 0},
            {channel: OTHER_VOTING_CHANNEL, count: 7},
        ])
    })
})
