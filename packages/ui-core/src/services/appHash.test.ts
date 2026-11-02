// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {shortAppHash} from "./appHash"

it("shows the first eight hex digits of the system hash, in upper case", () => {
    expect(shortAppHash("deadbeef0123456789abcdef0123456789abcdef")).toBe("DEAD-BEEF")
    expect(shortAppHash("94cc549b")).toBe("94CC-549B")
})

it.each([undefined, "", "-", "dev", "test", "story", "deadbee", "deadbeefz", " deadbeef"])(
    "shows no hash for %p",
    (appHash) => {
        expect(shortAppHash(appHash)).toBeUndefined()
    }
)
