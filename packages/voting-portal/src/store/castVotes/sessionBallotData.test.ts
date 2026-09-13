// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BALLOT_DATA_KEY,
    BALLOT_DATA_EXPIRATION_KEY,
    clearSessionStorageBallotData,
} from "./sessionBallotData"
import {getLanguageFromURL} from "../../utils/queryParams"

beforeEach(() => sessionStorage.clear())
afterEach(() => {
    sessionStorage.clear()
    window.history.replaceState({}, "", "/")
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

it.each([
    ["?lang=fr-CA", "fr-CA"],
    ["?other=en", undefined],
    ["?lang=", undefined],
])("reads an explicit language from %s", (query, expected) => {
    window.history.replaceState({}, "", `/${query}`)
    expect(getLanguageFromURL()).toBe(expected)
})
