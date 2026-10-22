// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EExportRange, exportBound, exportRange} from "./exportRange"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../../ui-core/src/services/timeZones")
)

describe("export range", () => {
    it("sends the wall-clock time of the event's zone as an instant with its offset", () => {
        expect(exportBound("2026-05-01T08:00", "Asia/Manila")).toBe("2026-05-01T08:00:00+08:00")
        expect(exportBound("2026-05-01T08:00:30", "UTC")).toBe("2026-05-01T08:00:30Z")
        expect(exportBound("", "Asia/Manila")).toBeUndefined()
    })

    it("takes each bound's own offset across a change of clocks", () => {
        // Madrid: summer time ends on 2026-10-25 at 03:00 (+02:00 → +01:00).
        expect(exportBound("2026-10-24T10:00", "Europe/Madrid")).toBe("2026-10-24T10:00:00+02:00")
        expect(exportBound("2026-10-26T10:00", "Europe/Madrid")).toBe("2026-10-26T10:00:00+01:00")
        // Summer time starts on 2026-03-29 at 02:00 (+01:00 → +02:00).
        expect(exportBound("2026-03-29T01:30", "Europe/Madrid")).toBe("2026-03-29T01:30:00+01:00")
        expect(exportBound("2026-03-29T03:30", "Europe/Madrid")).toBe("2026-03-29T03:30:00+02:00")
    })

    it("refuses invalid bounds and zones instead of exporting an unbounded range", () => {
        expect(exportRange("2026-05-01T08:00", "", "Nowhere/Invalid")).not.toBe(EExportRange.VALID)
        expect(exportRange("not-a-date", "", "Asia/Manila")).not.toBe(EExportRange.VALID)
        expect(exportRange("", "2026-02-30T08:00", "UTC")).not.toBe(EExportRange.VALID)
    })

    it("refuses nonexistent times and resolves repeated times to their first occurrence", () => {
        expect(exportRange("2028-03-12T02:30", "", "America/New_York")).not.toBe(EExportRange.VALID)
        expect(exportBound("2028-11-05T01:30", "America/New_York")).toBe(
            "2028-11-05T01:30:00-04:00"
        )
    })

    it("accepts an open range and refuses an end that is not after the start", () => {
        const zone = "Europe/Madrid"
        expect(exportRange("", "", zone)).toBe(EExportRange.VALID)
        expect(exportRange("2026-05-01T08:00", "", zone)).toBe(EExportRange.VALID)
        expect(exportRange("2026-05-01T08:00", "2026-05-01T09:00", zone)).toBe(EExportRange.VALID)
        expect(exportRange("2026-05-01T09:00", "2026-05-01T09:00", zone)).toBe(
            EExportRange.END_NOT_AFTER_START
        )
        // An hour across the night the clocks go back is still an hour later.
        expect(exportRange("2026-10-25T01:30", "2026-10-25T03:30", zone)).toBe(EExportRange.VALID)
    })
})
