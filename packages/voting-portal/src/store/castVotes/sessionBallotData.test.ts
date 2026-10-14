// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BALLOT_DATA_KEY,
    BALLOT_DATA_EXPIRATION_KEY,
    clearSessionStorageBallotData,
} from "./sessionBallotData"

beforeEach(() => sessionStorage.clear())
afterEach(() => {
    sessionStorage.clear()
})

it("clears the ballot and expiry together while preserving unrelated session preferences", () => {
    sessionStorage.setItem(BALLOT_DATA_KEY, "synthetic-private-ballot")
    sessionStorage.setItem(BALLOT_DATA_EXPIRATION_KEY, "1700000000")
    sessionStorage.setItem("language", "en")
    clearSessionStorageBallotData()
    clearSessionStorageBallotData() // Repeated cleanup must be harmless after logout/retry.
    expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBeNull()
    expect(sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY)).toBeNull()
    expect(sessionStorage.getItem("language")).toBe("en")
})
