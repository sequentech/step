// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {DEFAULT_TIME_ZONE, effectiveTimeZone, primaryTimeZone} from "./eventTimeZones"
import {ELogTimeZonePolicy} from "../types/ElectionEventPresentation"

// The same cases as sequent-core `time_zones_tests.rs`, under both configurations.
const event = (configured: Array<string>, primary: string) => ({
    timezones: {configured, primary, logs: ELogTimeZonePolicy.ELECTION},
})
const overseas = event(["Asia/Manila", "Asia/Dubai"], "Asia/Manila")
const association = event(["Europe/Madrid", "Atlantic/Canary"], "Europe/Madrid")

describe("effectiveTimeZone", () => {
    it("uses the election's zone when it is configured", () => {
        expect(effectiveTimeZone(overseas, {timezone: "Asia/Dubai"})).toBe("Asia/Dubai")
        expect(effectiveTimeZone(association, {timezone: "Atlantic/Canary"})).toBe(
            "Atlantic/Canary"
        )
    })

    it("uses the primary without a zone, with an unconfigured one, or for event-wide rows", () => {
        for (const config of [overseas, association]) {
            const primary = config.timezones.primary
            expect(effectiveTimeZone(config, {})).toBe(primary)
            expect(effectiveTimeZone(config, {timezone: "Asia/Tokyo"})).toBe(primary)
            expect(effectiveTimeZone(config, {timezone: " "})).toBe(primary)
            expect(effectiveTimeZone(config)).toBe(primary)
        }
    })

    it("uses UTC for an event without timezones", () => {
        expect(effectiveTimeZone(null)).toBe(DEFAULT_TIME_ZONE)
        expect(effectiveTimeZone({}, {timezone: "Asia/Dubai"})).toBe("UTC")
    })
})

describe("primaryTimeZone", () => {
    it("is the configured primary, else UTC", () => {
        expect(primaryTimeZone(overseas)).toBe("Asia/Manila")
        expect(primaryTimeZone(association)).toBe("Europe/Madrid")
        expect(primaryTimeZone({timezones: {configured: [], primary: ""}})).toBe("UTC")
        expect(primaryTimeZone(undefined)).toBe("UTC")
    })
})
