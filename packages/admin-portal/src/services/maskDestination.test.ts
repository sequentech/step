// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {callingCode, maskDestination} from "./maskDestination"

// The same shapes as the backend's Destination::masked.
describe("maskDestination", () => {
    it("keeps a number's country code and last 3 digits", () => {
        expect(maskDestination("+966501234567")).toBe("+966 ••• ••• 567")
        expect(maskDestination("+34600111214")).toBe("+34 ••• ••• 214")
        expect(maskDestination("+15550123456")).toBe("+1 ••• ••• 456")
    })

    it("keeps an email's first 2 characters and its domain", () => {
        expect(maskDestination("voter@example.org")).toBe("vo••••••••@example.org")
    })

    it("keeps a Page-scoped ID's last 3 characters", () => {
        expect(maskDestination("7304419925518842")).toBe("••• 842")
    })

    it("matches calling codes by length", () => {
        expect(callingCode("+447700900123")).toBe("44")
        expect(callingCode("+971501234567")).toBe("971")
        expect(callingCode("+0123")).toBeUndefined()
    })
})
