// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EExportRange, exportBound, exportRange} from "./exportRange"

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

    it("reads a zone it does not know as UTC", () => {
        expect(exportBound("2026-05-01T08:00", "Nowhere/Invalid")).toBe("2026-05-01T08:00:00Z")
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
