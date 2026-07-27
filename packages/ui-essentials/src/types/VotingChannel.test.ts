// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {VotingChannel} from "./VotingChannel"

describe("VotingChannel", () => {
    it("is available at runtime with the persisted channel values", () => {
        expect(Object.values(VotingChannel)).toEqual([
            "ONLINE",
            "KIOSK",
            "EARLY_VOTING",
            "TELEPHONE",
            "PAPER",
            "POSTAL",
        ])
    })
})
