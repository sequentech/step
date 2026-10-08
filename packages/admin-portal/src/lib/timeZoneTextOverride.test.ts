// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {TFunction} from "i18next"
import {timeZoneTextOverrideError} from "./timeZoneTextOverride"

// Jest maps @sequentech/ui-core to one file; the check is ui-core's timezone service.
jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../ui-core/src/services/timeZones")
)

const t = ((key: string, options?: {placeholders?: string}) =>
    `${key}: ${options?.placeholders ?? ""}`) as unknown as TFunction

const refused = (key: string, value: string) => timeZoneTextOverrideError(t, key, value)

describe("timeZoneTextOverrideError", () => {
    it("refuses a combined string that drops its zone placeholder", () => {
        expect(refused("timezones.dateTimeZone", "{{dateTime}}")).toBe(
            "electionEventScreen.localization.notify.invalidTimeZoneText: {{zone}}"
        )
        expect(refused("global:timezones.voterDateTimeZone", "{{dateTime}} {{zone}}")).toBe(
            "electionEventScreen.localization.notify.invalidTimeZoneText: {{zoneName}}"
        )
    })

    it("refuses a combined string that drops {{dateTime}} in any scope", () => {
        for (const key of [
            "templates:timezones.dateTimeZone",
            "adminPortal:timezones.myTime",
            "votingPortal:timezones.placeTime",
        ]) {
            expect(refused(key, "{{zone}}")).toContain("{{dateTime}}")
        }
        expect(refused("timezones.gap", "Clocks go forward")).toContain("{{dateTime}}, {{city}}")
        expect(refused("timezones.onThisDevice", "Here")).toContain("{{dateTime}}")
    })

    it("accepts reworded strings and every other key", () => {
        expect(
            refused("templates:timezones.dateTimeZone", "{{dateTime}} ({{zone}})")
        ).toBeUndefined()
        expect(
            refused("timezones.voterDateTimeZone", "{{ dateTime }} {{zoneName}}")
        ).toBeUndefined()
        expect(refused("global:timezones.abbr.Asia/Manila", "PHT")).toBeUndefined()
        expect(refused("welcome", "{{zone}}")).toBeUndefined()
    })
})
