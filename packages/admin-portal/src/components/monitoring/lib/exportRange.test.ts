// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EExportRange, exportBound, exportRange} from "./exportRange"

describe("export range", () => {
    it("sends a wall-clock time with seconds, or nothing", () => {
        expect(exportBound("2026-05-01T08:00")).toBe("2026-05-01T08:00:00")
        expect(exportBound("2026-05-01T08:00:30")).toBe("2026-05-01T08:00:30")
        expect(exportBound("")).toBeUndefined()
    })

    it("accepts an open range and refuses an end that is not after the start", () => {
        expect(exportRange("", "")).toBe(EExportRange.VALID)
        expect(exportRange("2026-05-01T08:00", "")).toBe(EExportRange.VALID)
        expect(exportRange("2026-05-01T08:00", "2026-05-01T09:00")).toBe(EExportRange.VALID)
        expect(exportRange("2026-05-01T09:00", "2026-05-01T09:00")).toBe(
            EExportRange.END_NOT_AFTER_START
        )
    })
})
